---

description: "Liste des tâches d'implémentation de la feature 002 : paquets Typst intégrés"
---

# Tasks: Paquets Typst intégrés au service

**Input**: Design documents from `specs/002-typst-packages/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: INCLUS. La constitution impose des tests de contrat API, de génération de bout en
bout et de bac à sable, et FR-010 exige des vérifications automatiques de chaque paquet. Dans
chaque story, les tests sont écrits en premier et DOIVENT échouer avant l'implémentation.

**Organization**: une phase par user story, dans l'ordre de priorité de la spec : US1 (P1),
US2 (P2), US4 (P2), US3 (P3).

**Rappels transverses** :
- Dans un template, un paquet s'importe **par son seul nom** (`#import "@preview/zero"`). Écrire
  une version est une erreur (R15).
- La résolution des imports a lieu **une fois, au chargement** du template, jamais à chaque
  génération (FR-006).
- Aucune nouvelle dépendance d'**exécution** : seulement des build-dependencies et des
  dev-dependencies.
- `src/` reste agnostique du contenu (FR-024 V1) : les exemples vivent dans `examples/` et
  `tests/fixtures/`.

**Liste validée** (spec, Assumptions ; `role` entre parenthèses) :

| Rôle | Paquets |
|---|---|
| `selected` (20) | tiaoma 0.3.0, zebra 0.1.0, qrypst 0.1.1, ~~codetastic 0.2.2~~, sepay 0.1.1, zero 0.7.1, oxifmt 1.0.0, frogst 1.0.0, ibanator 0.1.0, datify 1.3.0, linguify 0.5.0, tabut 1.0.2, tablem 0.3.0, cetz 0.5.2, cetz-plot 0.1.4, lilaq 0.6.0, primaviz 0.11.0, showybox 2.0.4, framefit 0.1.0, modern-mailmerge 0.1.0, payqr-swiss 0.5.0 |
| `dependency` (8) | datify-core 2.1.0, elembic 1.1.1, komet 0.1.0, komet 0.2.0, rustycure 0.2.0, suiji 0.5.1, tiptoe 0.4.0, zero 0.6.1 |

> **2026-10-10, implémentation** : `codetastic` 0.2.2 (2023) échoue avec Typst 0.15.1
> (`cannot add string and type`, comparaison type/chaîne supprimée en 0.14). Il est retiré
> selon la politique R11 : **20 paquets mis à disposition, 28 au total**. tiaoma couvre les
> mêmes codes-barres.

## Format: `[ID] [P?] [Story] Description`

- **[P]** : parallélisable (fichiers différents, aucune dépendance sur une tâche non terminée).
- **[Story]** : user story concernée (US1 à US4).
- Les chemins sont relatifs à la racine du dépôt.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose** : archives, fichier de verrouillage et outillage de construction.

- [X] T001 Écrire `scripts/add-package.sh <name> <version> [--dependency]` (bash, `set -euo pipefail`, contract lock-file.md, R13). Le script :
  - télécharge `https://packages.typst.org/preview/<name>-<version>.tar.gz` dans `packages/vendor/` ;
  - calcule le sha256 (`shasum -a 256` ou `sha256sum`) ;
  - lit `license` dans le `typst.toml` de l'archive (`tar -xzOf … typst.toml`) ;
  - écrit ou remplace l'entrée dans `packages/lock.toml`, triée par `name` puis `version`, avec `role = "selected"` par défaut ou `"dependency"`. En mode `selected`, une entrée `selected` existante du même nom est remplacée et son archive supprimée si aucune autre entrée ne la référence ;
  - rend le fichier exécutable.
- [X] T002 Lancer `scripts/add-package.sh` pour les 21 paquets `selected` et les 8 `dependency` du tableau ci-dessus. Vérifier :
  - 29 entrées dans `packages/lock.toml`, avec l'en-tête de commentaire du contract lock-file.md ;
  - 29 archives dans `packages/vendor/` (environ 1,7 Mo) ;
  - les licences (`rustycure` en EUPL-1.2, `cetz`/`cetz-plot` en LGPL-3.0-or-later, `payqr-swiss` en LGPL).
