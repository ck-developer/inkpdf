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
| `role` | `"selected"` (mis à disposition, l'un des 20) ou `"dependency"` (ajouté pour fermer l'ensemble) |

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

## R5 — Résolution des paquets dans `SandboxWorld`

**Décision.** `lookup(id)` branche sur `id.root()` :

- `VirtualRoot::Project` : comportement inchangé (instantané du template).
- `VirtualRoot::Package(spec)` : correspondance **exacte** dans la table statique (namespace,
  nom, version). On renvoie le fichier `vpath` du paquet, ou `FileError::NotFound`.

Une spec absente de la table renvoie `FileError::Package(PackageError::NotFound(spec))`. Ce cas
ne peut se produire que par un import interne de paquet non couvert, ce qu'exclut le test de
fermeture (R11), car les imports des templates ont déjà été résolus au chargement (R8, R15).

Il n'y a **aucune vérification supplémentaire au rendu** : la résolution d'un template se fait
une fois, à son chargement (FR-006).

L'isolation est **native** :

- la racine `Package(spec)` est étanche ;
- `..` au-delà de la racine est refusé par `VirtualPath` ;
- un paquet ne peut pas construire un chemin `Project` ;
- seuls une valeur `path`, des `bytes` ou une `image` passés explicitement par le template
  traversent (research/typst.md §1.6, FR-013).

Un dossier `packages/` présent dans un template reste un simple fichier de projet, jamais
utilisé pour résoudre un import.

**Rationale.** C'est le point d'extension prévu par Typst (`FileId` porte la racine). Le chemin
de rendu reste aussi simple qu'en V1.

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

## R8 — Résolution des imports au chargement d'un template

**Décision.** Le nouveau module `src/template/imports.rs`, appelé par le chargeur
(`registry/loader.rs`) **une fois par chargement** (dépôt, modification, rechargement à chaud),
procède ainsi pour chaque fichier `.typ` de l'instantané :

1. `typst::syntax::parse(text)` ;
2. parcours récursif des `SyntaxNode` ;
3. pour chaque `ast::ModuleImport` ou `ast::ModuleInclude`, lecture de `.source()` ;
4. si c'est un `ast::Expr::Str` dont la valeur commence par `@`, classement de l'import :

| Forme | Résultat |
|---|---|
| `@preview/<nom>` avec un paquet `selected` de ce nom | `Resolved` |
| `@preview/<nom>` sans paquet `selected` de ce nom | `Unavailable` |
| `@preview/<nom>:<…>` | `VersionWritten` |
| `@<autre>/…` | `OtherNamespace` |
| `@`, `@preview/`, `@preview/A` (nom invalide) | `Malformed` |

Ensuite :

- **Tous `Resolved`** : on réécrit le littéral dans le texte (`@preview/zero` devient
  `@preview/zero:0.7.1`) et l'instantané stocke le **texte réécrit** (R15).
- **Sinon** : `TemplateStatus::Invalid { reason }`, avec une ligne `fichier:ligne: message` par
  import fautif.

**Rationale.**
- Il n'y a qu'**un** contrôle, fait une seule fois, et rien à chaque génération (FR-006). C'est
  la demande de l'utilisateur : pas de double vérification au runtime.
- L'auteur voit l'erreur dès le dépôt (US2).
- Le parseur Typst est exact : pas de faux positifs dans les commentaires ou les chaînes
  ordinaires.

**Limites assumées.**
- Un import construit par calcul n'est pas pris en charge (Typst échoue au rendu avec
  « missing version »).
- Un import placé dans du code jamais exécuté est tout de même contrôlé.

## R9 — Namespaces

**Décision.** Seul `@preview` est servi. `@local/…` et tout autre namespace sont traités comme
« non intégré », avec le même message.

**Rationale.** Périmètre de la spec ; aucun stock local n'est prévu.

## R10 — Route `GET /packages`

**Décision.**
- La route renvoie `{ "packages": [PackageInfo] }`, avec **uniquement les paquets `selected`**,
  triés par nom.
- `PackageInfo` contient `name`, `import` (`@preview/name`), `version` (information),
  `description` et `license`.
- Aucun paramètre ; la réponse est constante pour un binaire donné.
- La route est décrite dans l'OpenAPI ; l'instantané `openapi/openapi.json` est régénéré.

Contrat : [contracts/api.md](./contracts/api.md).

**Rationale.** Couvre FR-014 et l'US3. Les dépendances internes ne sont pas importables, il est
donc inutile de les exposer.

## R11 — Vérifications automatiques (FR-007) et documentation (FR-017)

**Décision.** `tests/bundled_packages.rs` contient quatre tests :

1. **Import** : pour **chacun** des 28 paquets, un document minimal
   `#import "@preview/<n>:<v>"` compile en PDF dans le `SandboxWorld`.
