# ADR 002 : la notion de paquet dans les templates inkpdf

- **Statut** : proposé (à valider avant `/speckit-plan` de la feature 002)
- **Date** : 2026-10-10
- **Entrées** : constitution 1.0.1, `specs/001-pdf-generation-service/`, `docs/templates.md`,
  brouillon `specs/002-typst-packages/spec.md`, recherche `research/typst.md` (Typst 0.15.1).
  `research/landscape.md` n'existe pas encore et n'a donc pas été pris en compte.
- **Direction fixée par l'utilisateur** : des briques simples qui évoluent dans le temps ;
  introduire la notion de paquet dans la définition des templates ; **aucun framework de design
  ni de composants pour l'instant**. Le framework de composants « headless » piloté par
  `design` est reporté.

**Décision en une phrase.** La feature 002 retient l'option 1 : les paquets sont copiés dans le
template sous `packages/<ns>/<nom>/<version>/` et résolus depuis l'instantané en mémoire. Elle y
ajoute l'option 4 réduite au strict minimum : le namespace `@inkpdf` est réservé, sans aucun
contenu. L'option 3 (stock partagé) est différée, mais le nom `_packages` reste disponible.
L'option 2 (template = paquet Typst) est écartée pour l'instant.

---

## 1. Contexte et forces

### 1.1 Ce que fait le code aujourd'hui (vérifié)

| Mécanisme | Où | Conséquence pour les paquets |
|---|---|---|
| Refus de tout paquet | `src/render/world.rs:85-89`, dans `SandboxWorld::lookup` : `VirtualRoot::Package(_)` donne `FileError::Other("package imports are not supported")` | C'est **l'unique** point de refus. Il suffit de le remplacer par une résolution. |
| Chemin des diagnostics | `world.rs:76-81`, `relative_path` renvoie `None` pour `VirtualRoot::Package` ; utilisé par `to_diagnostic` (`src/render/mod.rs:132-152`) | Une erreur levée dans un paquet perd aujourd'hui le fichier, la ligne et la colonne. |
| Point d'entrée fixe | `world.rs:23`, `MAIN_FILE = "main.typ"` ; vérifié par `loader.rs:144` | Le point d'entrée n'est pas configurable : c'est un enjeu pour l'option 2. |
| Instantané en mémoire | `src/registry/loader.rs:104-126` : tous les fichiers sont lus dans `TemplateEntry.files: HashMap<String, Bytes>`, puis l'empreinte est recalculée (`Unstable` si elle a changé) | Les fichiers de `packages/**` sont **déjà** chargés et cohérents. Il ne manque que leur routage. |
| Empreinte | `src/registry/fingerprint.rs:58-99` : parcours récursif sans filtre de chemin, fichiers cachés ignorés, liens sortants exclus, profondeur ≤ 32 | Rechargement à chaud, limite de taille (`INKPDF_MAX_TEMPLATE_BYTES`) et exclusion des liens symboliques couvrent `packages/**` **sans aucune ligne de code**. |
| Surveillance | `src/registry/watcher.rs` : `notify` récursif sur tout le volume, plus un rescan périodique ; `Registry::refresh` compare l'empreinte **par template** (`registry/mod.rs:112-166`) | Une modification hors d'un dossier de template déclenche un rescan, mais ne recharge aucun template (cf. option 3). |
| Identifiants | `src/template/id.rs` : `^[a-z0-9][a-z0-9_-]{0,63}$` ; les dossiers non conformes sont ignorés (`registry/mod.rs:211-219`, événement `template.ignored`) | Un dossier `_packages` à la racine du volume n'est **jamais** pris pour un template. Le nom est donc réservable sans rien changer. |
| Polices | `src/render/fonts.rs:65-70` : seuls les fichiers sous `fonts/` sont chargés | Les polices d'un paquet ne sont **pas** chargées, ce qui satisfait déjà le cas limite de la spec. |
| Manifeste | `src/template/manifest.rs` : `template.json` avec `deny_unknown_fields` (`name`, `description`, `version`) | Il n'y a pas de place pour des dépendances, et c'est voulu (cf. §4, FR-009). |
| Détail d'un template | `src/api/templates.rs:65-77`, `TemplateDetail` | L'ajout d'un champ `packages` est **additif**. |
| Contrat OpenAPI | `tests/contract_openapi.rs` compare `/openapi.json` à l'instantané `openapi/openapi.json` | « Verrouillé » veut dire **instantané vérifié par un test**, pas figé : un ajout se fait en régénérant l'instantané (`INKPDF_UPDATE_OPENAPI=1`) et en mettant la doc à jour **dans le même changement** (constitution, Workflow). |
| Test de bac à sable | `tests/sandbox.rs:22` `package_import_is_refused` | Ce test devra être **inversé** : un paquet non embarqué est refusé et le diagnostic nomme `@ns/nom:ver`. |
| `today()` | `world.rs:137-141` lit l'horloge réelle | C'est déjà une source de non-déterminisme. Les paquets la rendent plus discrète. |
| Identifiant du PDF | `render/mod.rs:111`, `ident = "{id}@{fingerprint}"` | Les paquets sont inclus dans l'empreinte, donc dans l'`ident`. Le déterminisme est conservé. |

