---

description: "Liste des tâches d'implémentation du service inkpdf V1"
---

# Tasks: Service de génération de PDF à partir de templates (V1)

**Input**: Design documents from `specs/001-pdf-generation-service/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: INCLUS — la constitution (section « Workflow de développement et qualité ») impose des
tests de validation JSON, de contrat API, de génération de bout en bout et de bac à sable. Dans
chaque story, les tests sont écrits en premier et DOIVENT échouer avant l'implémentation.

**Organization**: une phase par user story, dans l'ordre de priorité de la spec.

**Rappel transverse (FR-024)** : le service est agnostique du contenu. Aucune tâche ne doit
introduire de type de document, de champ métier ou de logique propre à un domaine dans `src/`.
Les seuls contenus concrets vivent dans `examples/` et `tests/fixtures/`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallélisable (fichiers différents, aucune dépendance sur une tâche non terminée)
- **[Story]**: user story concernée (US1 à US4)
- Chemins relatifs à la racine du dépôt (projet unique : `src/`, `tests/`)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: initialisation de la crate et de l'outillage

- [X] T001 Créer `Cargo.toml` (crate `inkpdf`, édition 2024, `rust-version = "1.98"`, lib + bin `inkpdf`, `license = "MIT OR Apache-2.0"` avec commentaire `# TODO(LICENSE)`) avec les dépendances épinglées de plan.md « Technical Context » : `typst = "=0.15.1"`, `typst-pdf = "=0.15.1"`, `typst-kit = { version = "=0.15.1", default-features = false, features = ["embedded-fonts"] }`, `comemo = "0.5"` (même version que celle tirée par typst), `axum = "0.8.9"`, `tokio` (features `rt-multi-thread`, `macros`, `signal`, `sync`, `time`, `fs`), `tower-http = "0.7"` (features `limit`, `trace`), `utoipa = "=6.0.0"` (feature `axum_extras`), `utoipa-axum = "=0.3.0"`, `utoipa-scalar = { version = "=0.4.0", features = ["axum"] }`, `jsonschema = { version = "=0.58.6", default-features = false }`, `notify = "8.2"`, `arc-swap`, `serde` (derive), `serde_json`, `thiserror`, `tracing`, `tracing-subscriber` (features `json`, `env-filter`), `time` (feature `formatting`), `regex`, `num_cpus` ; dev-dependencies : `tower` (feature `util`), `http-body-util`, `tempfile`, `pdf-extract`, `criterion`, `serde_yaml` ; puis vérifier avec `cargo tree -i reqwest` qu'aucune dépendance HTTP cliente n'est tirée
- [X] T002 Créer l'arborescence de plan.md « Source Code » avec des modules vides compilables : `src/main.rs`, `src/lib.rs`, `src/config.rs`, `src/error.rs`, `src/api/{mod,templates,render,health}.rs`, `src/registry/{mod,loader,fingerprint,watcher}.rs`, `src/template/{mod,id,manifest,schema}.rs`, `src/render/{mod,world,fonts,value}.rs`, `tests/common/mod.rs`, `benches/render.rs` (déclaré `[[bench]] name = "render" harness = false` dans `Cargo.toml`)
- [X] T003 [P] Ajouter `rustfmt.toml` (édition 2024), `clippy.toml`, `.gitignore` (`/target`), `.dockerignore` (`target`, `.git`, `specs`, `.specify`, `.claude`) et `rust-toolchain.toml` (`channel = "1.98"`, composants `rustfmt`, `clippy`)
- [X] T004 [P] Créer le workflow CI `.github/workflows/ci.yml` : `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, puis `cargo test --release -- --ignored` (tests de performance) sur `ubuntu-latest`
- [X] T005 [P] Créer le template de démonstration neutre `examples/templates/sample/` : `template.json`, `schema.json` (copie exacte de l'exemple de contracts/template-format.md : `data.title`, `data.items[] {label, value}`, `design.primaryColor`, `design.align`, `design.showFooter` avec leurs `default`), `main.typ` (utilise `sys.inputs.data` / `sys.inputs.design` comme dans le contrat, tableau des items, couleur, alignement), `parts/footer.typ` (texte « inkpdf sample footer ») ; et `examples/requests/sample.json` (titre « Exemple », 20 items dont le premier « Alpha »)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: configuration, erreurs, modèle de template, registre chargé au démarrage, squelette
HTTP. Toutes les stories en dépendent.

**⚠️ CRITICAL**: aucune story ne commence avant la fin de cette phase.

### Tests fondationnels (écrire d'abord, doivent échouer)

- [X] T006 [P] Tests unitaires `TemplateId` dans `src/template/id.rs` (`#[cfg(test)]`) : acceptés `sample`, `a`, `a-b_c9`, 64 caractères ; refusés chaîne vide, `-a`, `A`, `../x`, `a/b`, `a.b`, `%2e`, 65 caractères
- [X] T007 [P] Tests unitaires du chargeur dans `src/registry/loader.rs` (`#[cfg(test)]`, dossiers via `tempfile`) : template valide → `Valid` avec nom/description/version ; sans `main.typ` → `Invalid` avec raison mentionnant `main.typ` ; `schema.json` illisible → `Invalid` ; racine du schéma sans `properties.data` → `Invalid` ; propriété racine autre que `data`/`design` → `Invalid` ; `$ref` externe (`https://…`) → `Invalid` ; `template.json` avec clé inconnue → `Invalid` ; sans `template.json` → nom = identifiant ; police illisible dans `fonts/` → `Invalid` ; dossier dont la taille totale dépasse `max_template_bytes` → `Invalid` avec raison citant la taille et la limite, **sans qu'aucun fichier ne soit lu**
- [X] T008 [P] Tests unitaires de l'empreinte dans `src/registry/fingerprint.rs` (`#[cfg(test)]`) : même contenu → même empreinte ; modification d'un fichier (taille ou mtime) → empreinte différente ; ajout/suppression d'un fichier → différente ; ordre de parcours sans effet

