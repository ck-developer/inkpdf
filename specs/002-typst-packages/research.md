# Research — Paquets Typst intégrés (002)

Études détaillées déjà menées et sourcées :

- [research/typst.md](./research/typst.md) : paquets, `World`, typst-pdf 0.15.1 ;
- [research/landscape.md](./research/landscape.md) : autres moteurs ;
- [research/architecture.md](./research/architecture.md) : options d'architecture ;
- [research/carte-paquets.md](./research/carte-paquets.md) : classement des 827 bibliothèques ;
- [synthese.md](./synthese.md) : décisions de l'utilisateur.

Ce fichier consolide les décisions techniques du plan.

## R1 — Mode d'intégration : archives versionnées dans le dépôt, table générée par `build.rs`

**Décision.**
- Les archives officielles (`https://packages.typst.org/preview/<nom>-<version>.tar.gz`) sont
  **committées** dans `packages/vendor/`.
- `build.rs` les vérifie, les décompresse dans `OUT_DIR` et génère `bundled_packages.rs`, qui
  contient une table statique `&[BundledPackageDef]`.
- Chaque fichier est inclus avec `include_bytes!`.
- Le module `src/packages` inclut ce fichier généré.

**Rationale.**
- C'est un binaire unique et autosuffisant (principe II : « ressources embarquées par le
  binaire »).
- `cargo build` et `cargo test` fonctionnent **sans réseau** et sans Docker, ce qui couvre la
  CI, le poste local et l'image.
- Le coût d'exécution est nul : pas de décompression au démarrage, pas de lecture disque.
- Aucune nouvelle dépendance d'exécution.
- Le poids est négligeable : 1,74 Mo compressés dans le dépôt, environ 6 Mo dans le binaire.

**Alternatives écartées.**
- *Téléchargement dans le `Dockerfile` vers un dossier lu au démarrage* : `cargo test` aurait
  besoin du réseau ou d'un script, la vérité serait répartie entre le Dockerfile et le code, et
  le binaire seul serait incomplet.
- *`include_bytes!` des `.tar.gz` puis décompression au démarrage* : il faudrait `flate2` et
  `tar` à l'exécution, ainsi qu'un coût de démarrage et un état global initialisé paresseusement.
- *Le stockage de paquets de `typst-kit`* : il est lié au disque et au téléchargement
  (research/typst.md §1.7).

## R2 — Fichier de verrouillage `packages/lock.toml`

**Décision.** Un tableau TOML `[[package]]` avec les champs suivants :

| Champ | Contenu |
|---|---|
| `namespace` | toujours `"preview"` |
| `name` | nom du paquet |
| `version` | version exacte |
| `sha256` | empreinte de l'archive |
| `license` | identifiant SPDX, recopié de `typst.toml` pour la revue |
| `role` | `"selected"` (choisi, l'un des 21) ou `"dependency"` (ajouté pour fermer l'ensemble) |

L'ordre des entrées est alphabétique, puis par version. Le format complet est dans
[contracts/lock-file.md](./contracts/lock-file.md).

**Rationale.**
- C'est un fichier lisible en revue de PR.
- C'est la seule source de vérité : l'archive doit exister et correspondre à son empreinte.
- `role` permet de distinguer, dans l'API et la documentation, les paquets choisis de ceux qui
  ne sont là que comme dépendances.

**Alternatives écartées.**
- *JSON* : moins lisible et sans commentaires.
- *Pas de `role`* : on ne pourrait plus dire pourquoi un paquet est présent.

## R3 — Intégrité et vérifications de construction

**Décision.** `build.rs` fait échouer la compilation dans les cas suivants, avec un message qui
nomme le paquet :

- archive absente ;
- sha256 différent ;
- archive illisible ;
- `typst.toml` absent ;
- `name` ou `version` du manifeste différents de ceux du lock ;
- `entrypoint` introuvable ;
- archive présente dans `vendor/` sans entrée dans le lock.

`build.rs` déclare `cargo:rerun-if-changed=packages`.

**Rationale.** Couvre FR-006 et SC-007. Une archive altérée ne peut pas atteindre le binaire.

**Alternatives écartées.**
- *Vérifier dans un test seulement* : le binaire pourrait être construit malgré une archive
  altérée.

## R4 — Contenu embarqué

**Décision.**
- On embarque **tous les fichiers** de l'archive (sources, plugins `.wasm`, données, `LICENSE`,
  `README`), sans filtrage.
- Les chemins sont normalisés : séparateur `/`, sans `./` initial. Un chemin contenant `..` ou
  absolu provoque une erreur de construction.