- [X] T003 [P] Ajouter dans `Cargo.toml` les `[build-dependencies]` `flate2`, `tar`, `sha2`, `toml` et `serde` (feature `derive`), avec des versions compatibles avec celles déjà présentes dans `Cargo.lock`. Ajouter `sha2` et `toml` aux `[dev-dependencies]`. Vérifier qu'aucune dépendance d'exécution n'est ajoutée (`cargo tree -e normal --depth 1`).
- [X] T004 [P] Mettre à jour `Dockerfile` : dans les deux étapes de build (couche de dépendances et build final), copier `build.rs` et `packages/` avant `cargo build`. Vérifier que `.dockerignore` n'exclut pas `packages/`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose** : les paquets existent dans le binaire et sont accessibles par une API Rust. Bloque toutes les stories.

- [X] T005 Écrire `build.rs` (R1, R3, R4, contract lock-file.md « Vérifications à la construction »). Il doit :
  - déclarer `cargo:rerun-if-changed=packages` et `cargo:rerun-if-changed=build.rs` ;
  - lire `packages/lock.toml` (structs serde) ;
  - pour chaque entrée, vérifier le sha256 de l'archive, la décompresser (flate2 + tar) dans `OUT_DIR/packages/<name>-<version>/`, puis vérifier que `typst.toml` existe, que `name` et `version` correspondent, que `entrypoint` existe et qu'aucun chemin n'est absolu ni ne contient `..` ;
  - refuser une archive de `vendor/` absente du lock, ainsi que deux entrées `selected` pour un même nom ;
  - générer `OUT_DIR/bundled_packages.rs`, une table statique triée par (`name`, `version`) de `BundledPackageDef { name, version, description, license, role, entrypoint, files: &[(&str, &[u8])] }`, chaque fichier étant inclus avec `include_bytes!` sur son chemin absolu dans `OUT_DIR`.

  Les messages d'échec sont ceux du contrat (`panic!` avec un message clair).
- [X] T006 Créer `src/packages/mod.rs` (data-model.md « BundledPackage ») et le déclarer dans `src/lib.rs`. Le module :
  - fait un `include!(concat!(env!("OUT_DIR"), "/bundled_packages.rs"))` ;
  - définit `BundledPackage` (avec `spec()` qui renvoie un `PackageSpec` `@preview/name:version`) ;
  - expose `all()`, `get(&PackageSpec) -> Option<&'static BundledPackage>` (namespace `preview` uniquement), `selected(name) -> Option<&'static BundledPackage>` et `file(&self, path) -> Option<&'static [u8]>`.

  Ajouter des tests unitaires dans le même fichier : 29 paquets, 21 `selected`, `selected("zero")` vaut 0.7.1, `get` de `@preview/zero:0.6.1` existe, `get` de `@local/zero:0.7.1` vaut `None`.
- [X] T007 Lancer `cargo build`, puis contrôler à la main SC-007 selon la procédure de quickstart.md §2 (archive altérée → la construction échoue en nommant le paquet ; archive restaurée → la construction passe). Noter le résultat dans le message de commit.

**Checkpoint** : `cargo test --lib packages` passe et le binaire contient les 29 paquets.

---

## Phase 3: User Story 1 - Utiliser un paquet du service dans un template (Priority: P1) 🎯 MVP

**Goal** : `#import "@preview/<nom>"` fonctionne dans un template, sans version et sans réseau.

**Independent Test** : générer `examples/templates/packages-demo` et vérifier dans le PDF le QR code, le montant formaté et le graphique ; deux générations identiques donnent les mêmes octets.

### Tests for User Story 1 ⚠️