### 1.2 Contraintes de la constitution

- **I (Typst embarqué)** : aucune CLI `typst` et aucun sous-processus. Les paquets sont résolus
  par le `World`, sans `typst-kit` sur disque (la recherche §1.7 confirme que `FsPackages` et
  `SystemPackages` lisent le disque et ne sont pas compilés).
- **II (template auto-suffisant)** : « toutes ses ressources sont résolues localement, à
  l'intérieur de son dossier (ou des ressources embarquées par le binaire) », « pas de paquets
  Typst téléchargés », « NE PEUT PAS référencer de fichiers hors de son propre dossier ».
  C'est **la** contrainte qui départage les options.
- **III (JSON `data` + `design`)** : elle n'est pas touchée par les paquets, mais c'est elle qui
  bornera plus tard un framework de composants piloté par `design`.
- **IV (bac à sable, non négociable)** : aucun accès disque hors du template et des ressources
  embarquées, et générations bornées en durée. Les plugins WASM échappent à cette borne
  (recherche §4, « Plugins WASM »).
- **V (templates à chaud, binaire figé)** : ajouter un paquet ne doit jamais exiger de
  reconstruire l'image. Tout cache doit rester cohérent avec le volume.
- **VI (API auto-descriptive)** : tout ajout au détail d'un template passe par l'OpenAPI.
- **VII (simplicité)** : la feature 002 est une spécification explicite, donc légitime. Toute
  source de vérité supplémentaire (lockfile, registre) doit justifier sa complexité.

### 1.3 Forces techniques de Typst 0.15.1 (recherche, toutes vérifiées [V])

- L'import exige une version exacte et complète (`@ns/nom:MAJ.MIN.PATCH`). Une version
  partielle échoue **au parsing**, avant tout appel au `World`. Typst ne fait aucun repli de
  version.
- Les dépendances sont implicites : il n'existe ni lockfile ni champ `dependencies`. Elles ne
  sont découvertes qu'à l'évaluation.
- L'isolation est native. Chaque paquet a sa racine `VirtualRoot::Package(spec)`, `..` ne peut
  pas sortir de cette racine, et un chemin absolu `/x` désigne la racine du paquet.
- Le type `path` (nouveau en 0.15) traverse les paquets. Un `path` construit dans `main.typ`
  reste en racine `Project` même s'il est lu par un paquet. Le `World` ne reçoit **aucune
  information sur l'appelant**.
- `PackageManifest::validate(&spec)` vérifie le nom, la version et la borne `compiler`, avec les
  messages exacts du compilateur.
- wasmi 1.0 exécute les plugins **sans fuel** : un appel de plugin ne peut pas être interrompu.

---

## 2. Options

Chaque option est évaluée selon le même canevas : arborescence, impact sur le code,
constitution, sécurité, rechargement à chaud et empreinte, expérience des auteurs,
réversibilité.

### Option 1 : paquets copiés dans chaque template (`packages/<ns>/<nom>/<version>/`)

```text
<volume>/
└── rapport/
    ├── main.typ                       # #import "@preview/cetz:0.5.2": canvas
    ├── schema.json
    ├── template.json
    ├── fonts/  assets/
    └── packages/
        ├── preview/
        │   ├── cetz/0.5.2/{typst.toml, lib.typ, cetz-core.wasm, …}
        │   └── oxifmt/1.0.0/{typst.toml, lib.typ}      # dépendance transitive de cetz
        └── equipe/
            └── helpers/0.1.0/{typst.toml, lib.typ}     # paquet maison, namespace libre
```

Cette arborescence est celle d'un dossier de paquets local de Typst (`{racine}/{ns}/{nom}/{ver}`,
`FsPackages::obtain`). Un auteur peut donc copier `~/Library/Application Support/typst/packages/`
ou le contenu d'une archive Universe sans rien transformer.

**Impact sur le code** (≈ 150 à 250 lignes avec les tests, réparties sur deux briques) :

- `src/registry/loader.rs` :
  - `TemplateEntry` gagne `packages: HashMap<PackageSpec, PackageFiles>`, où `PackageFiles` vaut
    `HashMap<String, Bytes>` avec des chemins relatifs à la racine du paquet ;
  - `validate()` découpe `files` sur le préfixe `packages/` ;
  - le nom de dossier est contrôlé par `format!("@{ns}/{nom}:{ver}").parse::<PackageSpec>()`,
    qui vérifie `is_ident` et `PackageVersion` d'un coup ;
  - **les fichiers de `packages/**` sont retirés de `files`** : un seul chemin mène à un octet,
    et `main.typ` ne peut ni `read("packages/…")` ni contourner le namespace réservé.