2. **Usage** : pour chacun des **20 paquets mis à disposition**, `tests/fixtures/package-smoke/<n>.typ`
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
- Les paquets avec plugin WASM (6 des 20 paquets mis à disposition, 10 sur 28 au total) sont identifiés dans `docs/packages.md`.

**Rationale.** `wasmi` n'expose pas de limite d'exécution dans Typst 0.15.1. Ajouter un
mécanisme demanderait de forker Typst, ce qui est hors périmètre (research/architecture.md B6).

## R13 — Ajout ou changement de version d'un paquet

**Décision.** Le script `scripts/add-package.sh <nom> <version> [--dependency]` :

1. télécharge l'archive dans `packages/vendor/` ;
2. calcule le sha256 ;
3. lit la licence dans `typst.toml` ;
4. écrit l'entrée du lock. En mode `selected`, il **remplace** l'entrée `selected` existante de
   ce nom.

Les dépendances manquantes sont signalées par le test de fermeture (R11). L'accès réseau n'est
nécessaire qu'à ce moment, sur le poste du mainteneur.

**Règles d'évolution** (détaillées dans [contracts/lock-file.md](./contracts/lock-file.md)) :

- une seule version `selected` par nom ;
- changer de version la change pour tous les templates ;
- retirer un paquet est une rupture.

## R15 — Import par le nom seul : réécriture au chargement

**Décision.**
- Dans un template, un paquet s'importe **uniquement** par son nom (`@preview/zero`) ; écrire
  une version est refusé (R8).
- La version installée est **injectée une seule fois**, au chargement : le chargeur réécrit le
  littéral `@preview/<nom>` en `@preview/<nom>:<version selected>` dans le texte du fichier
  `.typ`, et l'instantané en mémoire contient ce texte réécrit.
- `SandboxWorld::source` reste inchangé : il construit les `Source` à partir de l'instantané,
  comme en V1.
- Les fichiers des paquets ne sont jamais réécrits ; leurs imports internes versionnés sont
  résolus exactement (R5).

**Rationale.**
- C'est la demande de l'utilisateur : une seule façon d'importer, aucune version à retenir, et
  aucun contrôle à chaque génération.
- Typst exige la version **à l'évaluation** (`typst-eval-0.15.1/src/import.rs` l. 213,
  `PackageSpec::from_str`), pas à l'analyse syntaxique. Fournir à Typst un texte déjà complété
  est donc la seule intervention nécessaire, et la faire au chargement la rend gratuite au rendu.
- Seul le contenu d'une chaîne change, sur sa propre ligne. Les numéros de ligne des
  diagnostics restent exacts.
- L'empreinte du template (`fingerprint`) reste celle des fichiers sur disque, et le rechargement
  à chaud est inchangé.

**Conséquences, documentées pour les auteurs.**
- Changer la version d'un paquet dans le service la change pour tous les templates.
- Un template inkpdf ne se compile pas tel quel avec l'outil Typst standard, qui exige une
  version.

**Alternatives écartées.**
- *Version facultative* : deux façons d'écrire, donc deux chemins de contrôle (refusé par
  l'utilisateur).
- *Réécriture à chaque rendu dans `World::source`* : ce serait un coût à chaque génération.
- *Un namespace maison* (`@inkpdf/zero`) : Typst exige la version pour tout namespace.

## R14 — Performance

**Décision.**
- `tests/perf.rs` (ignoré par défaut, exécuté par le job `perf`) mesure deux choses :
  - le p95 du template de démonstration existant, qui ne doit pas régresser de plus de 5 %
    par rapport à la V1 ;
  - le p95 de `examples/templates/packages-demo`, qui doit rester sous 1 s (SC-006).
- Le démarrage n'est pas affecté (table statique).

**Rationale.** Les mesures se font dans les conditions de la V1. Le cache global des sources
(R6) est la principale protection.

---

# Extension du périmètre (2026-10-10) : téléchargement, métadonnées, `layout`, exemples

## R16 — Renommage `design` → `layout`

**Décision.** Renommage complet, sans alias ni période de compatibilité :
- corps de requête `{ data, layout, metadata? }` ;
- clé racine autorisée dans `schema.json` (`data` et `layout` seulement) ;
- entrée Typst `sys.inputs.layout` (dictionnaire vide si le schéma ne déclare pas `layout`) ;
- schéma OpenAPI `RenderRequest` ;
- documentation et exemples.

Un corps contenant `design` est refusé par `additionalProperties: false` (422, chemin
`/design`). La constitution (principe III) est amendée en version 1.0.2 (PATCH) : le concept
reste le même, seul le nom change. Les documents de la feature 001 restent historiques.

**Rationale.** La V1 n'a aucun appelant externe. Garder un alias créerait deux façons d'écrire,
exactement ce que l'utilisateur a refusé pour les imports.