- [X] T008 [P] [US1] Dans `tests/bundled_packages.rs`, écrire un test **d'import** (FR-010, R11.1). Pour chacun des 29 paquets de `inkpdf::packages::all()`, compiler en PDF, via `inkpdf::render::compile_pdf` sur un `TemplateEntry` en mémoire construit par un helper de `tests/common/mod.rs`, un `main.typ` minimal `#import "@preview/<n>:<v>"` suivi de `#[ok]` (les tests passent par le `World`, la version y est explicite). Afficher la liste des paquets en échec avant d'échouer.
- [X] T009 [P] [US1] Créer `tests/fixtures/package-smoke/<nom>.typ` pour chacun des 21 paquets `selected`. Chaque fichier importe le paquet **par son seul nom** et appelle une fonction représentative tirée de son README dans l'archive : un QR code pour tiaoma, zebra et qrypst, un code-barres pour codetastic, un QR EPC pour sepay, `num` pour zero, `strfmt` pour oxifmt, un nombre en toutes lettres pour frogst, le formatage d'un IBAN pour ibanator, une date en français pour datify, une traduction pour linguify, un tableau pour tabut et tablem, un dessin pour cetz, un graphique pour cetz-plot, lilaq et primaviz, un encadré pour showybox, un texte ajusté pour framefit, une page de publipostage pour modern-mailmerge, une QR-facture pour payqr-swiss. Utiliser des données littérales et aucune date du jour.
- [X] T010 [US1] Dans `tests/bundled_packages.rs`, écrire un test **d'usage** (R11.2). Chaque fixture de T009 est chargée comme template, en passant par le chargeur, donc avec la réécriture des imports, puis rendue en PDF non vide. Ajouter un test de **déterminisme** : deux rendus de la fixture `cetz` donnent les mêmes octets (FR-015).
- [X] T011 [P] [US1] Dans `tests/bundled_packages.rs`, écrire un test de **fermeture** (R11.3). Tous les imports littéraux `@preview/…` des `.typ` de chaque paquet, analysés avec `typst::syntax` et en excluant les sous-dossiers `tests/`, `docs/`, `examples/`, `gallery/` et `template/` du paquet, doivent désigner un paquet de `all()`.
- [X] T012 [P] [US1] Dans `tests/sandbox.rs` (FR-013) :
  - remplacer le test V1 qui exigeait le refus de `@preview/…` par un test vérifiant que `#import "@preview/zero"` est accepté ;
  - ajouter un test vérifiant qu'un paquet ne peut pas lire un fichier du template. Utiliser une fixture `tests/fixtures/templates/package-reads-template/` dont `main.typ` passe la chaîne `"secret.txt"` (et non un `path`) à une fonction qui fait `read`, ou à défaut tester directement `SandboxWorld::file` avec un `FileId` `Package(spec)` sur `/../secret.txt` ;
  - ajouter un test vérifiant qu'un `path("assets/x.svg")` transmis explicitement à un paquet fonctionne.
- [X] T013 [P] [US1] Dans `tests/api_render.rs`, faire un test de bout en bout : `POST /templates/packages-demo/render` avec `examples/requests/packages-demo.json` renvoie 200 `application/pdf`, et `pdf_text` contient le montant formaté et le titre attendus.

### Implementation for User Story 1

- [X] T014 [US1] Dans `src/render/world.rs` (R5) : `lookup` traite `VirtualRoot::Package(spec)` via `crate::packages::get(spec)` (correspondance exacte) et renvoie `Bytes` du fichier, `FileError::NotFound` si le chemin est absent, ou `FileError::Package(PackageError::NotFound(spec))` si le paquet est inconnu. Retirer le refus V1. Garder `check_cancelled()`.
- [X] T015 [US1] Dans `src/packages/mod.rs` et `src/render/world.rs` (R6) :
  - mettre en place un cache **global** de `Source` pour les fichiers de paquets (`OnceLock<Mutex<HashMap<FileId, Source>>>`) ;
  - créer les `Bytes` à partir des slices statiques sans copie, une fois par fichier (cache `OnceLock`, ou `Bytes::new` sur `&'static [u8]`) ;
  - dans `SandboxWorld::source`, utiliser ce cache pour `VirtualRoot::Package` et garder le cache par rendu pour `Project`.