### Implémentation fondationnelle

- [X] T009 [P] Implémenter `Config` dans `src/config.rs` : struct aux champs publics typés (`templates_dir: PathBuf`, `listen: SocketAddr`, `max_body_bytes: usize`, `render_timeout: Duration`, `max_concurrent_renders: usize`, `queue_timeout: Duration`, `rescan_interval: Duration`, `max_template_bytes: u64`, `log_format`), `impl Default` reprenant les défauts du tableau « Configuration » de plan.md (`max_concurrent_renders` = `num_cpus::get()`), et `Config::from_env()` qui part de `Default` et lit les variables `INKPDF_*` ; erreur explicite si une valeur est mal formée ; tests unitaires des défauts et d'une valeur invalide. Seul `main.rs` appelle `from_env()` ; les tests construisent `Config` directement
- [X] T010 [P] Implémenter `ApiError` dans `src/error.rs` : une variante par code du tableau « Codes d'erreur » de data-model.md (`TemplateNotFound`, `InvalidJson`, `UnsupportedMediaType`, `ValidationFailed { violations }`, `PayloadTooLarge`, `TemplateInvalid { reason }`, `RenderFailed { diagnostics }`, `RenderTimeout`, `Overloaded`), structs `Problem`, `Violation`, `Diagnostic` (`serde` + `utoipa::ToSchema`, champs en camelCase conformes à contracts/openapi.yaml), `impl IntoResponse` produisant `application/problem+json` avec `type = https://github.com/ck-developer/inkpdf/errors/<code>`, `title`, `status`, `code`, `detail`, `templateId` éventuel ; tests unitaires du statut HTTP et du `Content-Type` pour chaque variante
- [X] T011 [P] Implémenter `TemplateId` dans `src/template/id.rs` : newtype validé par `^[a-z0-9][a-z0-9_-]{0,63}$`, `FromStr`, `Display`, `Serialize`
- [X] T012 [P] Implémenter le manifeste dans `src/template/manifest.rs` : `Manifest { name: Option<String>, description: Option<String>, version: Option<String> }` avec `#[serde(deny_unknown_fields)]` et bornes de longueur (name 1–120, description ≤ 2000, version ≤ 64)
- [X] T013 Implémenter le chargement de schéma dans `src/template/schema.rs` : `TemplateSchema::load(bytes)` qui conserve les **octets d'origine** (`raw: Bytes`, exposés tels quels — constitution VI), parse le JSON, vérifie la racine (`type: object`, `properties.data` présent, seules `data`/`design` déclarées à la racine), refuse tout `$ref` non interne (ne commençant pas par `#`), puis compile un validateur `jsonschema` draft 2020-12 sans retriever externe à partir d'une **copie** du schéma à laquelle `additionalProperties: false` est ajouté à la racine s'il est absent ; le schéma exposé n'est jamais modifié (dépend de T011)
- [X] T014 [P] Implémenter les polices dans `src/render/fonts.rs` : `EmbeddedFonts` chargées une seule fois (`OnceLock`) depuis `typst-kit` `embedded-fonts`, et `load_dir_fonts(path) -> Result<Vec<(FontInfo, Font)>>` pour les `.ttf/.otf/.ttc` d'un dossier `fonts/` ; erreur si un fichier ne contient aucune police lisible ; aucune police système
- [X] T015 [P] Implémenter `Fingerprint::of(dir)` dans `src/registry/fingerprint.rs` : parcours récursif trié (en suivant les liens restant sous le dossier du template), hachage de (chemin relatif, taille, mtime), représentation courte hexadécimale, et taille totale des fichiers (`total_bytes`, utilisée pour la limite `max_template_bytes` avant lecture) (utilisée par le chargeur et par l'`ident` déterministe du PDF)
- [X] T016 Implémenter `TemplateEntry` et le chargeur dans `src/registry/loader.rs` : `load(dir, max_template_bytes) -> TemplateEntry` avec `id`, `name` (défaut identifiant), `description`, `version`, `status: Valid | Invalid { reason }`, `schema: Option<TemplateSchema>`, `files: HashMap<String, Bytes>` (**instantané en mémoire de tous les fichiers du dossier**, clé = chemin relatif normalisé avec `/`), `fonts` (chargées depuis `files`), `fingerprint`, `loaded_at` ; procédure : empreinte avant → si la somme des tailles dépasse `max_template_bytes`, `Invalid` immédiat sans lecture → lecture de tous les fichiers (un lien symbolique dont la cible sort du dossier du template est exclu de l'instantané avec un log `warn`) → empreinte après ; si les deux diffèrent, renvoyer `LoadOutcome::Unstable` (nouvel essai plus tard) ; appliquer les règles de validité 1 à 6 de data-model.md ; le fichier Typst n'est pas compilé ; ajouter aux tests T007 : lien symbolique sortant exclu de `files`, modification pendant la lecture → `Unstable` (dépend de T012, T013, T014, T015)
- [X] T017 Implémenter le registre dans `src/registry/mod.rs` : `Registry` contenant `ArcSwap<BTreeMap<TemplateId, Arc<TemplateEntry>>>` et un drapeau `ready: AtomicBool` ; `scan_all(dir)` (ignore les dossiers cachés et les noms invalides avec un log `warn`, volume absent ou vide → registre vide + log `warn`), `get(&TemplateId)`, `list()` trié par identifiant, `replace(map)` atomique (dépend de T016)
- [X] T018 Implémenter l'état partagé et le routeur dans `src/lib.rs` et `src/api/mod.rs` : `AppState { config, registry, render_slots: Arc<Semaphore> }`, `build_app(state) -> Router` basé sur `utoipa_axum::router::OpenApiRouter` avec `RequestBodyLimitLayer(max_body_bytes)` et `TraceLayer`, struct `ApiDoc` (`#[derive(OpenApi)]`, titre `inkpdf`, version de la crate, licence `TODO(LICENSE)`) ; pas encore de routes métier
- [X] T019 [P] Implémenter `GET /health` et `GET /ready` dans `src/api/health.rs` (schéma `Health` du contrat : `status` `ok`/`starting`, `templates` = nombre de templates valides, `version`) ; `/ready` répond `503` tant que `registry.ready` est faux ; annotations `#[utoipa::path]`
- [X] T020 Implémenter `src/main.rs` : initialisation `tracing-subscriber` (JSON ou pretty selon `INKPDF_LOG_FORMAT`, `RUST_LOG` défaut `info`), `Config::from_env()`, scan initial, `ready = true`, écoute sur `INKPDF_LISTEN`, arrêt propre sur SIGTERM/SIGINT ; sous-commande `inkpdf healthcheck` qui interroge `http://127.0.0.1:<port>/health` avec un client TCP minimal (sans dépendance HTTP cliente) et sort en code 0/1
- [X] T021 Créer les utilitaires de test dans `tests/common/mod.rs` : `TestVolume` (dossier `tempfile` + `copy_template(src, id)`, `write_file`, `remove`), `test_config(volume) -> Config` (= `Config { templates_dir, ..Config::default() }`, **jamais** via variables d'environnement, pour que les tests parallèles restent isolés), `test_app(config) -> Router` via `build_app`, helpers `get_json`, `post_json(uri, body) -> (StatusCode, HeaderMap, Bytes)` via `tower::ServiceExt::oneshot`, et `pdf_text(bytes) -> String` via `pdf-extract`

**Checkpoint**: `cargo test` passe (T006, T007, T008), `cargo run` démarre et `/health` répond.

---

## Phase 3: User Story 1 - Générer un PDF à partir d'un template et de données (Priority: P1) 🎯 MVP

**Goal**: `POST /templates/{id}/render` valide le corps puis renvoie le PDF, dans le bac à sable,
avec durée et concurrence bornées.

**Independent Test**: avec `examples/templates/sample` dans le volume, poster
`examples/requests/sample.json` renvoie `200 application/pdf` dont le texte contient « Exemple »
et « Alpha » ; un corps sans `data.title` renvoie `422` sans compilation.

### Tests for User Story 1 (écrire d'abord, doivent échouer)

- [X] T022 [P] [US1] Tests unitaires de conversion dans `src/render/value.rs` (`#[cfg(test)]`) : objet → `Dict`, tableau → `Array`, chaîne → `Str`, entier → `Int`, décimal → `Float`, booléen → `Bool`, `null` → `None`, entier hors plage `i64` → `Float`, objets imbriqués
- [X] T023 [P] [US1] Tests unitaires de validation dans `src/template/schema.rs` (`#[cfg(test)]`) : `design` absent → initialisé à `{}` puis défauts insérés ; propriété de `design` présente → non écrasée ; défauts non appliqués à `data` ; corps valide → `Ok` ; corps invalide → toutes les violations avec `path` (JSON Pointer instance), `schemaPath` et `message` ; clé racine inconnue → violation
- [X] T024 [P] [US1] Tests d'intégration HTTP dans `tests/api_render.rs` : (a) S2 du quickstart → `200`, `Content-Type: application/pdf`, `Content-Disposition: inline; filename="sample.pdf"`, texte contenant « Exemple » et « Alpha » ; (b) corps sans `design` → `200` ; (c) corps sans `data.title` → `422 validation-failed` avec violation `/data` ; (d) identifiant inconnu → `404 template-not-found` ; (e) identifiant `..%2Fetc` → `404` ; (f) corps non JSON → `400 invalid-json` ; (g) `Content-Type: text/plain` → `415 unsupported-media-type` ; (h) corps > `INKPDF_MAX_BODY_BYTES` → `413 payload-too-large` au format problem+json ; (i) template invalide → `409 template-invalid` ; (j) `main.typ` avec erreur Typst → `500 render-failed` avec au moins un `diagnostics[]` portant `file` et `line` ; (k) deux rendus identiques → octets identiques (FR-017)
- [X] T025 [P] [US1] Tests de bac à sable dans `tests/sandbox.rs` avec des templates malveillants dans `tests/fixtures/templates/` : `evil-package` (`#import "@preview/anything:0.1.0"`) → `500 render-failed` ; `evil-parent` (`#read("../../etc/passwd")`) → `500` ; `evil-absolute` (`#read("/etc/passwd")`) → `500` ; `evil-symlink` (lien `assets/x` vers un fichier hors du dossier, créé par le test) → `500` ; injection : `data.title = "#import \"/etc/passwd\""` sur `sample` → `200` et texte rendu littéralement
- [X] T026 [P] [US1] Tests de durée et de concurrence dans `tests/render_limits.rs` : fixture `tests/fixtures/templates/slow` (boucle longue mais finie lisant un fichier du template à chaque itération) avec `Config { render_timeout: 1 s, .. }` → `504 render-timeout` en moins de 2 s ; avec `max_concurrent_renders: 1` et `queue_timeout: 1 s`, un second rendu pendant le premier → `503 overloaded` ; après la fin du rendu lent, un rendu de `sample` réussit (le créneau est libéré)

### Implementation for User Story 1

- [X] T027 [P] [US1] Implémenter `json_to_value(&serde_json::Value) -> typst::foundations::Value` dans `src/render/value.rs` selon la table de correspondance de contracts/template-format.md
- [X] T028 [US1] Implémenter `TemplateSchema::prepare(body: serde_json::Value) -> Result<serde_json::Value, Vec<Violation>>` dans `src/template/schema.rs` : vérifie que la racine est un objet, initialise `design` à `{}` si absent, insère récursivement les `default` de `properties.design.properties` (et sous-objets), puis valide avec `iter_errors` et convertit chaque erreur en `Violation { path, schemaPath, message }` (dépend de T013)
- [X] T029 [US1] Implémenter `SandboxWorld` dans `src/render/world.rs` (trait `typst::World`) : `library` construite avec `Library::builder().with_inputs(dict! { "data" => …, "design" => … })` ; `main` = `main.typ` ; `source`/`file` refusent tout `FileId` avec `package()` (`FileError::Other("package imports are not supported")`), normalisent le chemin virtuel (refus de toute composante `..` sortant de la racine → `FileError::AccessDenied`) et servent **uniquement** l'instantané `entry.files` (`FileError::NotFound` sinon) — **aucun accès disque pendant le rendu** ; `book`/`font` = polices embarquées + polices du template ; `today` = date UTC ; un `Arc<AtomicBool>` d'annulation vérifié au début de `source`, `file` et `font` (retourne une erreur si levé) ; `Source` parsées mises en cache pour la durée d'une compilation (dépend de T014, T016, T027)
- [X] T030 [US1] Implémenter le pipeline dans `src/render/mod.rs` : `render(entry, prepared_body, &AppState) -> Result<Bytes, ApiError>` : acquisition d'un permis du sémaphore bornée par `config.queue_timeout` (sinon `Overloaded`) ; `spawn_blocking` qui compile (`typst::compile::<PagedDocument>`) puis exporte (`typst_pdf::pdf` avec `PdfOptions { ident: Smart::Custom(format!("{id}@{entry.fingerprint}")), timestamp: None, .. }`) et rend le permis à la fin réelle du thread ; `tokio::time::timeout(config.render_timeout)` → `RenderTimeout` et lève le drapeau d'annulation ; si le thread finit après le délai, log `warn` `render.overrun` avec `templateId` et durée ; erreurs Typst → `RenderFailed` avec `Diagnostic { message, file (relatif au template), line, column, hints }` ; `comemo::evict(10)` après chaque rendu (dépend de T029)
- [X] T031 [US1] Implémenter `POST /templates/{templateId}/render` dans `src/api/render.rs` : identifiant invalide ou inconnu → `TemplateNotFound` ; `Content-Type` ≠ `application/json` → `UnsupportedMediaType` ; corps non JSON → `InvalidJson` ; template `Invalid` → `TemplateInvalid` ; `prepare` → `ValidationFailed` ; puis `render` ; réponse `200` `application/pdf` + `Content-Disposition: inline; filename="<id>.pdf"` ; mapper le rejet de `RequestBodyLimitLayer` vers `PayloadTooLarge` en problem+json ; annotation `#[utoipa::path]` conforme à `renderTemplate` dans contracts/openapi.yaml (corps `RenderRequest`, toutes les réponses d'erreur) ; enregistrer la route dans `src/api/mod.rs` (dépend de T028, T030)
- [X] T032 [US1] Journaliser chaque génération dans `src/api/render.rs` : événement `render` avec `templateId`, `durationMs`, `outcome` (`ok` | `validation_failed` | `render_failed` | `timeout` | `overloaded`), sans aucune donnée du corps (FR-022)
- [X] T033 [US1] Créer les fixtures `tests/fixtures/templates/{broken-typst,evil-package,evil-parent,evil-absolute,slow}/` (chacune avec `main.typ` et un `schema.json` minimal acceptant `data: {}`) et un fixture `invalid-schema/` (`schema.json` = `{`) ; faire passer T022 à T026

**Checkpoint**: US1 complète — un appelant peut générer un PDF ; MVP livrable.

---

## Phase 4: User Story 2 - Personnaliser l'apparence sans dupliquer le template (Priority: P2)

**Goal**: les paramètres de design varient le rendu d'un même template ; défauts et valeurs
hors contrat gérés.

**Independent Test**: deux rendus de `sample` avec les mêmes `data` et `design.primaryColor`
différents produisent des PDF au texte identique et aux octets différents ;
`design.showFooter: false` retire le texte du pied de page.

### Tests for User Story 2 (écrire d'abord, doivent échouer)

- [X] T034 [P] [US2] Tests d'intégration dans `tests/api_design.rs` : (a) mêmes `data`, `primaryColor` `#1f4e79` vs `#c0392b` → textes extraits identiques, PDF différents ; (b) `showFooter: false` → texte « inkpdf sample footer » absent, présent sans `design` ; (c) `align: "top"` → `422` avec violation `/design/align` ; (d) `primaryColor: "red"` → `422` avec violation `/design/primaryColor` ; (e) propriété de design inconnue → `422` ; (f) violations simultanées sur `data` et `design` → toutes listées (S4 du quickstart)
- [X] T035 [P] [US2] Tests unitaires dans `src/template/schema.rs` : défauts appliqués récursivement à un sous-objet de `design` (fixture de schéma avec `design.header.visible` par défaut `true`) ; sous-objet fourni partiellement → défauts complétés sans écraser les valeurs fournies

### Implementation for User Story 2

- [X] T036 [US2] Compléter l'application récursive des défauts dans `src/template/schema.rs` pour les sous-objets de `design` (création de l'objet intermédiaire s'il a des propriétés avec `default`), si T028 ne le couvre pas déjà ; faire passer T035
- [X] T037 [US2] Vérifier que `examples/templates/sample/main.typ` exploite les trois paramètres (`primaryColor` appliqué au titre et à l'en-tête du tableau, `align` au titre, `showFooter` conditionne `#include "parts/footer.typ"`) et ajuster si besoin ; faire passer T034

**Checkpoint**: US1 et US2 fonctionnent indépendamment.

---

## Phase 5: User Story 3 - Découvrir les templates et leur contrat (Priority: P3)

**Goal**: liste, détail, schéma brut et description OpenAPI servie par le service, verrouillée
par un test de contrat.

**Independent Test**: avec `sample` et `invalid-schema` dans le volume, `GET /templates`
renvoie les deux (valid / invalid + reason), `GET /templates/sample` expose le schéma avec
`data` et `design`, `GET /openapi.json` contient tous les chemins du contrat.

### Tests for User Story 3 (écrire d'abord, doivent échouer)

- [X] T038 [P] [US3] Tests d'intégration dans `tests/api_templates.rs` : liste triée avec `id`, `name`, `description`, `version`, `status`, `reason` si invalide ; détail de `sample` avec `loadedAt`, `schema` (contenant `properties.data` et `properties.design`, `default` conservés) et `links.schema`/`links.render` ; détail d'un template invalide → `200` avec `status: invalid`, `reason`, sans `schema` ; `GET /templates/sample/schema` → `200` `application/schema+json` dont le corps est **identique octet pour octet** au fichier `schema.json` (y compris quand il ne déclare pas `additionalProperties`) ; schéma d'un template invalide → `409` ; inconnu → `404` sur les deux routes
- [X] T039 [P] [US3] Test de contrat dans `tests/contract_openapi.rs` : le document généré par `ApiDoc` (via `build_app`) est égal à `openapi/openapi.json` versionné, avec régénération si `INKPDF_UPDATE_OPENAPI=1`
- [X] T040 [P] [US3] Test dans `tests/api_docs.rs` : `GET /openapi.json` → `200` JSON avec `openapi` = `3.1.0` ; `GET /docs` → `200` HTML

### Implementation for User Story 3

- [X] T041 [P] [US3] Définir les DTO dans `src/api/templates.rs` : `TemplateSummary`, `TemplateDetail` (avec `loaded_at` sérialisé en RFC 3339 sous `loadedAt`, `schema: Option<serde_json::Value>`, `links`), `TemplateList { templates }`, avec `utoipa::ToSchema`, conformes à contracts/openapi.yaml
- [X] T042 [US3] Implémenter `GET /templates`, `GET /templates/{templateId}` et `GET /templates/{templateId}/schema` dans `src/api/templates.rs` à partir de `Registry::list`/`get` (schéma servi avec `Content-Type: application/schema+json`, `409 template-invalid` si invalide), annotations `#[utoipa::path]` avec les `operationId` `listTemplates`, `getTemplate`, `getTemplateSchema` ; enregistrer les routes dans `src/api/mod.rs`
- [X] T043 [US3] Servir `GET /openapi.json` (operationId `openapi`) et l'UI Scalar sur `GET /docs` dans `src/api/mod.rs` ; compléter `ApiDoc` (serveur, tags `templates`/`render`/`ops`, composants `Problem`, `Violation`, `Diagnostic`, `Health`, paramètre `TemplateId` avec son `pattern`)
- [X] T044 [US3] Générer et versionner `openapi/openapi.json` (`INKPDF_UPDATE_OPENAPI=1 cargo test --test contract_openapi`) ; vérifier une fois, à la main, l'alignement avec la référence de conception `specs/001-pdf-generation-service/contracts/openapi.yaml` (chemins, méthodes, `operationId`, codes de réponse, énumération `Problem.code`) et corriger les écarts jusqu'à ce que T038 à T040 passent

**Checkpoint**: US1 à US3 fonctionnent ; l'API est auto-descriptive.

---

## Phase 6: User Story 4 - Publier un template sans redéployer (Priority: P4)

**Goal**: ajout, modification et suppression de templates pris en compte à chaud en moins de 5 s,
sans état partiel pendant une copie.

**Independent Test**: service démarré sur un volume temporaire, copier `sample` sous `sample-copy`
→ visible et utilisable en moins de 5 s ; casser son `schema.json` → `invalid` ; le supprimer →
`404` ; `sample` reste `valid` tout du long.

### Tests for User Story 4 (écrire d'abord, doivent échouer)

- [X] T045 [P] [US4] Tests d'intégration dans `tests/hot_reload.rs` (watcher démarré, `Config { rescan_interval: 1 s, .. }`, attente par sondage toutes les 100 ms, échec au-delà de 5 s) : ajout d'un template → listé et rendable ; modification de `main.typ` (nouveau texte) → les rendus suivants contiennent le nouveau texte ; `schema.json` corrompu → `status: invalid` et `sample` toujours `valid` ; suppression → `404` ; volume initialement vide puis peuplé → template visible
- [X] T046 [P] [US4] Test de copie en cours dans `tests/hot_reload.rs` : écrire `main.typ` d'un template existant en deux temps espacés de 300 ms ; pendant l'écriture, les rendus utilisent l'ancienne version ou la nouvelle complète, jamais une erreur due à un fichier tronqué

### Implementation for User Story 4

- [X] T047 [US4] Implémenter `Registry::refresh(dir)` dans `src/registry/mod.rs` : pour chaque dossier, recalculer l'empreinte ; si inchangée → garder l'entrée ; si changée → la mémoriser comme « candidate » et programmer une observation de confirmation 1 s plus tard (sans attendre le rescan) : recharger seulement si l'empreinte est restée identique (stabilité) ; si `load` renvoie `Unstable`, conserver l'ancienne entrée et réessayer ; dossiers disparus → retirés ; publication par un seul `replace` atomique ; log `info` `template.loaded` / `template.removed` / `template.invalid` (avec `reason`) (dépend de T016)
- [X] T048 [US4] Implémenter le watcher dans `src/registry/watcher.rs` : `notify::recommended_watcher` récursif sur `INKPDF_TEMPLATES_DIR` ; événements regroupés avec anti-rebond de 500 ms puis `refresh` ; tâche tokio de rescan périodique toutes les `INKPDF_RESCAN_INTERVAL_SECS` ; si le watcher ne peut pas être créé (volume réseau), log `warn` et rescan seul ; démarrage depuis `src/main.rs` et via un helper exposé pour les tests (dépend de T047)
- [X] T049 [US4] Faire passer T045 et T046 ; vérifier que `GET /templates` reflète les statuts `invalid` produits à chaud

**Checkpoint**: les quatre stories sont fonctionnelles et testées indépendamment.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T050 [P] Écrire le `Dockerfile` multi-étapes : `rust:1.98-bookworm` (build `--release --locked`, cache des dépendances) → `gcr.io/distroless/cc-debian12:nonroot` ; binaire `/usr/local/bin/inkpdf`, `ENV INKPDF_TEMPLATES_DIR=/templates`, `VOLUME /templates`, `EXPOSE 3000`, `USER nonroot`, `HEALTHCHECK CMD ["/usr/local/bin/inkpdf","healthcheck"]` ; vérifier que l'image fait moins de 100 Mo (SC-009)
- [X] T051 [P] Créer le workflow `.github/workflows/docker.yml` : build multi-arch (amd64, arm64) et push vers `ghcr.io/ck-developer/inkpdf` sur tag `v*`
- [X] T052 [P] Écrire le benchmark `benches/render.rs` (criterion) : rendu de `examples/templates/sample` avec `examples/requests/sample.json` via le pipeline de `src/render/mod.rs` ; mesure fine et comparable entre versions (le seuil bloquant est dans T053)
- [X] T053 [P] Tests de performance dans `tests/startup.rs` et `tests/perf.rs` : volume de 50 copies de `sample` → registre prêt en moins de 2 s (SC-002) ; 20 rendus simultanés de `sample` → 20 succès (SC-007) ; `tests/perf.rs` marqué `#[ignore]` (exécuté en CI par `cargo test --release -- --ignored`, cf. T004) : 50 rendus séquentiels de `examples/requests/sample.json` après un rendu de chauffe, **échec si le p95 dépasse 200 ms** (SC-001, garde-fou de régression de la constitution)
- [X] T054 [P] Rédiger `README.md` : objectif (service générique, agnostique du contenu), démarrage Docker, variables `INKPDF_*`, résumé des endpoints, lien vers `/docs`, renvoi vers `docs/templates.md`
- [X] T055 [P] Rédiger `docs/templates.md` à partir de contracts/template-format.md (guide auteur : arborescence, `schema.json`, `sys.inputs`, défauts, restrictions du bac à sable, déterminisme et `datetime.today()`)
- [X] T056 Vérifier par revue que `src/` ne contient aucun nom de champ ou type de document métier (FR-024) : `grep -rniE "invoice|factur|customer|devis" src/` doit être vide
- [X] T057 Exécuter tous les scénarios de `specs/001-pdf-generation-service/quickstart.md` (local et Docker) et corriger les écarts ; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` verts

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)** : aucune dépendance.
- **Foundational (Phase 2)** : dépend de Setup ; bloque toutes les stories.
- **US1 (Phase 3)** : dépend de Foundational.
- **US2 (Phase 4)** : dépend de US1 (réutilise `prepare` et le pipeline de rendu).
- **US3 (Phase 5)** : dépend de Foundational seulement ; le test de contrat T039 exige que la
  route de rendu (T031) existe, donc à terminer après US1.
- **US4 (Phase 6)** : dépend de Foundational ; ses tests utilisent la liste (T042) et le rendu
  (T031), donc à terminer après US1 et US3.
- **Polish (Phase 7)** : après les stories visées.

### Ordre conseillé

```text
Setup → Foundational → US1 (MVP) → US2 ┐
                                 └→ US3 → US4 → Polish
```

### Within Each User Story

- Tests écrits d'abord et en échec, puis implémentation.
- `value.rs` / `schema.rs` → `world.rs` → `render/mod.rs` → endpoint.

### Parallel Opportunities

- Setup : T003, T004, T005 en parallèle après T001–T002.
- Foundational : T006–T012, T014, T015, T019 en parallèle ; puis T013 → T016 → T017 → T018 → T020/T021.
- US1 : T022–T026 (tests) en parallèle ; T027 en parallèle de T028.
- US2 : T034 et T035 en parallèle.
- US3 : T038–T041 en parallèle ; US3 peut démarrer en parallèle de US1 (seul T039 attend T031).
- US4 : T045 et T046 en parallèle ; le développement de T047–T048 peut avancer en parallèle de US1.
- Polish : T050–T055 en parallèle.

## Parallel Example: User Story 1

```text
# Tests en parallèle :
T022 tests de conversion          (src/render/value.rs)
T023 tests de validation/défauts  (src/template/schema.rs)
T024 tests HTTP de génération     (tests/api_render.rs)
T025 tests de bac à sable         (tests/sandbox.rs)
T026 tests de durée/concurrence   (tests/render_limits.rs)

# Puis en parallèle :
T027 json_to_value                (src/render/value.rs)
T028 TemplateSchema::prepare      (src/template/schema.rs)
```

## Parallel Example: User Story 3

```text
T038 tests liste/détail/schéma    (tests/api_templates.rs)
T039 test de contrat OpenAPI      (tests/contract_openapi.rs)
T040 test /openapi.json et /docs  (tests/api_docs.rs)
T041 DTO                          (src/api/templates.rs)
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 Setup, puis Phase 2 Foundational.
2. Phase 3 US1 : génération validée et en bac à sable.
3. **STOP & VALIDATE** : scénario S2, S4 (partie `data`) et S6 du quickstart.
4. Image Docker (T050) possible dès ce point pour un premier déploiement interne.

### Incremental Delivery

1. Setup + Foundational → socle.
2. US1 → génération (MVP).
3. US2 → personnalisation par `design`.
4. US3 → découverte et OpenAPI (auto-description complète, exigée par la constitution avant la
   première release publique).
5. US4 → templates à chaud.
6. Polish → Docker, docs, mesures SC-001/002/007/009.

## Notes

- [P] = fichiers différents, aucune dépendance sur une tâche non terminée.
- Les tests de chaque story doivent échouer avant l'implémentation.
- Committer après chaque tâche ou groupe logique.
- Avant publication : trancher `TODO(LICENSE)` (constitution) et remplacer la licence provisoire
  dans `Cargo.toml` et l'OpenAPI.
