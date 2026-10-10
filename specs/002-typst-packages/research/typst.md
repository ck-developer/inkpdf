# Recherche : Typst 0.15.1 pour inkpdf (paquets, PDF, templates)

Date : 2026-10-10. Version cible : `typst`, `typst-pdf`, `typst-layout`, `typst-kit` **=0.15.1** (`Cargo.toml`).

**Conventions**
- `$REG` désigne `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`. Les chemins `$REG/typst-*-0.15.1/...` sont le code source publié des crates : **c'est la référence**.
- **[V]** = vérifié dans le code source ou une source primaire citée.
- **[S]** = supposé, déduit, ou non testé par compilation (le binaire `typst` n'est pas installé sur la machine).

---

## 1. Système de paquets

### 1.1 `typst.toml` : champs du manifeste

Le parseur est `typst::syntax::package::PackageManifest` (`$REG/typst-syntax-0.15.1/src/package.rs`) **[V]**. Il est désérialisé par serde depuis TOML. Les champs inconnus sont tolérés et collectés dans `unknown_fields` (`#[serde(flatten)]`). Ils servent à la validation, mais l'import ne les rejette pas.

**`[package]`** (`PackageInfo`)

| Champ | Type Rust | Requis par le compilateur | Notes |
|---|---|---|---|
| `name` | `EcoString` | oui | comparé au `PackageSpec` par `validate` |
| `version` | `PackageVersion` | oui | exactement `MAJ.MIN.PATCH` en `u32` ; une 4e composante ou une composante vide est une erreur (`PackageVersion::from_str`) |
| `entrypoint` | `EcoString` | oui | chemin relatif au manifeste, résolu dans la racine du paquet |
| `authors` | `Vec` | non (oui pour Universe) | |
| `license` | `Option` | non (oui pour Universe : SPDX) | |
| `description` | `Option` | non (oui pour Universe) | |
| `homepage`, `repository` | `Option` | non | |
| `keywords` | `Vec` | non | |
| `categories` | `Vec` | non | 3 au plus (règle Universe) |
| `disciplines` | `Vec` | non | |
| `compiler` | `Option<VersionBound>` | non | borne **partielle** autorisée (`"0.15"`, `"0"`) ; vérifiée par `validate` avec `matches_ge` contre `PackageVersion::compiler()` |
| `exclude` | `Vec` (globs) | non | sémantique `.gitignore`, appliquée **à la publication** du tarball, pas à l'import |

**`[template]`** (`TemplateInfo`) : `path` (requis, dossier copié par `typst init`), `entrypoint` (requis, relatif à `path`), `thumbnail` (optionnel pour le compilateur, requis par Universe : PNG ou WebP sans perte, ≥ 1080 px, ≤ 3 MiB).

**`[tool]`** (`ToolInfo`) : `BTreeMap<EcoString, toml::Table>`, réservé aux outils tiers (`[tool.mon-outil]`). Une valeur non-table directement sous `[tool]` fait **échouer** le parsing (test `tool_section`).

Règles Universe de nommage et de versions **[V]** (https://github.com/typst/packages/blob/main/docs/manifest.md) :
- kebab-case, pas de « typst » dans le nom ;
- SemVer ;
- « The name and version in the folder name and manifest must match » ;
- `exclude` est appliqué à la publication.

**Validation intégrée** **[V]** : `PackageManifest::validate(&self, spec: &PackageSpec) -> Result<(), EcoString>` vérifie trois choses :
1. `name` égal au spec, sinon `package manifest contains mismatched name`… ;
2. `version` égale au spec ;
3. `compiler` ≤ version courante, sinon `package requires Typst {required} or newer (current version is {current})`.

### 1.2 Namespaces, arborescence, résolution CLI

- `@preview` est le seul namespace téléchargeable, depuis Universe ou un miroir. La constante est `UniversePackages::NAMESPACE` (`$REG/typst-kit-0.15.1/src/packages.rs`, `SystemPackages::obtain`) **[V]**.
- `@local` n'est **pas réservé** : « a good namespace for system-local packages is `local` », et « You can create an arbitrary `{namespace}` » (https://github.com/typst/packages/blob/main/README.md) **[V]**.
- Le namespace et le nom doivent être des identifiants Typst (`is_ident`). Les tirets sont autorisés (`parse_namespace`, `parse_name` dans `package.rs`) **[V]**.
- Arborescence : `{racine}/{namespace}/{name}/{version}/typst.toml` (`FsPackages::obtain` : `format!("{}/{}/{}", namespace, name, version)` puis `dir.exists()`) **[V]**.
- Ordre de résolution de la CLI (`SystemPackages::obtain`) **[V]** :
  1. dossier *data* (`$XDG_DATA_HOME/typst/packages`, `~/Library/Application Support/typst/packages`, `%APPDATA%/typst/packages`) ;
  2. dossier *cache* (`~/.cache/typst/packages`, `~/Library/Caches/typst/packages`…) ;
  3. téléchargement, uniquement si `namespace == "preview"`. L'archive est stockée dans le cache.
  4. Sinon : `PackageError::NotFound(spec)`.
- Ces dossiers se surchargent par `--package-path` / `TYPST_PACKAGE_PATH` et `--package-cache-path` / `TYPST_PACKAGE_CACHE_PATH` (https://man.archlinux.org/man/typst-compile.1.en) **[V]**.

### 1.3 Règle de version

- **Exacte et complète à l'import.** `PackageSpec::from_str` exige `@ns/name:version`, sinon l'erreur est `package specification is missing version`. `PackageVersion` exige 3 composantes. `"@preview/cetz:0.3"` échoue donc **au parsing** (`version number is missing patch version`), avant tout appel au `World` (`package.rs`, `$REG/typst-eval-0.15.1/src/import.rs` `import()`) **[V]**.
- « You must always specify the full package version » (README typst/packages) **[V]**.
- Les versions partielles ou absentes n'existent que pour `typst init` (`VersionlessPackageSpec` + `latest_version`, `packages.rs`) **[V]**.
- Aucun fallback de version : `World::file` reçoit exactement le `PackageSpec` demandé.

### 1.4 Lockfile et dépendances transitives

- Il n'existe **ni lockfile, ni déclaration de dépendances** dans `typst.toml` : aucun champ `dependencies` dans `PackageInfo` **[V]**.
- Une dépendance transitive est simplement un `#import "@preview/x:1.2.3"` dans le code du paquet. Elle n'est découverte qu'à l'évaluation **[V]** (import.rs).
- Deux versions d'un même paquet coexistent naturellement : ce sont deux `PackageSpec`, donc deux `VirtualRoot` distincts. Exemple réel : fletcher 0.5.8 importe cetz 0.3.4 (voir §5).
- La reproductibilité repose donc entièrement sur le fait que chaque import nomme une version exacte.

### 1.5 `FileId`, `PackageSpec` et ce que reçoit le `World`

API 0.15 (`$REG/typst-syntax-0.15.1/src/path.rs`) **[V]** :
- `RootedPath { root: VirtualRoot, vpath: VirtualPath }` ;
- `enum VirtualRoot { Project, Package(PackageSpec) }` ;
- `FileId` est l'interning global de `RootedPath` (`NonZeroU16`). On l'obtient par `RootedPath::intern()` ou `FileId::new`, et `id.root()` / `id.vpath()` y accèdent par `Deref`.
- `RootedPath::package()` est **déprécié** au profit de `root()`.

`VirtualPath` est toujours absolu et normalisé. Ses segments ne sont jamais vides, `.` ou `..`. `join("..")` depuis `/` donne `PathError::Escapes`, et un backslash donne `PathError::Backslash` (nouveau en 0.15).

Séquence d'un `#import "@ns/nom:1.2.3"` (`import.rs::resolve_package`) **[V]** :
1. `World::file(FileId{ Package(spec), "/typst.toml" })`, puis décodage UTF-8. En cas d'erreur, le message est `package manifest is malformed (…)`.
2. `toml::from_str::<PackageManifest>` puis `manifest.validate(&spec)`.
3. L'entrypoint est résolu par `PathOrStr::Str(entrypoint).resolve(manifest_id)`, donc dans la **même racine** `Package(spec)`.
4. `World::source(FileId{ Package(spec), "/<entrypoint>" })`.
5. Le module est nommé `manifest.package.name`.

`World::source` (fichiers `.typ`) et `World::file` (manifeste, images, données, wasm) reçoivent donc des `FileId` de racine `Package(spec)` **[V]**.

L'erreur prévue pour un paquet absent est `FileError::Package(PackageError::NotFound(spec))`. Elle s'affiche « package not found (searched for @ns/nom:1.2.3) » (`$REG/typst-library-0.15.1/src/diag.rs`) **[V]**. `VersionNotFound(spec, latest)` n'affiche que la version et « latest is … », pas le spec complet.

`#include "@ns/nom:v"` passe par la même fonction `import()` que `#import` (`ModuleInclude::eval`) **[V]**.

### 1.6 Isolation (racines) et type `path` (nouveau en 0.15)

- Chaque paquet a sa racine. **Dans un paquet, un chemin absolu `/x` désigne la racine du paquet.** La résolution utilise `within.root()` du fichier appelant (`PathOrStr::resolve`, `$REG/typst-library-0.15.1/src/foundations/path.rs`) **[V]**.
- `..` au-delà de la racine donne l'erreur « path … would escape the package root », avec le hint « cannot access files outside of the package sandbox » **[V]**.
- Un paquet ne peut donc **pas construire seul** un chemin vers le projet ou vers un autre paquet. Sa seule voie est un `#import "@…"`, qui ouvre la racine du paquet importé.
- **Type `path` (0.15)** : `#ty(name = "path") type RootedPath`. Il est accepté partout où une chaîne-chemin l'était : `image`, `read`, `json`, `import`, `include`, `pdf.attach`, `plugin`… (changelog 0.15 : « Paths can be passed across package boundaries », https://typst.app/docs/changelog/0.15.0/) **[V]**.
- Exemple de la doc **[V]** :

  ```typst
  #import "@local/my-pkg:0.1.0": process
  #let data-path = path("data.json")
  #process(data-path)
  ```

  Le paquet lit ensuite `data.json` **du projet**. `path()` se résout au site d'appel : un `path` construit dans `main.typ` reste en racine `Project`, même s'il est consommé dans un paquet.
- **Conséquence pour FR-005** **[V par lecture du code]** : le `World` reçoit un `FileId{Project, "/assets/logo.png"}` **sans aucune information sur l'appelant**. Une lecture du projet initiée par le paquet, via un `path` transmis, est indiscernable d'une lecture faite par `main.typ`.
  - L'option (a) « le template transmet l'image ou le contenu » est le comportement **natif**. Typst la documente aussi sous la forme « passez `image("logo.svg")` ou le résultat de `read` en argument » (doc de `path`).
  - L'option (c) « aucune exception » est **inapplicable au niveau du `World`**. Le paquet ne peut pas forger le `path`, mais si le template lui en transmet un, rien ne le distingue.
  - L'option (b) « le paquet lit librement les images du template » est **impossible sans** que le template lui passe un `path`.
- Seules les **lectures de fichiers** sont concernées. Les valeurs (`bytes`, `content`, dictionnaires, fonctions) passent librement entre racines.

### 1.7 Le stockage de `typst-kit` 0.15.1

Le crate a été « completely reworked » en 0.15 (changelog) **[V]**.
- `FsPackages(PathBuf)` et `SystemPackages` sont **liés au disque** : `obtain` fait `dir.exists()`, et `FsRoot::load` fait `fs::read` (`packages.rs`, `files.rs`) **[V]**. Les features `system-packages` / `universe-packages` tirent `dirs`, `flate2`, `tar`, et le téléchargement en plus avec `system-downloader`.
- inkpdf compile `typst-kit` **sans** ces features (`default-features = false, features = ["embedded-fonts"]`). Aucun de ces types n'est donc compilé, et c'est le bon choix.
- Le point d'extension prévu est le trait `FileLoader { fn load(&self, id: FileId) -> FileResult<Bytes>; }`, avec `FileStore<L: FileLoader>` (cache `FileId → Source/Bytes`, `source()`, `file()`, `dependencies()`). Ils sont disponibles **sans feature** (`files.rs`) **[V]**. La doc dit : « match on the root() of the id to check whether the file should be loaded from the project or a package ».
- **Résoudre depuis un instantané en mémoire ne demande rien de plus** que de brancher `VirtualRoot::Package(spec)` sur une map `(ns, name, version) → fichiers`. Aucun type de typst-kit n'est nécessaire. inkpdf a déjà son propre cache de `Source`, et `FileStore` n'apporterait que l'édition incrémentale des sources entre compilations, inutile ici.

### 1.8 Templates de paquets (`typst init`)

- Un paquet-template déclare `[template] path = "template"`, `entrypoint = "main.typ"`.
- `typst init @preview/nom[:version]` copie le dossier `template/` dans un nouveau projet. Le `main.typ` copié importe en général `@preview/nom:version` lui-même (README typst/packages, manifest.md) **[V]**.
- Pour inkpdf, un paquet-template embarqué n'est qu'un paquet comme un autre. La section `[template]` est ignorée par l'import (`resolve_package` ne lit que `[package]`) **[V]**.

---

## 2. typst-pdf 0.15.1

### 2.1 `PdfOptions` (`$REG/typst-pdf-0.15.1/src/lib.rs`) **[V]**

| Champ | Type | Défaut | Rôle |
|---|---|---|---|
| `ident` | `Smart<String>` | `Auto` | identifiant **stable** du document. Il est haché pour produire le `/ID` et le `xmpMM:DocumentID`, et n'est pas divulgué en clair. Avec `Auto`, c'est un hash du titre et des auteurs, ou à défaut l'instance ID. |
| `creator` | `Smart<Option<String>>` | `Auto` → `"Typst 0.15.1"` | `/Creator`. Il **change à chaque montée de version** de Typst. |
| `timestamp` | `Option<Timestamp>` | `None` | date de création, utilisée seulement si `document.date` vaut `auto` |
| `page_ranges` | `Option<PageRanges>` | `None` | pages exportées |
| `standards` | `PdfStandards` | PDF 1.7 | construit par `PdfStandards::new(&[PdfStandard])`, qui vérifie la compatibilité |
| `tagged` | `bool` | `true` | PDF balisé par défaut, depuis 0.14 |
| `pretty` | `bool` | `false` | sortie lisible : désactive la compression des flux. À garder à `false`. |

**`PdfStandard`** (`#[non_exhaustive]`) **[V]** :
- versions : `V_1_4`, `V_1_5`, `V_1_6`, `V_1_7`, `V_2_0` ;
- PDF/A : `A_1b`, `A_1a`, `A_2b`, `A_2u`, `A_2a`, `A_3b`, `A_3u`, `A_3a`, `A_4`, `A_4f`, `A_4e` ;
- PDF/UA : `Ua_1`. Il n'y a pas de PDF/UA-2.

Les noms serde sont `"1.7"`, `"a-3b"`, `"ua-1"`. Règles de combinaison : une seule version, un seul PDF/A, un seul PDF/UA. Une combinaison PDF/A + PDF/UA est possible si leurs plages de versions se recoupent (changelog 0.15 : « PDF/UA-1 and PDF/A-2a »). Une incompatibilité renvoie une erreur avec un hint de version.

### 2.2 Pièces jointes

- `pdf.attach(path, [data], relationship:, mime-type:, description:)` (`$REG/typst-library-0.15.1/src/pdf/attach.rs`) **[V]**.
  - `data` est en `bytes`. S'il est absent, le fichier est lu via `path`, qui accepte un `path` ou une `str`.
  - `relationship` vaut `"source"`, `"data"`, `"alternative"` ou `"supplement"`. Il est **ignoré hors PDF/A-3**.
  - `mime-type` est validé syntaxiquement depuis 0.14.
  - La compression est automatique (`attach.rs::should_compress`).
  - Attacher deux fois le même `path` est une erreur.
- Doc **[V]** : « File attachments are not currently supported for PDF/A-2 ».
- **`pdf.embed` est supprimé en 0.15.** C'était un alias déprécié depuis 0.14 (https://typst.app/docs/changelog/0.14.0/, /0.15.0/) **[V]**.
- Le tableau `/AF` du catalogue n'est écrit que si la version ou le validateur « specifies associated files », c'est-à-dire PDF/A-3 ou PDF 2.0 (`$REG/krilla-0.8.2/src/chunk_container.rs` l. 337, `serialize.rs` l. 133) **[V]**.
- Les autres fonctions du module `pdf` sont `pdf.artifact` et, derrière la feature `a11y-extras`, `pdf.table-summary`, `pdf.header-cell`, `pdf.data-cell` (`pdf/mod.rs`) **[V]**.

### 2.3 Métadonnées

- `set document(title:, author:, description:, keywords:, date:)`. 0.15 ajoute `path:` et `format:`, réservés à la cible *bundle* (`$REG/typst-library-0.15.1/src/model/document.rs`) **[V]**.
- La langue PDF vient de `text.lang`. Elle est toujours écrite, `Locale::DEFAULT` sinon (`metadata.rs`) **[V]**.
- `title` est requis en PDF/UA.
- **Aucune XMP personnalisée** : le builder `krilla::metadata::Metadata` n'expose que title, description, keywords, language, creator, producer, authors, creation_date, document_id, text_direction et page_layout (`$REG/krilla-0.8.2/src/interchange/metadata.rs`) **[V]**.

### 2.4 Liens, signets, destinations

- Les titres produisent l'outline (signets), avec leur numérotation depuis 0.14 (`$REG/typst-pdf-0.15.1/src/outline.rs`).
- Depuis 0.15, les titres **labellisés** génèrent des destinations nommées même sans référence (changelog 0.15) **[V]**.
- `link` gère les URL, les labels et les positions. Une URL vide est une erreur depuis 0.14.
- `heading(bookmarked: …)` contrôle l'outline **[S, doc de référence https://typst.app/docs/reference/model/heading/]**.

### 2.5 Déterminisme (chaîne vérifiée)

- `instance_id = stable_hash(octets du PDF sérialisé)` ;
- `document_id = stable_hash(version PDF, ident)` si `ident` est fourni ;
- `/ID = (document_id, instance_id)` (`krilla chunk_container.rs` l. 158-191) **[V]**.

Avec `timestamp: None` et sans `document(date:)`, **aucune date** n'est écrite (`metadata.rs::creation_date`) **[V]**. La date de modification des pièces jointes reprend la même source. La configuration actuelle d'inkpdf (`ident` stable, `timestamp: None`) est donc correcte.

Sources de non-déterminisme restantes :
- `datetime.today()` : `SandboxWorld::today` renvoie la vraie date (`src/render/world.rs`) ;
- le changement de version de Typst (Creator, mise en page) ;
- les polices.

### 2.6 Limites connues

- Pas de PDF/UA-2.
- Pas de pièces jointes en PDF/A-2.
- Pas de signature numérique, pas de formulaires AcroForm, pas de chiffrement **[S : absents du code typst-pdf et de `PdfOptions`]**.
- Pas d'XMP personnalisée.
- `page.bleed` (0.15) produit une TrimBox (`convert.rs` l. 118).

### 2.7 Factur-X / ZUGFeRD

Exigences Factur-X **[S, connaissance de la norme, non revérifiée en ligne ici]** :
- PDF/A-3 (b ou u) ;
- XML CII nommé `factur-x.xml` (ou `xrechnung.xml`), attaché avec `AFRelationship` `Data`, `Source` ou `Alternative` ;
- **XMP avec un schéma d'extension `fx:`** (`DocumentType`, `DocumentFileName`, `Version`, `ConformanceLevel`) et sa déclaration `pdfaExtension`.

Ce que fournit Typst 0.15.1 **[V]** :

```typst
#pdf.attach("factur-x.xml", xml-bytes, relationship: "alternative", mime-type: "text/xml")
```

avec `standards: [A_3b]`, ce qui donne une pièce jointe PDF/A-3 conforme avec `/AF`.

**Ce qui manque [V]** : il est impossible d'injecter l'extension XMP `fx:`. Factur-X n'est donc **pas atteignable en un seul passage**. Il faut un post-traitement du PDF : réécrire le flux `/Metadata`, avec `lopdf` par exemple. Ce post-traitement doit rester déterministe et préserver la conformité PDF/A-3. Le XML lui-même peut être produit en Typst, par exemple avec le paquet `xmlit` (§5), ou fourni dans `data`.

---

## 3. Autres sorties (aperçus)

- **PNG** : `typst-render` 0.15.1, publié le 2026-07-17 (https://crates.io/api/v1/crates/typst-render/versions) **[V]**. La signature est `pub fn render(page: &Page, opts: &RenderOptions) -> tiny_skia::Pixmap`, et il existe aussi `render_merged` (https://docs.rs/typst-render/0.15.1/typst_render/fn.render.html) **[V]**. **Rupture par rapport à ≤ 0.14**, où l'on passait `pixel_per_pt: f32`. Les champs de `RenderOptions` ne sont pas relevés **[S : densité, bleed]**. Dépendances : tiny-skia 0.12, resvg 0.47. Le crate n'est pas encore une dépendance d'inkpdf.
- **SVG** : `typst_svg::svg(&Page, &SvgOptions) -> String`, `svg_merged(&PagedDocument, &SvgOptions, gap)`. `SvgOptions { render_bleed, pretty, … }` (`$REG/typst-svg-0.15.1/src/lib.rs`) **[V]**. Le crate est déjà dans l'arbre, tiré par `typst-html`. Les classes CSS `typst-*` ont été supprimées en 0.15.
- **HTML** : `typst_html::html(&HtmlDocument, &HtmlOptions{pretty}) -> SourceResult<String>`, obtenu par `typst::compile::<HtmlDocument>` (`$REG/typst-html-0.15.1/src/encode.rs`, `dom.rs`) **[V]**. Il faut `Feature::Html` dans `Library::builder().with_features(...)` (`$REG/typst-library-0.15.1/src/lib.rs`). La doc dit « still very incomplete and only available for experimentation behind a feature flag » et ne produit pas de CSS (https://typst.app/docs/reference/html/) **[V]**. **Inadapté aux aperçus fidèles.**
- **Bundle** (0.15, expérimental, `Feature::Bundle`) : plusieurs sorties (PDF, HTML, PNG, SVG, assets) depuis un projet (changelog 0.15) **[V]**. Sans intérêt immédiat.
- Pour un aperçu, on peut rendre la page 1 en PNG avec `typst-render` sur le même `PagedDocument` que celui du PDF, sans recompiler **[S]**.

---

## 4. Langage : bonnes pratiques pour des templates paramétrés

Référence : https://typst.app/docs/reference/ (v0.15.1). Les points marqués [V] ont été vérifiés dans le code des crates.

### Données

- `sys.inputs` est un dictionnaire, fourni par `Library::builder().with_inputs` (`world.rs`).
- Accès sûr : `sys.inputs.data.at("champ", default: none)`. L'accès `.champ` échoue si la clé est absente.
- Les `float` JSON perdent l'exactitude. Pour les montants, utiliser `decimal("12.30")` depuis une chaîne, ou des centimes entiers (`int`). Les opérations sur `decimal` sont exactes (https://typst.app/docs/reference/foundations/decimal/).
- `json(source)` accepte un `path`, une `str` ou des `bytes` (`DataSource::{Path,Bytes}`, `loading/mod.rs`) **[V]**. `read(path, encoding: none)` renvoie des `bytes` (`loading/read.rs`) **[V]**. Un BOM UTF-8 dans un JSON est une erreur explicite (`json.rs`).
- `*.decode` est supprimé en 0.15 : utiliser `json(bytes(...))`.

### Styles

- `set` règle des propriétés.
- `show sel: it => …` transforme des éléments. `show: template.with(..)` est le schéma recommandé pour une fonction-template.
- Fonctions : arguments nommés avec défaut (`let f(x, size: 10pt) = …`), `..args` pour les variadiques, `f.with(..)` pour l'application partielle.

### Contexte et introspection

https://typst.app/docs/reference/context/, /introspection/

- `context { … }` est obligatoire pour lire l'état dépendant de la position : `counter(page).get()`, `here()`, `query`, `measure`, `text.lang`.
- `state("k", init)` avec `.update()` et `.get()` / `.final()`.
- `counter` avec `.step()`, `.display()`. 0.15 ajoute le paramètre `at` à `counter.display` (changelog 0.15) **[V]**.
- `query(selector)` est complété en 0.15 par le sélecteur `within` (changelog 0.15) **[V]**.
- « Page X/Y » :

  ```typst
  context [#counter(page).display() / #counter(page).final().first()]
  ```

  dans `page(footer: …)`.
- `measure(content)` et `layout(size => …)` servent à adapter la mise en page à la place disponible.

### Convergence

- Le layout est itératif, avec au plus **5 itérations** (`MAX_ITERS = 5`, `$REG/typst-library-0.15.1/src/introspection/convergence.rs`) **[V]**. 0.15 donne des diagnostics détaillés quand il ne converge pas.
- Causes typiques :
  - un `state` mis à jour à partir de sa propre valeur `final` ;
  - un contenu dont la taille dépend du nombre de pages (sommaire, « X/Y » qui change la pagination) ;
  - des boucles `measure` / `layout` imbriquées.
- Chaque itération recompile le layout complet : c'est un coût de performance direct.

### Tableaux

https://typst.app/docs/reference/model/table/

- `table(columns: (auto, 1fr, auto), table.header(..), ..lignes, table.footer(..))`.
- `table.header` et `table.footer` ont un champ `repeat: bool` (`$REG/typst-library-0.15.1/src/model/table.rs` l. 499, 529) **[V]**, qui répète l'en-tête à chaque page.
- Les en-têtes multiples et les sous-en-têtes existent depuis 0.14.
- `table.cell(breakable: auto|bool)` (l. 771) **[V]** et `block(breakable:)` (`layout/container.rs`) contrôlent la coupure.
- `grid` a la même API sans sémantique de tableau. Utiliser `table` pour l'accessibilité (balisage TH/TD).

### Page

- `set page(paper:, margin:, header:, footer:, header-ascent:, numbering:)`.
- 0.15 ajoute `page.bleed`.

### Images

- `image(source)` accepte un `path`, une `str` ou des `bytes`.
- Formats (`visualize/image/{mod,raster}.rs`) **[V]** :
  - raster : PNG, JPEG, GIF, WebP ;
  - pixels bruts : `format: (encoding: "rgb8"|"rgba8"|"luma8"|"lumaa8", width:, height:)` ;
  - vectoriel : SVG et **PDF** (depuis 0.14).
- Paramètres : `fit`, `scaling`, `icc`, `alt`.

### Polices

- `text(font: ("Inter", "Noto Sans"), fallback: true)`. Le fallback, actif par défaut (`text/mod.rs` l. 203), ne cherche **que dans le `FontBook` du World** : chez inkpdf, ce sont les polices embarquées et celles de `fonts/`.
- `font: ((name: "X", covers: "latin-in-cjk"), "Y")` restreint la couverture (0.13).
- Les polices variables sont supportées en 0.15 (`text.variations`). Une police de paquet n'est jamais chargée automatiquement, car le `FontBook` est fixé par le World.

### Dates

- `datetime(year:, month:, day:)` et `.display("[day]/[month]/[year]")`.
- Les noms de mois de `display` sont **en anglais uniquement** **[S, doc https://typst.app/docs/reference/foundations/datetime/]**. Pour le français, utiliser un tableau maison ou le paquet `datify` (§5).
- **Ne pas utiliser `datetime.today()`** : passer la date dans `data`. 0.15 accepte une `duration` comme `offset` de `today`.

### Nombres et monnaie

- **Aucun séparateur de milliers ni formatage localisé natif.** Le mot n'apparaît pas dans `str`, `int`, `float` ni `decimal` (`grep thousand` sur `foundations/`) **[V]**.
- Outils natifs : `calc.round(x, digits: 2)`, `str(x)`, `str(n, base:)`, `decimal`.
- Il manque : groupement, virgule décimale, zéros de fin forcés, symbole monétaire.
- On peut écrire une fonction maison sur `decimal` / `str`, ou utiliser `zero` (`num`, séparateurs configurables) ou `oxifmt` (`strfmt`, style Rust).

### Modules

- `#import "parts/x.typ": f, g`, `#import "x.typ" as x`, `#include "y.typ"`.
- Les chemins relatifs se résolvent au fichier appelant, les absolus `/…` à la racine.
- Depuis 0.13, un import dynamique sans `as` est une erreur.
- Le backslash est interdit dans les chemins (0.15).

### Plugins WASM

- `plugin(path|bytes)` exécute du wasm via **wasmi 1.0**, avec le SIMD relâché désactivé pour le déterminisme (`foundations/plugin.rs` l. 269-272) **[V]**.
- **Aucun compteur de « fuel » ni limite d'exécution** **[V]**. Un appel de plugin long est **non interruptible** par le drapeau `cancel` d'inkpdf, qui n'est consulté qu'aux accès au World.

### Erreurs courantes

- `.at()` sur une clé absente.
- Concaténer `str` et `int` sans `str()`.
- Oublier `context`, qui donne l'erreur « can only be used when context is known ».
- Appeler `counter.display` hors contexte.
- `type == "string"` : supprimé en 0.14.
- Utiliser `path(...)` comme forme géométrique : l'élément a été supprimé en 0.15 au profit de `curve`, et `path` est maintenant le type chemin.

### Ce qui casse le déterminisme

- `datetime.today()`.
- Un `PdfOptions.timestamp` non nul.
- Un `ident: Auto` avec un titre variable : c'est stable, mais ce n'est pas l'ident voulu.
- Des polices différentes d'une machine à l'autre : sans objet ici.
- Le changement de version de Typst ou de paquet.
- Des plugins impurs : interdits par contrat mais non vérifiés.

---

## 5. Paquets Typst Universe utiles

Sources :
- l'index https://packages.typst.org/preview/index.json (version, date, `compiler`, licence) ;
- les archives `https://packages.typst.org/preview/<nom>-<version>.tar.gz`, téléchargées et inspectées (taille, wasm, imports `@preview/`).

La compatibilité 0.15.1 est **[S] partout** : rien n'a été compilé, et `compiler` n'est qu'un minimum. Typst 0.15.0 est sorti le 2026-06-15 (crates.io).

| Paquet | Version (date) | `compiler` | tar / décompressé | WASM | Dépendances `@preview` | Licence |
|---|---|---|---|---|---|---|
| cetz | 0.5.2 (2026-05-07) | 0.14.0 | 219 Ko / 768 Ko | **oui**, 344 Ko | oxifmt:1.0.0 | LGPL-3.0+ |
| cetz-plot | 0.1.4 (2026-05-26) | 0.13.1 (0.14 via cetz) | 58 / 324 Ko | via cetz | cetz:0.5.2 | LGPL-3.0+ |
| lilaq | 0.6.0 (2026-03-14) | 0.13.0 (0.14 de fait) | 104 / 560 Ko | via komet, suiji | elembic:1.1.1, komet:0.1.0 **et** 0.2.0, suiji:0.5.1, tiptoe:0.4.0, zero:0.6.1 | MIT |
| fletcher | 0.5.8 (2025-05-27) | 0.13.0 | 51 / 212 Ko | via cetz | **cetz:0.3.4** (→ oxifmt:0.2.1) | MIT |
| tiaoma (codes-barres et QR, Zint) | 0.3.0 (2025-03-10) | — | 461 / 992 Ko | **oui**, 879 Ko | — | MIT |
| rustycure (QR) | 0.2.0 (2025-11-06) | — | 29 / 76 Ko | oui, 46 Ko | — | EUPL-1.2 |
| qrypst (QR) | 0.1.1 (2026-09-17) | 0.13.0 | 22 / 68 Ko | oui, 42 Ko | — | (index) |
| zebra (QR, DataMatrix) | 0.1.0 (2026-02-09) | — | 58 / 136 Ko | oui, 113 Ko | — | (index) |
| cades (QR) | 0.3.1 (2025-10-27) | — | 9 / 40 Ko | via jogs (**moteur JS en wasm**, 902 Ko) | jogs:0.2.4 | MIT |
| zebraw | 0.6.3 (2026-04-27) | 0.14.0 | 26 / 148 Ko | non | — | MIT |
| oxifmt | 1.0.0 (2025-06-04) | — | 20 / 84 Ko | non | — | MIT OR Apache-2.0 |
| zero (nombres) | 0.7.1 (2026-09-25) | 0.12.0 | 36 / 160 Ko | non | — | MIT |
| unify (unités) | 0.8.1 (2026-05-15) | — | 9 / 68 Ko | non | — | MIT |
| linguify (i18n, Fluent) | 0.5.0 (2025-12-19) | 0.11.0 | 88 / 228 Ko | oui, 195 Ko | — | MIT |
| datify (dates localisées) | 1.3.0 (2026-07-21) | 0.13.1 | 7 / 40 Ko | non | datify-core:2.1.0 (CLDR, **2,7 Mo** décompressé) | MIT |
| icu-datetime | 0.2.2 (2026-04-13) | 0.13.0 | **1,25 Mo / 4 Mo** | **oui**, ICU4X 4 Mo | — | MIT |
| tablex | 0.0.9 (2024-10-25) | — | 48 / 184 Ko | non | — | MIT OR Apache-2.0 |
| tablem / tabut | 0.3.0 / 1.0.2 | — | < 10 Ko | non | — | MIT |
| invoice-pro | 0.5.0 (2026-10-01) | 0.14.0 | 211 Ko / 1 Mo | via rustycure | letter-pro, loom, sepay → ibanator, rustycure | MIT |

Remarques :
- **tablex est obsolète** depuis `table` natif (0.11). Son README le dit.
- **zebraw est un paquet de blocs de code, pas de codes-barres.**
- Il n'existe **aucun paquet monnaie dédié** : seuls `a2c-nums` (montants en chinois) et `ea-nasir` (comptabilité) apparaissent dans l'index. Le formatage passe par `zero` ou par du code maison.
- Pistes à évaluer : `sepay` (QR EPC/SEPA), `rubrol-invoice` (EN 16931 / Factur-X, contenu non vérifié), `xmlit` (génération XML, `compiler = 0.15.0`).
- Surprise : cetz 0.5 embarque désormais un cœur wasm. Combiner fletcher et un cetz récent charge **deux** cetz et **deux** oxifmt.

---

## 6. Changements 0.13 → 0.15 qui nous concernent

Sources : https://typst.app/docs/changelog/0.13.0/, /0.14.0/, /0.15.0/.

### 0.13

- `image(path:)` est renommé `source:`.
- `image`, `json`, `csv`, etc. acceptent des `bytes` ; `*.decode` est déprécié.
- Le nom d'un import doit être statique (un import dynamique exige `as`).
- `counter.display` exige `context`.
- `pdf.embed` et PDF/A-3b apparaissent.
- Le fallback de police devient configurable par couverture (`covers`).
- `decimal` peut être construit depuis un flottant.
- HTML expérimental.

### 0.14

- PDF **balisé par défaut** : `PdfOptions.tagged`, défaut `true`.
- PDF/UA-1, tous les PDF/A, versions PDF 1.4 à 2.0.
- `pdf.embed` → `pdf.attach` (l'alias reste jusqu'à 0.15).
- Le MIME de `pdf.attach` est validé.
- PDF et WebP utilisables comme images.
- Plusieurs en-têtes de tableau.
- Suppression de la comparaison type/chaîne.
- `Library::default` supprimé (il faut `LibraryExt`).
- Correctif d'un panic de `World::font` à appliquer dans les World personnalisés.

### 0.15 (ruptures)

- **Le backslash est interdit dans les chemins.**
- `path` (forme) → `curve`.
- `pattern` → `tiling`.
- **`pdf.embed` supprimé.**
- `json.decode` et les autres `*.decode` supprimés.
- Ligne de base conservée dans plus d'endroits, avec des décalages de mise en page possibles.
- `typst-kit` refondu.
- API `FileId` avec `RootedPath` / `VirtualRoot` ; `package()` est déprécié.
- `typst-render` prend `&RenderOptions`.
- Rust ≥ 1.92.

### 0.15 (nouveautés)

- Type `path` qui traverse les paquets.
- Plusieurs standards PDF à la fois.
- `page.bleed`.
- Destinations nommées pour les titres labellisés.
- Sélecteur `within`.
- `counter.display(at:)`.
- Polices variables.
- Diagnostics de convergence.
- Cible *bundle*.
- `typst eval`.

### Impact sur les paquets embarqués

Un paquet ancien épinglé par un template (fletcher 0.5.8 → cetz 0.3.4, par exemple) peut utiliser `path(...)`, `pattern` ou `*.decode`, et échouer **à la compilation**. Le champ `compiler` (minimum seulement) ne le détecte pas. **`compiler ≤ 0.15.1` est nécessaire mais pas suffisant** : seul un rendu de test le prouve.

---

## 7. Recommandations pour inkpdf

### 7.1 Résolution dans `SandboxWorld`

- **Au chargement**, construire pour chaque template une `HashMap<PackageSpec, PackageFiles>`, où `PackageFiles` = `HashMap<String /*vpath sans slash*/, Bytes>`. On l'obtient en découpant l'instantané sur `packages/<ns>/<nom>/<version>/…`.
- **Dans `lookup`**, remplacer le refus actuel par :

  ```rust
  match id.root() {
      VirtualRoot::Project => self.entry.files.get(vpath),            // inchangé
      VirtualRoot::Package(spec) => match self.entry.packages.get(spec) {
          None => Err(FileError::Package(PackageError::NotFound(spec.clone()))),
          Some(pkg) => pkg.get(vpath).ok_or(FileError::NotFound(..)),
      },
  }
  ```

- Ne **pas** servir `packages/**` via la racine `Project`. Sinon `main.typ` pourrait faire `read("packages/...")`, ce qui est inoffensif, mais deux chemins mèneraient alors au même octet. On peut au choix exclure `packages/` de `entry.files`, ou l'accepter consciemment.
- Le message de FR-008 (« les paquets doivent être fournis avec le template ») ne fait pas partie de `PackageError::NotFound`. Deux options : `FileError::Other(Some("package @ns/n:v is not embedded in the template (packages/…); downloads are disabled"))`, ou un hint ajouté lors de la conversion du diagnostic.
- **Diagnostics** : `SandboxWorld::relative_path` renvoie `None` pour `VirtualRoot::Package`. Il faut le mapper vers `packages/<ns>/<nom>/<ver><vpath>`, sinon les erreurs dans un paquet perdent fichier et ligne (US2).
- **Isolation** : elle est **native**. Les racines `Package(spec)` sont étanches, `..` est refusé par `VirtualPath`, et l'instantané est déjà filtré des liens sortants.
  - Le seul « trou » est le `path` transmis volontairement par le template.
  - Recommandation pour la clarification de FR-005 : **option (a)** (le template transmet l'image, les bytes ou un `path`), documentée comme comportement Typst standard.
  - L'option (c) n'est pas implémentable au niveau du `World`, et l'option (b) n'a pas de sens dans le modèle de racines.

### 7.2 Validation au chargement (FR-007)

1. Arborescence `packages/<ns>/<nom>/<version>/` : `ns` et `nom` passent `typst::syntax::is_ident`, et `version` passe `PackageVersion::from_str`. On peut aussi parser `format!("@{ns}/{nom}:{ver}").parse::<PackageSpec>()`, qui fait les trois contrôles d'un coup.
2. `typst.toml` présent, en UTF-8, et passe `toml::from_str::<PackageManifest>` (`typst::syntax::package`). **Il faut ajouter `toml = "0.8"` aux dépendances d'inkpdf** : typst-syntax l'utilise, en 0.8.23 dans `Cargo.lock`, mais ne le réexporte pas.
3. `manifest.validate(&spec)`. Cela couvre le nom, la version et `compiler`, avec exactement les messages du compilateur, ce qui satisfait l'edge case « version minimale du moteur ».
4. L'entrypoint existe dans l'instantané du paquet, après normalisation par `VirtualPath::new("/").join(entrypoint)`.
5. Facultatif :
   - avertir sur `unknown_fields` ;
   - ignorer `exclude` (déjà appliqué par Universe) ;
   - ignorer `[template]` ;
   - ne pas charger les polices des paquets (le `FontBook` est fixe).
6. Fichiers hors arborescence (README à la racine de `packages/`, dossier sans version) : les ignorer et les journaliser, conformément aux edge cases.

### 7.3 Détection statique des imports (FR-009 c)

C'est **possible pour les littéraux uniquement**. Procédure (`$REG/typst-syntax-0.15.1/src/ast.rs` l. 2414-2600) **[V]** :
1. Pour chaque `.typ` (template **et** paquets embarqués, pour la fermeture transitive) : `typst::syntax::parse(text)` ou `Source::detached`.
2. Parcourir récursivement `SyntaxNode::children()`.
3. Pour chaque nœud `.cast::<ast::ModuleImport>()` **ou** `.cast::<ast::ModuleInclude>()`, lire `.source()`.
4. Si c'est `ast::Expr::Str(s)` et que `s.get()` commence par `@`, appliquer `PackageSpec::from_str`. Une erreur de parsing (version manquante, par exemple) peut être signalée dès le chargement.

Limites :
- un import calculé (`import ("@preview/" + nom)`, une variable, un `path`) est **indétectable** (`BareImportError::Dynamic`) ;
- le code mort (une branche `if` jamais prise) est détecté quand même, donc des faux positifs sont possibles.

La détection statique est donc un **contrôle précoce complémentaire**. Le contrôle au rendu (§7.1) reste obligatoire.

### 7.4 Risques à reporter dans le plan

- **FR-014 / WASM** : wasmi n'a pas de fuel. Un appel de plugin (cetz, tiaoma, icu-datetime…) ou une boucle Typst infinie n'atteint jamais le contrôle `cancel`, et `RenderGuard` garde le créneau jusqu'à la fin réelle du thread. Un paquet défectueux peut donc **consommer durablement un créneau**. Le problème existe déjà en V1, mais les paquets à plugin le rendent plus probable.
- **Taille** : icu-datetime (4 Mo) et datify-core (2,7 Mo) restent compatibles avec la limite de 50 Mo. Les plugins wasm sont recompilés par wasmi à chaque instanciation. Le cache comemo entre rendus atténue ce coût **[S]**.
- **SC-005** (< 1 s pour un graphique cetz) : à mesurer, car cetz 0.5 passe par un cœur wasm **[S]**.
- **Déterminisme** : il est garanti par les versions exactes et l'instantané. Il est rompu si l'auteur remplace le contenu d'un paquet sans changer de version. C'est accepté par le modèle, puisque le fingerprint du template change.
- Le paquet maison de helpers (US4) se ferait sous un namespace dédié (`@inkpdf/helpers:x.y.z`). Avec l'option (a), il serait embarqué dans l'image et servi par le même `match` sur `VirtualRoot::Package`, avec une priorité à fixer face aux paquets du template.