- [X] T016 [US1] Créer `src/template/imports.rs` (R8, R15, data-model.md « TemplateImport ») et le déclarer dans `src/template/mod.rs`. Il expose `resolve_imports(path: &str, text: &str) -> Result<Option<String>, Vec<ImportError>>`, qui :
  1. analyse le texte avec `typst::syntax::parse` ;
  2. collecte les `ModuleImport` et `ModuleInclude` dont la source est un `Expr::Str` commençant par `@` ;
  3. classe chaque import en `Resolved`, `VersionWritten`, `OtherNamespace`, `Unavailable` ou `Malformed` ;
  4. si tout est `Resolved`, renvoie le texte réécrit (`@preview/x` → `@preview/x:<version selected>`, remplacement par plage d'octets du nœud chaîne, de la fin vers le début), ou `None` s'il n'y a aucun import de paquet.

  Chaque erreur porte le fichier, la ligne (à partir de 1) et le message exact de contracts/template-imports.md. Ajouter des tests unitaires : réécriture correcte, plusieurs imports sur une ligne, `#include`, commentaires ignorés, et chacun des quatre cas d'erreur.
- [X] T017 [US1] Dans `src/registry/loader.rs`, après la lecture de l'instantané et avant la construction de l'entrée valide, appeler `resolve_imports` sur chaque fichier `.typ` (UTF-8). En cas de succès, remplacer les octets du fichier dans `entry.files` par le texte réécrit. En cas d'erreurs, appeler `invalid(…)` avec une raison faite d'une ligne `fichier:ligne: message` par erreur (triées par fichier puis ligne). L'empreinte reste calculée sur le disque. Compléter les tests unitaires du chargeur : import valide réécrit, import avec version → invalide.
- [X] T018 [US1] Créer `examples/templates/packages-demo/` (FR-019) :
  - `template.json` ;
  - `schema.json`, avec `data.title`, `data.amount` (chaîne décimale), `data.reference` (texte du QR code) et `data.series` (tableau de nombres) ;
  - `main.typ`, qui importe **par le nom seul** `@preview/tiaoma`, `@preview/zero` et `@preview/cetz-plot` et affiche un QR code, le montant formaté à la française (« 1 234,56 ») et un graphique à barres.

  Créer aussi `examples/requests/packages-demo.json`.

**Checkpoint** : T008–T013 passent ; quickstart.md P2 est vérifié à la main avec `cargo run`.

---

## Phase 4: User Story 2 - Être prévenu d'un import incorrect (Priority: P2)

**Goal** : tout import autre que `@preview/<nom disponible>` rend le template invalide avec un message clair ; une erreur dans un paquet indique le paquet, le fichier et la ligne.

**Independent Test** : déposer les fixtures « version écrite » et « paquet inconnu » et lire `status` et `reason` via `GET /templates/{id}`.

### Tests for User Story 2 ⚠️

- [X] T019 [P] [US2] Créer les fixtures `tests/fixtures/templates/{version-written,unknown-package,other-namespace,dynamic-import}/`, chacune avec `main.typ` et un `schema.json` minimal. Elles importent respectivement `@preview/zero:0.7.1`, `@preview/does-not-exist`, `@local/zero` et `("@preview/" + "zero")`. Dans `tests/api_templates.rs`, vérifier :
  - pour les trois premières, `status: invalid` avec la raison exacte de contracts/template-imports.md (fichier et ligne compris) ;
  - qu'une génération répond 409 `template-invalid` ;
  - pour `dynamic-import`, `status: valid`, puis une génération en 500 `render-failed`.
- [X] T020 [P] [US2] Dans `tests/api_render.rs` (FR-012), écrire un test : une fixture `tests/fixtures/templates/package-error/` provoque une erreur dans le code d'un paquet (par exemple un argument invalide passé à une fonction de `zero`). La réponse 500 `render-failed` contient un diagnostic dont `file` commence par `@preview/zero:0.7.1/` et dont `line` est renseigné.

### Implementation for User Story 2

- [X] T021 [US2] Dans `src/render/world.rs`, faire renvoyer à `SandboxWorld::relative_path` la chaîne `@preview/<nom>:<version>/<chemin>` pour `VirtualRoot::Package(spec)` (R7). Vérifier que `src/render/mod.rs::to_diagnostic` calcule la ligne pour ces fichiers, la source étant obtenue via le même `World` grâce au cache de T015.
- [X] T022 [US2] Vérifier que les messages et le tri de `src/template/imports.rs` et `src/registry/loader.rs` correspondent mot pour mot au contrat. Ajuster si T019 échoue, puis relancer T019 et T020.

**Checkpoint** : US1 et US2 fonctionnent ; quickstart.md P3 et P4 sont vérifiés.

---

## Phase 5: User Story 4 - Faire évoluer la liste en toute sécurité (Priority: P2)

**Goal** : la liste fixée est la seule source de vérité, elle est vérifiée par la construction et par les tests, et changer une version est simple.

**Independent Test** : changer la version `selected` d'un paquet avec le script, construire, et constater que les templates utilisent la nouvelle ; une archive altérée fait échouer la construction.

### Tests for User Story 4 ⚠️

- [X] T023 [P] [US4] Dans `tests/bundled_packages.rs`, écrire un test **d'intégrité du dépôt**. Il relit `packages/lock.toml` avec `toml` et `serde` en dev-dependencies, puis vérifie :
  - que le sha256 de chaque archive de `packages/vendor/` correspond, avec `sha2` en dev-dependency ;
  - que chaque archive a une entrée ;
  - qu'il y a au plus un `selected` par nom ;
  - que les entrées sont triées ;
  - que `inkpdf::packages::all()` correspond exactement au lock (noms, versions, rôles).

### Implementation for User Story 4

- [X] T024 [US4] Tester `scripts/add-package.sh` en mode remplacement sur une copie temporaire du dépôt. Par exemple, remplacer `showybox` 2.0.4 par une version antérieure publiée, puis vérifier que l'entrée est remplacée, l'ancienne archive supprimée et `cargo test --test bundled_packages` vert. Ne pas committer ce changement. Corriger le script si besoin.
- [X] T025 [P] [US4] Rédiger la section « Mainteneurs : faire évoluer les paquets » dans `docs/packages.md` (création du fichier si absent) : règles de contracts/lock-file.md (ajout, remplacement, retrait, échec avec le moteur) et usage du script.

**Checkpoint** : US4 vérifiée ; la procédure SC-007 (T007) est documentée.

---

## Phase 6: User Story 3 - Découvrir les paquets disponibles (Priority: P3)

**Goal** : `GET /packages` et la documentation présentent les 21 paquets mis à disposition.

**Independent Test** : `curl /packages` renvoie 21 éléments, chacun avec `import: "@preview/<nom>"`.

### Tests for User Story 3 ⚠️

- [X] T026 [P] [US3] Créer `tests/api_packages.rs` (contracts/api.md). Vérifier :
  - `GET /packages` renvoie 200 `application/json` avec 21 éléments triés par `name` ;
  - chaque élément a exactement les champs `name`, `import`, `version`, `description` et `license` ;
  - `import == "@preview/" + name` ;
  - aucun paquet `dependency` n'est présent (ex. `komet`, `suiji`) ;
  - `zero` a la version 0.7.1.
- [X] T027 [P] [US3] Dans `tests/bundled_packages.rs`, écrire un test de **synchronisation de la doc** (R11.4) : `docs/packages.md` mentionne `@preview/<nom>` pour chacun des 21 paquets `selected`, et aucun autre `@preview/…` dans sa section « Paquets disponibles ».

### Implementation for User Story 3

- [X] T028 [US3] Créer `src/api/packages.rs` :
  - une DTO `PackageInfo` (`ToSchema`, champs de contracts/api.md, motifs `pattern` sur `name` et `version`) ;
  - une DTO `PackageList` ;
  - un handler `list_packages` annoté `#[utoipa::path(get, path = "/packages", tag = "packages", responses(200))]`, qui projette `packages::all()` filtré sur `selected`.

  L'enregistrer dans `src/api/mod.rs` (route `OpenApiRouter` et tag `packages` dans `ApiDoc`).
- [X] T029 [US3] Régénérer `openapi/openapi.json` avec `INKPDF_UPDATE_OPENAPI=1 cargo test --test contract_openapi`, relire le diff (seulement `/packages` et les nouveaux schémas), puis relancer le test de contrat sans la variable.
- [X] T030 [US3] Rédiger `docs/packages.md`, section « Paquets disponibles » : pour chacun des 21 paquets, ce qu'il fait (reprendre `research/carte-paquets.md`), la ligne d'import, la version installée, la licence et la mention WASM le cas échéant. Ajouter une section « Licences » avec les 29 paquets et leurs licences.

**Checkpoint** : les quatre stories fonctionnent indépendamment.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T031 [P] Dans `docs/templates.md`, remplacer la ligne « `#import "@preview/..."` → échec » par une section « Utiliser un paquet » qui reprend les 8 règles de contracts/template-imports.md, avec un exemple et un lien vers `docs/packages.md` et `GET /packages`.
- [X] T032 [P] Dans `specs/001-pdf-generation-service/contracts/template-format.md`, ajouter une note sur la ligne du tableau concernant `@preview` : « remplacé par la feature 002, voir specs/002-typst-packages/contracts/template-imports.md ».
- [X] T033 [P] Dans `README.md`, mentionner les paquets intégrés (fonctionnalités, `GET /packages`, lien vers docs/packages.md), décrire la limite WASM (R12) dans la section limites, et ajouter `GET /packages` à la liste des routes.
- [X] T034 [P] Mettre à jour le skill `.claude/skills/typst-dev/SKILL.md` et `reference/packages.md` : dans inkpdf, import par le nom seul, aucune version, liste dans `packages/lock.toml`, ajout via `scripts/add-package.sh`.
- [X] T035 Dans `tests/perf.rs` (R14, SC-005, SC-006), ajouter un test ignoré qui mesure le p95 de `packages-demo` (moins de 1 s) et vérifier que le p95 de `sample` reste dans la limite existante. Lancer `cargo test --release --test perf -- --ignored` et noter les chiffres.
- [X] T036 Lancer `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` et `cargo test`, puis corriger.
- [X] T037 Dérouler quickstart.md de bout en bout avec `docker compose up --build`, scénarios P1 à P5. Vérifier la taille de l'image et que l'image démarre et génère `packages-demo`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)** : T001 → T002 ; T003 et T004 en parallèle.
- **Foundational (Phase 2)** : dépend de T002 et T003. T005 → T006 → T007. Bloque toutes les stories.
- **US1 (Phase 3)** : dépend de la Phase 2.
- **US2 (Phase 4)** : dépend de T014, T016 et T017 (US1). Peut commencer dès que le chargeur réécrit les imports.
- **US4 (Phase 5)** : dépend de la Phase 2 seulement ; indépendante de US1 à US3.
- **US3 (Phase 6)** : dépend de la Phase 2 seulement ; T030 avant T027 pour que le test passe.
- **Polish (Phase 7)** : après les stories visées.