- `src/render/world.rs` :
  - `lookup` passe à `match id.root()` : `Project` reste inchangé ; `Package(spec)` cherche dans
    `entry.packages`, sinon renvoie un message qui nomme le spec et précise que le paquet doit
    être fourni dans `packages/<ns>/<nom>/<ver>/` et que tout téléchargement est désactivé. La
    variante d'erreur est à fixer au moment du plan : `FileError::Other(Some(m))` s'affiche
    « failed to load file (m) », alors que `FileError::Package(PackageError::Other(Some(m)))`
    s'affiche « failed to load package (m) », ce qui est plus juste (`typst-library`
    `diag.rs:658`, `:739`) ;
  - `relative_path` renvoie `packages/<ns>/<nom>/<ver>/<vpath>`.
- Validation des manifestes (FR-007) :
  - ajout de `toml = "0.8"` aux dépendances directes (déjà présent en 0.8.23 dans `Cargo.lock`
    via `typst-syntax`, mais non réexporté) ;
  - `toml::from_str::<PackageManifest>` puis `manifest.validate(&spec)`, et vérification que
    l'entrypoint existe dans `PackageFiles`.
- `src/api/templates.rs` : `TemplateDetail.packages: Vec<PackageRef { namespace, name, version }>`,
  plus l'instantané OpenAPI.