- Les métadonnées sont lues de `typst.toml` par `build.rs` (parse TOML générique) et exposées
  dans la table : `description`, `license`, `entrypoint`, `compiler`.

**Rationale.**
- Les licences sont conservées dans le binaire (FR-016).
- Le surcoût est faible : environ 6 Mo.
- Filtrer les fichiers (tests, docs) risquerait de casser un paquet qui les importe.

## R5 — Résolution dans `SandboxWorld`

**Décision.** `lookup(id)` branche sur `id.root()` :

- `VirtualRoot::Project` : comportement inchangé (instantané du template).
- `VirtualRoot::Package(spec)` :
  - si `spec.namespace == "preview"` et que le paquet est dans la table, on renvoie le fichier
    `vpath` du paquet, ou `FileError::NotFound` s'il n'existe pas ;
  - sinon, on renvoie `FileError::Package(PackageError::Other(Some(msg)))`, où `msg` vaut
    « package @preview/x:1.2.3 is not bundled with inkpdf (available versions: 0.7.1) » ou
    « … (no version of this package is bundled; see GET /packages) ».

L'isolation est **native** :

- la racine `Package(spec)` est étanche ;
- `..` au-delà de la racine est refusé par `VirtualPath` ;
- un paquet ne peut pas construire un chemin `Project` ;
- seul un `path`, des `bytes` ou une `image` passés explicitement par le template traversent
  (research/typst.md §1.6, FR-012).

Un dossier `packages/` présent dans un template reste un simple fichier de projet : il n'est
jamais consulté pour résoudre un import (spec, edge case).

**Rationale.**
- C'est le point d'extension prévu par Typst (`FileId` porte la racine).
- Le message répond à FR-008 et à l'US2 (il cite les versions disponibles).

**Alternatives écartées.**
- *`PackageError::NotFound`* : son message Typst ne permet pas d'indiquer les versions
  disponibles ni que les téléchargements sont impossibles.

## R6 — Cache des sources de paquets

**Décision.**
- Les `Source` des fichiers `.typ` de paquets sont mises en cache **globalement**, dans un
  `OnceLock<Mutex<HashMap<FileId, Source>>>` du module `packages`. Les sources de templates
  restent en cache par rendu, comme en V1.
- Les `Bytes` des paquets sont créés une seule fois par fichier, à partir du `&'static [u8]`
  sans copie, via `Bytes::new` sur la slice statique.

**Rationale.**
- Les `FileId` de paquets sont stables entre rendus (interning global).
- Réutiliser la même `Source` permet à `comemo` de réutiliser l'évaluation des modules de
  paquets d'un rendu à l'autre. C'est le principal levier de performance (SC-005, SC-006).
- La création des plugins WASM est elle aussi mémoïsée par Typst (`#[comemo::memoize]` dans
  `foundations/plugin.rs`). Le `comemo::evict(10)` de la V1 conserve les entrées utilisées
  récemment.

**Alternatives écartées.**
- *Cache par rendu* : chaque rendu réévaluerait cetz et lilaq. Le coût serait mesurable.

## R7 — Diagnostics dans un paquet

**Décision.**
- `SandboxWorld::relative_path` renvoie, pour `VirtualRoot::Package(spec)`, la chaîne
  `@preview/<nom>:<version>/<chemin>` (par exemple `@preview/cetz:0.5.2/src/draw.typ`).
- La ligne est calculée comme pour les fichiers du template, la source étant obtenue par le
  même `World`.

**Rationale.** Couvre FR-011. Le préfixe `@…` ne peut pas être confondu avec un chemin du
template.

## R8 — Détection des imports au chargement d'un template

**Décision.** Le nouveau module `src/template/imports.rs` procède ainsi pour chaque fichier
`.typ` de l'instantané du template :

1. `typst::syntax::parse(text)` ;
2. parcours récursif des `SyntaxNode` ;
3. pour chaque `ast::ModuleImport` ou `ast::ModuleInclude`, lecture de `.source()` ;
4. si c'est un `ast::Expr::Str` commençant par `@`, `PackageSpec::from_str`.

Les erreurs possibles sont les suivantes :

- spec invalide (version manquante ou partielle) ;
- namespace différent de `preview` ;
- spec absente de la table.

Elles sont rassemblées, et le template passe à l'état
`TemplateStatus::Invalid { reason }`, avec une raison qui liste chaque import fautif sous la
forme `fichier:ligne`. Les imports calculés dynamiquement restent couverts par le contrôle au
rendu (R5).

**Rationale.**
- L'auteur voit l'erreur dès le dépôt, sans générer (US2, FR-009).
- Le parseur Typst est exact sur la syntaxe, sans les faux positifs d'une expression régulière.