### Within Each User Story

- Les tests sont écrits et DOIVENT échouer avant l'implémentation.
- US1 : T014 et T015 (World), puis T016 → T017 (imports, chargeur), puis T018 (exemple).
- US3 : T028 → T029 ; T030 indépendant.

### Parallel Opportunities

- Phase 1 : T003 ∥ T004 (pendant T001 et T002).
- US1 : T008, T009, T011, T012 et T013 en parallèle (fichiers distincts ; T010 dépend de T009).
- US2 : T019 ∥ T020.
- US4 et US3 en parallèle de US1 et US2 une fois la Phase 2 terminée.
- Polish : T031 à T034 en parallèle.

---

## Parallel Example: User Story 1

```text
# Tests US1 en parallèle :
Task: "T008 Test d'import des 29 paquets dans tests/bundled_packages.rs"
Task: "T009 Fixtures d'usage tests/fixtures/package-smoke/<nom>.typ"
Task: "T011 Test de fermeture dans tests/bundled_packages.rs"
Task: "T012 Tests de confinement dans tests/sandbox.rs"
Task: "T013 Test de bout en bout packages-demo dans tests/api_render.rs"
```

(T008 et T011 touchent le même fichier : à écrire ensemble, ou l'un après l'autre.)

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phases 1 et 2 : les paquets sont dans le binaire.
2. Phase 3 (US1) : imports par le nom, `packages-demo` génère son PDF.
3. **STOP et valider** : tests US1 verts, quickstart.md P2.