- Tests : inversion de `tests/sandbox.rs:22`, nouvelles fixtures (paquet minimal, paquet qui en
  importe un autre, version absente, manifeste invalide, tentative d'évasion depuis un paquet).
- Docs : `docs/templates.md` et `contracts/template-format.md`. La ligne « `#import
  "@preview/..."` interdit » du tableau du bac à sable devient « autorisé si embarqué ».

**Constitution** : compatible **sans amendement**. II est respecté à la lettre : tout reste
dans le dossier et rien n'est téléchargé, ce que II vise explicitement par « pas de paquets
Typst téléchargés ». Un amendement **PATCH** facultatif peut préciser dans II que
`packages/` fait partie des ressources du template. I, IV, V et VI sont compatibles. VII est
couvert par la spec 002.

**Sécurité** :
- l'isolation entre paquets et projet est assurée par Typst (racines distinctes) ;
- l'instantané est déjà filtré des liens symboliques sortants ;
- aucune lecture disque n'a lieu au rendu ;
- l'auteur ajoute du code tiers, mais ce code a exactement les droits d'un `.typ` du template.
  Le niveau de confiance ne change donc pas : le volume est déjà de confiance ;
- **seul angle nouveau** : les plugins WASM, qu'on ne peut pas interrompre (§5).

**Rechargement à chaud et empreinte** : il n'y a rien à faire. Le parcours est récursif, la
taille est comptée (FR-010), la stabilité est confirmée par la double observation et l'`ident`
du PDF change avec l'empreinte (FR-011, FR-013).

**Expérience des auteurs** :
- on copie un dossier et on importe exactement comme en local ;
- les messages d'erreur sont ceux de Typst, complétés par le chemin du paquet ;
- inconvénients : les paquets sont dupliqués entre templates et il faut gérer à la main les
  dépendances transitives (fletcher 0.5.8 → cetz 0.3.4 → oxifmt 0.2.1).

**Coût mémoire** : chaque template porte sa propre copie. Par exemple, 50 templates × cetz
(768 Ko décompressé) ≈ 38 Mo. C'est acceptable et c'est le levier qui justifiera l'option 3
plus tard.

**Réversibilité** : **élevée**. Le format est purement additif : un template sans `packages/`
est inchangé. Retirer la fonctionnalité reviendrait à rétablir le refus d'une ligne.

---

### Option 2 : le template est lui-même un paquet Typst (`typst.toml` à la racine)

```text
<volume>/
└── rapport/
    ├── typst.toml        # [package] name = "rapport", version = "1.0.0", entrypoint = "main.typ"
    │                     # [tool.inkpdf] schema = "schema.json"
    ├── main.typ
    ├── schema.json
    ├── (template.json ?) # redondant avec [package] → à supprimer ou à fusionner
    └── packages/…        # mêmes paquets copiés qu'en option 1
```

**Impact sur le code** (≈ 150 lignes en plus de l'option 1) :
- `src/template/manifest.rs` : nouveau parseur, `typst.toml` remplaçant ou complétant
  `template.json`, avec une règle de priorité à écrire ;
- `loader.rs:135-160` : entrypoint lu dans le manifeste au lieu de `"main.typ"` ;
- `world.rs:23,53-57` : `MAIN_FILE` devient `entry.entrypoint` ;
- `api/templates.rs` : `name` et `version` viennent de `typst.toml` ;
- documentation du format.

**Constitution** : compatible avec I, II et IV. En revanche, cette option **change le contrat
de format** (`template-format.md` : « toute rupture de ce format est une rupture MAJOR ») :
- MINOR si `typst.toml` reste facultatif avec repli sur `main.typ` et `template.json` ;
- MAJOR s'il devient obligatoire.

**Points de friction réels** :
- `[package].version` doit être strictement `MAJ.MIN.PATCH`, alors que `template.json.version`
  est libre (≤ 64 caractères) : des templates existants deviendraient invalides ;
- `[package].name` doit être un identifiant Typst, alors que `TemplateId` (le nom du dossier)
  est l'identifiant de l'API. On aurait deux noms à garder cohérents ;
- **`typst init` ne correspond pas** à ce modèle. Un paquet-template Universe déclare
  `[template] path = "template"` : c'est un sous-dossier copié, dont le `main.typ` importe
  `@preview/<lui-même>:<ver>`. Pour être réellement compatible, inkpdf devrait servir le
  template à la fois comme `Project` et comme `Package(son propre spec)`. C'est faisable, mais
  c'est une sémantique nouvelle sans besoin démontré ;
- **publier un template inkpdf sur Universe n'a guère de sens** : il dépend de
  `sys.inputs.data` et `sys.inputs.design` et d'un `schema.json` qu'Universe ignore.

**Sécurité, rechargement à chaud, empreinte** : identiques à l'option 1.

**Expérience des auteurs** : un seul manifeste, familier aux habitués de Typst. Mais deux
sources de nom et de version tant que `template.json` subsiste.

**Réversibilité** : **moyenne**. Une fois adoptée, la règle de priorité entre `typst.toml` et
`template.json` devient un contrat. À retenir pour plus tard : la table `[tool.inkpdf]` est
l'emplacement prévu par Typst pour des métadonnées d'outil (`ToolInfo`). Si l'option est un
jour adoptée, c'est là qu'on placera le pointeur vers le schéma, et `typst.toml` devra rester
**facultatif**.

---

### Option 3 : stock de paquets partagé, en lecture seule, dans le volume (`/templates/_packages`)

```text
<volume>/
├── _packages/                       # ignoré comme template (id invalide : commence par `_`)
│   └── preview/cetz/0.5.2/…
├── rapport/  main.typ  schema.json  (packages/ facultatif, prioritaire ?)
└── devis/    main.typ  schema.json
```

**Impact sur le code** (≈ 250 à 400 lignes) :
- un chargeur dédié pour `_packages` (instantané partagé `Arc`, validation identique à
  l'option 1) ;
- `TemplateEntry` référence l'instantané partagé, et `lookup` applique un ordre de résolution
  (template d'abord, puis stock) ;
- **l'empreinte devient composite**. Aujourd'hui `Registry::refresh` compare l'empreinte de
  chaque dossier de template. Une modification de `_packages` déclenche bien un rescan
  (surveillance récursive), mais ne recharge **aucun** template. Il faut une génération globale
  du stock incluse dans l'empreinte de chaque template (ou de ceux qui l'utilisent), et donc
  dans l'`ident` du PDF ;
- la limite de taille par template ne sait pas comment compter le stock ;
- le détail d'un template devrait distinguer l'origine des paquets (template ou stock).

**Constitution** : **incompatible en l'état**. Cette option viole II (« NE PEUT PAS référencer
de fichiers hors de son propre dossier ») et la lettre de IV (« aucun accès au système de
fichiers hors du dossier du template et des ressources embarquées »). Elle demande un
amendement **MINOR** de II et IV (« extension matérielle d'une règle ») qui ajoute le stock
partagé comme troisième source autorisée. On pourrait même soutenir que c'est MAJOR, puisque
l'auto-suffisance est redéfinie. La spec 002 la classe déjà hors périmètre.

**Sécurité** : le risque est faible, car le stock est dans le même volume de confiance.
Mais le rayon d'impact grandit : un paquet défectueux du stock casse **tous** les templates
qui l'importent, d'un coup.

**Rechargement à chaud** : c'est le point délicat. Un changement dans le stock doit
invalider, de manière atomique, tous les templates qui en dépendent. Il faut aussi éviter de
servir un mélange entre ancien stock et nouveau template.

**Expérience des auteurs** : excellente pour une équipe qui maintient beaucoup de templates
(une seule copie de cetz). Mais un template n'est plus déployable par simple copie de son
dossier.

**Réversibilité** : **faible** une fois adoptée. Les templates qui comptent sur le stock
cesseraient de fonctionner si on le retirait.

**Décision** : à **différer**. On réserve le nom maintenant, sans écrire de code :
`_packages` est déjà ignoré par le registre (id invalide), il suffit de le documenter comme
réservé. On réévaluera quand la duplication posera un problème mesuré (mémoire, maintenance).

---

### Option 4 : namespace réservé `@inkpdf/...` pour des paquets fournis par le service

```text
binaire inkpdf (include_bytes!) :
    @inkpdf/helpers:0.1.0  → lib.typ, typst.toml          (futur, hors 002)
volume :
    rapport/packages/inkpdf/**   → refusé (namespace réservé, template invalide)
```

**Impact sur le code** :
- **variante « réservation seule »** (≈ 30 lignes avec les tests) :
  - `loader.rs` marque le template invalide si `packages/inkpdf/` existe (raison : « namespace
    `inkpdf` is reserved ») ;
  - `world.rs` renvoie pour `@inkpdf/*` « no package `@inkpdf/x:v` is provided by this version
    of inkpdf » ;
- **variante « contenu »** (plus tard, ≈ 100 lignes plus le paquet) :
  - un module `src/render/builtin_packages.rs` avec `include_bytes!` ;
  - une branche `Package(spec) if spec.namespace == "inkpdf"` dans `lookup`.

**Constitution** : **compatible sans amendement**. II autorise déjà « les ressources
embarquées par le binaire, comme les polices par défaut ». Une tension avec V est à nommer :
le contenu de `@inkpdf/*` évolue **au rythme des images**, pas des templates. C'est cohérent
avec « binaire figé », mais un correctif de helper exige une release du service. Le versionnage
exact des paquets amortit le problème : une image peut embarquer `0.1.0` et `0.2.0` côte à côte.

**Sécurité** : le code est contrôlé par le projet. Il faut veiller au déterminisme : l'`ident`
du PDF ne contient que l'empreinte du template. Si un paquet embarqué changeait sans changer
de version, le PDF changerait à `ident` égal. La règle est donc « un paquet `@inkpdf` publié
est immuable », vérifiée par un test d'empreinte en CI.

**Rechargement à chaud** : sans objet, le contenu est figé dans le binaire.

**Expérience des auteurs** : un import sans copie (`#import "@inkpdf/helpers:0.1.0"`). C'est le
point d'ancrage naturel des futurs helpers et composants.

**Réversibilité** : la réservation seule est **totalement** réversible et ne coûte rien. Le
contenu, lui, devient un contrat public versionné dès sa première version.

---

### Synthèse

| Critère | 1. Copiés | 2. Template = paquet | 3. Stock partagé | 4. `@inkpdf` réservé |
|---|---|---|---|---|
| Amendement de la constitution | aucun (PATCH facultatif) | aucun, mais changement du format (MINOR ou MAJOR) | **MINOR de II et IV** | aucun |
| Code (ordre de grandeur) | 150 à 250 l. | +150 l. | 250 à 400 l. | 30 l. (réservation) |
| Rechargement à chaud et empreinte | gratuits | gratuits | empreinte composite à construire | sans objet |
| Isolation | native (Typst) | native | native, mais rayon d'impact partagé | native |
| Déploiement par simple copie | oui | oui | **non** | oui |
| Réversibilité | élevée | moyenne | faible | élevée (réservation) |
| Dans 002 | **oui** | non | non (réservation du nom seulement) | **réservation seulement** |

Compatibilité principe par principe :

| Principe | 1. Copiés | 2. Template = paquet | 3. Stock partagé | 4. `@inkpdf` |
|---|---|---|---|---|
| I. Typst embarqué | compatible | compatible | compatible | compatible |
| II. Template auto-suffisant | compatible (PATCH de clarification facultatif) | compatible | **amendement MINOR** (référence hors du dossier) | compatible (« ressources embarquées par le binaire ») |
| III. JSON `data` + `design` | sans objet | sans objet (le schéma reste à part, `[tool.inkpdf]`) | sans objet | sans objet en 002 ; cadre de B10 |
| IV. Bac à sable | compatible (risque WASM, §5) | compatible | **amendement MINOR** (lecture hors du template) | compatible |
| V. À chaud, binaire figé | compatible, sans code | compatible | compatible, à condition de construire une empreinte composite | compatible ; le contenu suit les releases de l'image |
| VI. API auto-descriptive | ajout additif à l'OpenAPI (B3) | changement du format documenté et de l'OpenAPI (`name`, `version`) | origine des paquets dans l'OpenAPI | sans changement d'API |
| VII. Simplicité | couvert par la spec 002 | complexité sans besoin démontré | spec dédiée exigée | réservation : coût quasi nul ; contenu : spec dédiée |

---

## 3. Recommandation : feuille de route en briques

**Combinaison retenue** :
- **option 1** comme socle ;
- **option 4 réduite à la réservation** du namespace, comme point d'ancrage ;
- **option 3 différée**, avec le nom `_packages` documenté comme réservé ;
- **option 2 écartée** tant qu'aucun besoin d'interopérabilité avec Universe n'est démontré.

Pourquoi :
- c'est la seule combinaison qui respecte la constitution **sans amendement** ;
- elle hérite gratuitement du rechargement à chaud, de l'empreinte, de la limite de taille et
  du bac à sable de la V1 ;
- chaque étape suivante (stock, helpers, composants) se branche au **même endroit** : le
  `match id.root()` de `SandboxWorld::lookup`. Cet endroit devient l'unique résolveur, avec un
  ordre d'ajout clair.

Règle commune à toutes les briques : chaque brique qui touche le format ou l'API livre **dans
le même changement** ses tests, sa fixture, la mise à jour de `docs/templates.md` et de
`contracts/template-format.md`, et, si besoin, l'instantané OpenAPI. Ce n'est pas une brique à
part : la constitution l'exige.

### Briques de la feature 002

| # | Brique | Périmètre précis | Test isolé | Débloque |
|---|---|---|---|---|
| **B1** | **Résolveur de paquets embarqués** | Découpe de l'instantané en `TemplateEntry.packages` (les noms de dossier non conformes sont ignorés et journalisés) ; retrait de `packages/**` de `files` ; `lookup` sur `VirtualRoot::Package` ; message « non embarqué, téléchargement désactivé » qui nomme `@ns/nom:ver` ; `relative_path` → `packages/<ns>/<nom>/<ver>/<fichier>` pour garder fichier et ligne ; inversion de `tests/sandbox.rs:22` ; fixtures « paquet simple », « paquet → paquet », « version absente », « évasion `..` depuis un paquet » ; template d'exemple avec un petit paquet pur Typst (sans WASM) ; docs. Taille : ≈ 120 à 180 lignes. | Un rendu avec paquet embarqué produit un PDF stable à l'octet ; un paquet manquant donne `render-failed` avec le spec dans le message ; une erreur dans un paquet donne `file: packages/...` et une ligne. | US1 entière ; FR-001 à 006, 008, 010, 011, 013 à 016 ; le diagnostic de paquet de US2. |
| **B2** | **Validation des paquets au chargement** | Ajout de `toml = "0.8"` ; pour chaque `packages/<ns>/<nom>/<ver>/` : `typst.toml` présent et en UTF-8, `PackageManifest` parsé, `validate(&spec)` (nom, version, `compiler` ≤ 0.15.1), entrypoint présent ; sinon le template est `invalid` avec une raison qui nomme le fichier (`packages/preview/cetz/0.5.2/typst.toml: …`). Les sections `[template]` et `exclude` sont ignorées. Les fichiers hors arborescence (README à la racine de `packages/`) sont ignorés. Taille : ≈ 80 lignes. | Fixtures avec manifeste absent, mal formé, nom ou version incohérents, `compiler = "0.16"`, entrypoint absent : chacune donne `invalid` avec la bonne raison ; la correction dans le volume rend le template valide sans redémarrage. | FR-007 ; US2 (scénarios 2 et 3). |
| **B3** | **Paquets visibles dans le détail** | `TemplateDetail.packages: [{namespace, name, version}]`, trié et toujours présent (vide si aucun paquet) ; ajout additif à l'OpenAPI, avec régénération de l'instantané. Option : `license` lu dans `typst.toml`, utile pour l'audit (§5). Taille : ≈ 50 lignes. | Test de l'API (zéro, un et deux paquets) ; `contract_openapi` vert après régénération. | US3 ; FR-012. |
| **B4** | **Namespace `@inkpdf` réservé** | `packages/inkpdf/**` dans un template le rend `invalid` ; un import de `@inkpdf/*` donne « not provided by this version of inkpdf » ; `_packages` est documenté comme nom réservé du volume (déjà ignoré par le registre). Aucun contenu livré. Taille : ≈ 30 lignes. | Une fixture `packages/inkpdf/x/0.1.0` est invalide ; un import de `@inkpdf/x:0.1.0` échoue avec le bon message. | Ancrage de US4 ; prépare B8 et B9 sans engager de contenu. |
| **B5** | **Détection statique des imports au chargement** | Parcours de l'AST (`ast::ModuleImport` et `ast::ModuleInclude`, source littérale `@…`) sur un **périmètre atteignable** : tous les `.typ` du template, puis, dans chaque paquet importé, l'entrypoint et les fichiers qu'il importe ou inclut littéralement, et ainsi de suite (fermeture transitive). Un spec mal formé (version absente) ou non embarqué **dans ce périmètre** rend le template `invalid` avec le fichier et la ligne. Les autres `.typ` d'un paquet (`docs/`, `tests/`, `examples/` d'une copie git, qui importent souvent `@preview/tidy` ou d'autres outils) ne produisent qu'un **avertissement** dans les journaux. Les imports calculés ne sont pas détectables : le contrôle au rendu de B1 reste la garantie. Taille : ≈ 120 lignes. | Fixtures : import littéral manquant → `invalid` ; import transitif manquant (paquet → paquet) → `invalid` ; import manquant dans `packages/…/tests/x.typ` non atteignable → valide plus un avertissement ; import calculé → valide au chargement, échec au rendu. | FR-009 (c) ; erreurs visibles dès `GET /templates` (US2, scénario 1). |

Ordre : B1 → B2 → B3, puis B4 et B5 dans n'importe quel ordre. B1 seule livre déjà la valeur
principale. B3, B4 et B5 sont indépendantes entre elles.

### Briques suivantes (hors 002, chacune avec sa propre spec)

| # | Brique | Débloque | Prérequis constitutionnel |
|---|---|---|---|
| B6 | **Politique WASM** : inventaire au chargement des `.wasm` et des appels `plugin(...)` littéraux ; option `INKPDF_WASM_PLUGINS=allow\|deny` (défaut `allow`, car cetz 0.5 en dépend) ; journalisation de `render.overrun` avec le paquet en cause. Une **vraie** limite d'exécution demanderait de forker `typst-library` (`plugin.rs:269-274` crée `wasmi::Engine` sans fuel) : hors de portée. | Maîtrise du risque principal de IV. | aucun (IV renforcé) |
| B7 | **`today()` maîtrisé** : option `INKPDF_TODAY=clock\|none` (`none` fait renvoyer `None` par `World::today`, donc une erreur Typst explicite) ; défaut `clock` pour ne pas casser la V1. | Garantir FR-017 même avec des paquets tiers. | aucun |
| B8 | **Stock partagé `_packages`** (option 3) : instantané partagé, empreinte composite, origine des paquets dans le détail. | Déduplication à grande échelle. | **Amendement MINOR de II et IV** |
| B9 | **Contenu `@inkpdf/helpers:0.1.0`** embarqué : petites fonctions pures (formatage de nombres sur `decimal`, dates en français, lecture sûre de `sys.inputs`), sans aucun composant visuel. Règle d'immuabilité testée en CI. | Fin de la duplication des helpers. | aucun (II, ressources embarquées) |
| B10 | **Composants « headless » pilotés par `design`** (façon React Email), sous `@inkpdf/…`. | Le framework reporté. | Spec dédiée (VII) ; s'appuie sur B4 et B9. |
| B11 | (éventuelle) **Template = paquet** (option 2) avec `typst.toml` facultatif et `[tool.inkpdf]`. | Interopérabilité avec Universe ou `typst init`, si un besoin apparaît. | Changement MINOR du format |

---

## 4. Réponses recommandées aux 3 questions ouvertes de la spec 002

### US4 : « paquet maison des helpers » → **(c) pour le contenu**, et réservation de `@inkpdf` dans 002

- Ce que (b) décrit marche **sans aucun code supplémentaire** dès B1. Une équipe peut copier son
  paquet `@equipe/helpers:0.1.0` dans `packages/` de chaque template, et FR-002 accepte tout
  namespace. Il suffit de le documenter comme usage recommandé.
- Livrer un paquet fourni par le service, l'option (a), engage un contrat public versionné et
  lié aux releases de l'image (tension avec V). Cela contredit aussi la consigne « pas de
  framework pour l'instant ». Ce contenu relève donc d'une feature dédiée (B9).
- Dans 002, on ne fait que **réserver** `@inkpdf` (B4). Le coût est quasi nul, et cela empêche
  qu'un template occupe aujourd'hui le namespace dont B9 et B10 auront besoin demain.

**Formulation proposée** : « US4 : hors périmètre. Un paquet de helpers d'équipe s'embarque
comme tout paquet (FR-001). Le namespace `inkpdf` est réservé au service (nouveau FR-017). »
En conséquence, FR-002 doit être reformulé : « tout namespace est accepté **sauf `inkpdf`**,
réservé au service ».

### FR-005 : un paquet peut-il afficher une image du template ? → **(a)**

- C'est le **comportement natif** de Typst 0.15. Le template passe au paquet l'image
  (`image("assets/logo.png")`), ses octets (`read(..., encoding: none)`) ou un `path`
  (`path("assets/logo.png")`). Un paquet ne peut pas **forger** un chemin vers le projet
  (racine propre, `..` refusé).
- (c) est **inapplicable** au niveau du `World`. Quand le template transmet un `path`, la
  lecture arrive sous la forme `FileId{Project, "/assets/logo.png"}`, sans aucune information
  sur l'appelant (recherche §1.6). Rien ne permet de la distinguer d'une lecture de `main.typ`.
- (b) n'a pas de sens dans le modèle de racines : sans `path` transmis, le paquet n'a aucun
  moyen de désigner un fichier du projet.

**Formulation proposée** : « Un paquet ne peut lire que ses propres fichiers. Il n'accède à un
fichier du template que si le template lui transmet explicitement le contenu ou un chemin
(`path`), ce qui est le comportement standard de Typst. »

### FR-009 : quand détecter un paquet manquant ? → **(a) obligatoire dans B1, (c) en complément dans B5 ; pas (b)**

- Le contrôle **au rendu** (a) est la seule garantie complète : un import calculé
  (`import ("@preview/" + nom)`) n'est connu qu'à l'évaluation.
- La détection **statique** (c) attrape le cas courant (imports littéraux du template et des
  fichiers de paquet atteignables depuis leur entrypoint) dès le chargement. L'erreur apparaît
  alors dans `GET /templates` avant la première génération. Dans ce périmètre atteignable, on
  marque le template `invalid` plutôt que de seulement journaliser. Hors de ce périmètre
  (fichiers de tests ou de documentation d'un paquet), on ne fait qu'avertir. Raisons :
  - un import littéral d'un paquet absent est presque toujours une erreur d'auteur ;
  - le faux positif possible (import dans une branche morte) se corrige trivialement ;
  - c'est cohérent avec FR-010 de la V1 (un template défectueux est signalé, pas servi à moitié).
- (b), une liste de dépendances dans le manifeste, est à **rejeter**. Typst n'a ni lockfile ni
  champ `dependencies` (recherche §1.4) : ce serait une seconde source de vérité à maintenir à
  la main, que les imports réels peuvent contredire. Ce n'est pas compatible avec VII.

---

## 5. Risques et points de vigilance

| Risque | Constat (vérifié) | Mesure |
|---|---|---|
| **WASM sans limite** | `typst-library` `plugin.rs:269-274` : `wasmi::Config::default()`, sans fuel. Le drapeau `cancel` n'est lu qu'aux accès au `World`. Un plugin (cetz 0.5, tiaoma, icu-datetime) ou une boucle Typst peut garder un créneau de rendu indéfiniment : `RenderGuard` ne libère le créneau qu'à la fin réelle du thread. | 002 : documenter le risque ; l'exemple de B1 évite le WASM ; mesurer SC-005 avec cetz. Plus tard : B6 (inventaire et politique). La vraie limite demanderait un fork, ou plus tard un processus isolé, ce que I interdit. On accepte donc un risque borné par la confiance dans le volume. |
| **Ruptures de Typst entre versions** | En 0.15 : `path` (forme) → `curve`, `pattern` → `tiling`, `*.decode` et `pdf.embed` supprimés, backslash interdit. `compiler` n'est qu'un **minimum** : `compiler ≤ 0.15.1` est nécessaire mais pas suffisant (fletcher 0.5.8 → cetz 0.3.4). | B2 vérifie `compiler`. La doc dit qu'un rendu d'essai est la seule preuve. Chaque montée de Typst dans inkpdf est une release à notes explicites, avec CI sur les templates d'exemple. Pistes ultérieures : un rendu de fumée au chargement à partir d'un `example.json` facultatif du template. |
| **`today()` non déterministe** | `world.rs:137` lit l'horloge. Un paquet tiers peut l'appeler sans que l'auteur le sache. Le PDF reste stable sur une journée, puis change. | Documenté dès 002 ; B7 rend la politique configurable. |
| **Diagnostics de paquet sans chemin** | `relative_path` renvoie `None` pour `Package` (`world.rs:79`). Par ailleurs, `PackageError::VersionNotFound` n'affiche pas le spec complet. | Corrigé **dans B1** (chemin `packages/…`, message propre à inkpdf nommant le spec). Le champ `Diagnostic.file` est déjà « relatif au dossier du template » : **aucun changement d'OpenAPI**. |
| **Taille des paquets** | icu-datetime 4 Mo, datify-core 2,7 Mo ; cetz et fletcher récent chargent deux cetz et deux oxifmt. Chaque template porte sa copie en mémoire. Les plugins sont recompilés par wasmi à chaque instanciation, ce que le cache `comemo` atténue en partie [S]. | La limite de 50 Mo s'applique déjà (FR-010). On surveille l'empreinte mémoire (constitution, Performance). Le stock partagé (B8) est le remède le jour où cela compte. |
| **Licences** | cetz et cetz-plot sont en LGPL-3.0+, rustycure en EUPL-1.2, les autres majoritairement en MIT. FR-016 oblige le **dépôt** à fournir un exemple avec paquet embarqué. Le `Dockerfile` ne copie pas `examples/` (seulement `src`, `benches`, `Cargo.*`), donc l'exemple n'est pas dans l'image, mais il est bien dans le dépôt. | Dans le volume, la licence est sous la responsabilité de l'auteur (hypothèse de la spec), et B3 rend les paquets auditables (option `license`). **L'exemple versionné de FR-016** utilise un paquet MIT ou `MIT OR Apache-2.0` (oxifmt 1.0.0, zero, ou un petit paquet écrit dans le dépôt). La mesure SC-005 avec cetz (LGPL) se fait sur une fixture de bench dont la licence est traitée explicitement (fichier de licence conservé, ou paquet récupéré en CI et non versionné). **Tout contenu livré sous `@inkpdf` dans l'image** doit être compatible avec la licence du projet (`TODO(LICENSE)`), donc pas de LGPL ni d'EUPL dans B9 et B10. |
| **Double chemin vers un octet** | Si `packages/**` restait servi par `Project`, `main.typ` pourrait lire un fichier de paquet et contourner la réservation de `inkpdf`. | B1 retire `packages/**` de `files`. |
| **Provenance et intégrité** | Aucune signature et aucun hash vérifiés. Un auteur peut modifier un paquet sans changer sa version. | Accepté : l'empreinte change, donc l'`ident` aussi. Une vérification de hash Universe éventuelle reste hors 002. |

---

## 6. Explicitement hors de la feature 002

- Tout accès réseau, téléchargement, miroir ou registre de paquets, et toute mise à jour
  automatique.
- Le stock de paquets partagé `_packages` (option 3) : seul le nom est réservé et documenté.
- Tout contenu sous `@inkpdf/*` (helpers, composants) : seule la réservation du namespace est
  livrée (B4).
- Le framework de composants « headless » piloté par `design`, et tout framework de design.
- Le template comme paquet Typst (`typst.toml` à la racine, compatibilité `typst init` ou
  Universe) : option 2.
- Une limite d'exécution des plugins WASM (fuel) et la politique WASM (B6).
- La maîtrise de `today()` (B7).
- Les outils d'auteur : installation, résolution transitive automatique, lockfile, CLI de
  vendoring.
- Le chargement des polices embarquées dans les paquets (inchangé : seul `fonts/` est chargé).
- Les vérifications de signature, de hash ou de licence des paquets.
- Tout changement d'API au-delà de l'ajout de `packages[]` au détail d'un template.