## R17 — Métadonnées du document

**Décision.**
- `metadata` est une section **facultative** du corps, retirée avant la validation par le
  schéma du template et validée par un schéma **fixe** propre au service :

  | Champ | Type et contraintes |
  |---|---|
  | `title` | 1 à 500 caractères |
  | `author` | chaîne, ou tableau de 1 à 20 chaînes de 1 à 200 caractères |
  | `subject` | ≤ 2 000 caractères |
  | `keywords` | tableau de ≤ 50 chaînes de 1 à 100 caractères |
  | `date` | chaîne `AAAA-MM-JJ` |

  Les propriétés inconnues sont refusées. Les violations ont des chemins `/metadata/...` et
  s'ajoutent à celles de `data` et `layout` (une seule réponse 422).
- Les métadonnées sont appliquées **après compilation**, sur `PagedDocument::info_mut()`
  (`typst-layout-0.15.1/src/document.rs` l. 38), avant l'export PDF. Elles ne sont donc jamais
  injectées comme code Typst. Ordre de priorité :
  - **titre** : requête, puis `set document(title:)` du template, puis `name` du template ;
  - **auteurs** : requête, puis template, puis `INKPDF_DEFAULT_AUTHOR` (défaut `inkpdf`) ;
  - **sujet** (`description` dans Typst), **mots-clés** et **date** : requête, sinon ceux du
    template, sinon rien (pas de date : déterminisme).
- Une date fournie par l'appelant reste déterministe, puisque c'est une donnée d'entrée.
  L'`ident` du PDF reste `{id}@{fingerprint}`.

**Alternatives écartées.**
- *Une source Typst enveloppe avec `set document(...)`* : les règles `document` sont limitées
  au premier niveau, et ce serait de la génération de code.
- *Laisser chaque template lire `sys.inputs.metadata`* : chaque auteur devrait le faire.

## R18 — Téléchargement

**Décision.**
- Deux paramètres de requête facultatifs sur `POST /templates/{templateId}/render` :
  - `download` (`true` ou `false`, `false` par défaut) ;
  - `filename` (chaîne, 200 caractères au maximum).
- Réponse :
  - `Content-Disposition: attachment` si `download=true`, `inline` sinon ;
  - `filename="<ascii>"` (non-ASCII remplacé par `_`) suivi de `filename*=UTF-8''<encodé>`
    (RFC 6266 / 5987).
- Nettoyage du nom :
  1. retrait des caractères de contrôle et de `/ \ " : * ? < > |` ;
  2. espaces et points superflus retirés en bordure ;
  3. un `.pdf` final est retiré puis rajouté ;
  4. si le résultat est vide, `<templateId>`.
- Une valeur de `download` invalide donne **400**, avec le nouveau code `invalid-parameter`.

**Rationale.** C'est la même route et le même rendu, donc pas de seconde génération à prévoir
côté serveur. Le cache est explicitement hors périmètre : un client qui veut un aperçu puis un
téléchargement réutilise le PDF déjà reçu.

## R19 — Exemples et Bruno

**Décision.**
- **Facture de situation de travaux** : `examples/templates/progress-invoice/`.
  - Montants en chaînes décimales, calculés en Typst avec `decimal` et arrondis au centime.
  - En-tête répété : logo SVG de démonstration et coordonnées de l'entreprise. Pied de page :
    mentions légales et « page X / Y ».
  - Tableau des postes par lot, avec en-tête répété.
  - Avenants, récapitulatif, conditions de paiement et QR SEPA (`sepay`) facultatif.
  - Montant en lettres (`frogst`), dates en français (`datify`), montants avec `zero`.
  - Les variantes sont pilotées par `data` (conditions du marché) et par `layout` (couleurs,
    logo, densité, blocs).
- **Requêtes d'exemple** dans `examples/requests/progress-invoice/*.json`, une par famille de
  conditions, dont une longue (au moins 3 pages).
- **Exemple de `layout` soigné** : couleurs, police, position et affichage du logo, densité,
  affichage des colonnes d'avancement, du QR et du montant en lettres, tous avec des valeurs par
  défaut. `sample` passe aussi à `layout`.
- **Collection Bruno** `examples/bruno/` :
  - `bruno.json` et `environments/local.bru` (`baseUrl`) ;
  - une requête `.bru` par route et par cas utile (rendu affiché, téléchargement, métadonnées,
    chaque variante de facture) ;
  - assertions simples sur le statut.
- **Tests** :
  - chaque requête d'exemple de la facture produit un PDF ;
  - un cas est vérifié au centime par un calcul indépendant fait dans le test ;
  - la variante longue fait au moins 3 pages ;
  - un test vérifie que chaque `.bru` cible une route de l'OpenAPI.