### Incremental Delivery

1. MVP (US1), puis US2 (erreurs claires), puis US4 (évolution sûre), puis US3 (découverte).
2. Chaque story est testable seule et ne casse pas les précédentes.

### Politique d'échec d'un paquet (R11)

Si T008 ou T010 échoue pour un paquet avec Typst 0.15.1 :
- le retirer du lock et de `vendor/`, avec ses dépendances devenues orphelines ;
- l'indiquer dans le résumé à l'utilisateur ;
- ne jamais corriger le paquet localement.

Ajuster en conséquence les compteurs (21, 29) dans les tests et la doc.

---

## Notes

- [P] = fichiers différents, sans dépendance.
- Commit à la fin de chaque phase. Les messages de commit ne mentionnent pas Claude.
- `build.rs` est la seule génération de code ; ne pas committer `OUT_DIR`.

---

# Extension du périmètre (2026-10-10)

## Phase 8: US7 - `design` → `layout` (Priority: P2)

- [X] T038 [US7] Renommer `design` en `layout` dans les fichiers suivants (R16) :
  - `src/template/schema.rs`, `src/render/world.rs`, `src/api/render.rs`, `src/api/mod.rs` ;
  - tests unitaires et `tests/*.rs` ;
  - `examples/templates/sample/` (`schema.json`, `main.typ`), `examples/templates/packages-demo/`, `examples/requests/*.json` ;
  - fixtures concernées.

  Ajouter un test vérifiant qu'un corps contenant `design` est refusé (422, `/design`).