**Alternatives écartées.**
- *Liste de dépendances déclarée dans `template.json`* : ce serait une seconde source de vérité
  qui divergerait.
- *Contrôle au rendu seulement* : l'erreur serait tardive.

**Note.** Le code mort (une branche jamais exécutée) qui importe un paquet absent rend aussi le
template invalide. C'est un choix assumé, plus strict et documenté.

## R9 — Namespaces

**Décision.** Seul `@preview` est servi. `@local/…` et tout autre namespace sont traités comme
« non intégré », avec le même message.

**Rationale.** Périmètre de la spec ; aucun stock local n'est prévu.

## R10 — Route `GET /packages`

**Décision.**
- La route renvoie `{ "packages": [PackageInfo] }`, trié par nom puis par version.
- `PackageInfo` contient `namespace`, `name`, `version`, `import` (chaîne prête à copier),
  `description`, `license`, `role`.
- Aucun paramètre ; la réponse est constante pour un binaire donné.
- La route est décrite dans l'OpenAPI ; l'instantané `openapi/openapi.json` est régénéré.

Contrat : [contracts/api.md](./contracts/api.md).

**Rationale.** Couvre FR-013 et l'US3. La route est simple et cachable.

## R11 — Vérifications automatiques (FR-007) et documentation (FR-017)

**Décision.** `tests/bundled_packages.rs` contient quatre tests :

1. **Import** : pour **chacun** des 29 paquets, un document minimal
   `#import "@preview/<n>:<v>"` compile en PDF dans le `SandboxWorld`.
2. **Usage** : pour chacun des **21 paquets sélectionnés**, `tests/fixtures/package-smoke/<n>.typ`
   appelle une fonction représentative (un QR code, un `zero.num`, un graphique…) et le PDF est
   produit.
3. **Fermeture** : tous les imports littéraux `@preview/…` des `.typ` de chaque paquet (via R8)
   sont présents dans la table. On exclut les fichiers sous `tests/`, `docs/`, `examples/`,
   `gallery/` et `template/` du paquet, qui ne sont pas atteignables depuis l'entrypoint.
4. **Documentation** : `docs/packages.md` mentionne chaque `@preview/<n>:<v>` du lock, et
   uniquement ceux-là.

Ces tests s'exécutent avec `cargo test`, donc dans le job `test` de la CI, sans changement.

**Politique d'échec.** Si un paquet sélectionné échoue au test d'import ou d'usage avec Typst
0.15.1 :

- il est **retiré** du lock (archive supprimée) ;
- ses dépendances devenues orphelines sont retirées aussi ;
- l'utilisateur est prévenu ;
- aucun correctif local n'est appliqué (spec, Assumptions).

## R12 — Durée des plugins WASM

**Décision.**
- Pas de nouveau mécanisme. La dérogation V1 (principe IV) couvre le cas : réponse 504 au-delà
  de `render_timeout`, créneau de rendu conservé par `RenderGuard` jusqu'à la fin réelle,
  événement `render.overrun`.
- Les paquets avec plugin WASM (10 sur 29) sont identifiés dans `docs/packages.md`.

**Rationale.** `wasmi` n'expose pas de limite d'exécution dans Typst 0.15.1. Ajouter un
mécanisme demanderait de forker Typst, ce qui est hors périmètre (research/architecture.md B6).

## R13 — Ajout ou mise à jour d'un paquet

**Décision.** Le script `scripts/add-package.sh <nom> <version>` :

1. télécharge l'archive dans `packages/vendor/` ;
2. calcule le sha256 ;
3. lit la licence dans `typst.toml` ;
4. ajoute l'entrée au lock, avec `role = "selected"` par défaut ou `--dependency`.

Les dépendances manquantes sont signalées par le test de fermeture (R11). L'accès réseau n'est
nécessaire qu'à ce moment, sur le poste du mainteneur.

**Règles d'évolution.** Elles sont détaillées dans
[contracts/lock-file.md](./contracts/lock-file.md) :

- une mise à jour **ajoute** une version ;
- le retrait d'une version est une rupture, à signaler dans le changelog.

## R14 — Performance

**Décision.**
- `tests/perf.rs` (ignoré par défaut, exécuté par le job `perf`) mesure deux choses :
  - le p95 du template de démonstration existant, qui ne doit pas régresser de plus de 5 %
    par rapport à la V1 ;
  - le p95 de `examples/templates/packages-demo`, qui doit rester sous 1 s (SC-006).
- Le démarrage n'est pas affecté (table statique).

**Rationale.** Les mesures se font dans les conditions de la V1. Le cache global des sources
(R6) est la principale protection.