- [X] T039 [US7] Amender `.specify/memory/constitution.md` en version 1.0.2 : principe III, « paramètres de design » devient « paramètres de mise en page (`layout`) », avec un Sync Impact Report. Mettre à jour `docs/templates.md` et le README. Ajouter une note dans `specs/001-pdf-generation-service/contracts/template-format.md`. Régénérer `openapi/openapi.json`.

## Phase 9: US6 - Métadonnées (Priority: P2)

- [ ] T040 [P] [US6] Écrire `tests/api_metadata.rs` :
  - métadonnées complètes, avec lecture des propriétés du PDF (`/Title`, `/Author`, `/Subject`, `/Keywords`, `/CreationDate`) via une recherche dans le flux XMP ou le dictionnaire `Info`, sans nouvelle dépendance ;
  - corps sans `metadata` : auteur `inkpdf`, titre égal au nom du template ;
  - `default_author` configuré ;
  - propriété inconnue ou type incorrect : 422, chemin `/metadata/...` ;
  - déterminisme.
- [ ] T041 [US6] Créer `src/render/metadata.rs` :
  - une struct `DocumentMetadata` ;
  - le schéma fixe de R17, validé par `jsonschema` ;
  - `extract(body) -> Result<(Value, Option<DocumentMetadata>), Vec<Violation>>` ;
  - `apply(&mut DocumentInfo, &DocumentMetadata?, template_name, default_author)`.

  Ajouter `default_author` à `src/config.rs` (`INKPDF_DEFAULT_AUTHOR`, défaut `inkpdf`) et à ses tests. Brancher le tout dans `src/api/render.rs` (fusion des violations) et dans `src/render/mod.rs::compile_pdf` (application via `info_mut()` avant `typst_pdf::pdf`). Mettre à jour `RenderRequest` (champ `metadata` documenté) et `tests/common` (`compile`).

## Phase 10: US5 - Téléchargement (Priority: P2)

- [ ] T042 [P] [US5] Ajouter des tests dans `tests/api_render.rs` :
  - `?download=true&filename=Facture 042` donne `attachment; filename="Facture 042.pdf"; filename*=UTF-8''Facture%20042.pdf` ;
  - sans nom, `<id>.pdf` ;
  - nom dangereux nettoyé ;
  - nom accentué en ASCII de repli plus `filename*` ;
  - `download=oui` donne 400 `invalid-parameter` ;
  - sans paramètre, `inline` inchangé.
- [ ] T043 [US5] Dans `src/api/render.rs`, ajouter une struct `RenderQuery` (`IntoParams`) et une fonction pure `content_disposition(id, download, filename)` avec ses tests unitaires. Ajouter `ErrorCode::InvalidParameter` (400) dans `src/error.rs`. Régénérer l'OpenAPI.

## Phase 11: US8 - Exemples riches et Bruno (Priority: P2)

- [ ] T044 [US8] Créer `examples/templates/facture-situation/` (R19) :
  - `template.json` ;
  - `schema.json` complet : `data` (entreprise, client, chantier, marché, situation, lots et postes, avenants, conditions) et `layout` avec ses valeurs par défaut ;
  - `assets/logo.svg` de démonstration ;
  - `main.typ` et des sous-fichiers `parts/` (en-tête, tableau, récapitulatif, conditions).
- [ ] T045 [US8] Créer `examples/requests/facture-situation/` avec ces requêtes :
  - `01-premiere-situation.json` : retenue de garantie 5 %, TVA 20 %, une page ;
  - `02-situation-longue.json` : au moins 3 pages, plusieurs lots, avenants, révision de prix, remboursement d'avance, compte prorata, QR SEPA ;
  - `03-sous-traitance-autoliquidation.json` : autoliquidation, caution bancaire au lieu de la retenue, `layout` différent (logo à droite, autres couleurs, densité compacte) ;
  - `04-multi-taux-tva.json` : rénovation à 10 % et 5,5 %, métadonnées et sans montant en lettres.
- [ ] T046 [P] [US8] Écrire `tests/examples.rs` :
  - chaque requête d'exemple, y compris `sample` et `packages-demo`, produit un PDF ;
  - `01` est vérifiée au centime par un calcul indépendant fait dans le test ;
  - `02` fait au moins 3 pages ;
  - `03` mentionne l'autoliquidation et aucune TVA.
- [ ] T047 [US8] Créer la collection Bruno `examples/bruno/` :
  - `bruno.json` et `environments/local.bru` ;
  - une requête par route : santé, disponibilité, liste, détail, schéma, paquets, OpenAPI ;
  - les rendus : `sample`, `packages-demo`, chaque variante de facture, un téléchargement, des métadonnées, un import incorrect.

  Ajouter dans `tests/examples.rs` un test vérifiant que chaque `.bru` cible une route présente dans `openapi/openapi.json`. Documenter l'usage dans le README.

## Phase 12: Polish (extension)

- [ ] T048 Mettre à jour `quickstart.md` (paramètres `download` et `filename`, `metadata`, `layout`, Bruno), `docs/templates.md` (métadonnées et `layout`), `synthese.md` et la mémoire du projet.
- [ ] T049 Lancer `cargo fmt`, `clippy -D warnings` et `cargo test`. Rejouer la collection Bruno contre `docker compose up --build`, avec `bru run` si la CLI est disponible, sinon avec `curl` sur les mêmes requêtes.
