# Research: Service de génération de PDF à partir de templates (V1)

**Feature**: `001-pdf-generation-service` | **Date**: 2026-10-09

Versions vérifiées sur crates.io le 2026-10-09 ; API vérifiées dans les sources publiées
(`cargo fetch` dans un projet sonde).

## R1. Moteur de rendu embarqué

- **Decision**: crates `typst` 0.15.1 (compilation), `typst-pdf` 0.15.1 (export),
  `typst-kit` 0.15.1 avec uniquement la feature `embedded-fonts`. Implémentation maison du
  trait `typst::World` (`SandboxWorld`).
- **Rationale**: bibliothèque Rust pure, aucun sous-processus (constitution I). `typst-kit`
  sans `packages`/`http-server`/`scan-fonts` n'embarque ni téléchargeur ni scan des polices
  système.
- **Alternatives considered**: CLI `typst` en sous-processus (interdit, principe I) ;
  utiliser `typst-kit` complet (tire le téléchargeur de paquets, contraire au principe II).

## R2. Injection des données dans Typst

- **Decision**: les données sont passées via `LibraryBuilder::with_inputs(Dict)` et lues par le
  template avec `sys.inputs.data` et `sys.inputs.design`. Le JSON est converti récursivement en
  `typst::foundations::Value` (objet → `Dict`, tableau → `Array`, chaîne → `Str`, entier →
  `Int`, flottant → `Float`, booléen → `Bool`, null → `None`).
- **Rationale**: `with_inputs` accepte un `Dict` de valeurs arbitraires (vérifié dans
  `typst-library-0.15.1/src/lib.rs:207`) ; la limitation « chaînes uniquement » n'existe que
  dans la CLI. Aucune source Typst n'est générée ni interpolée : une chaîne contenant
  `#import` reste une chaîne (constitution IV, spec edge case « texte ressemblant à du code »).
- **Alternatives considered**: fichier virtuel `input.json` lu par `json()` (fonctionne mais
  ajoute un chemin réservé dans l'espace de fichiers du template) ; interpolation de source
  (interdite, principe IV).
- **Note**: les nombres JSON hors plage `i64` sont convertis en `Float` ; les valeurs
  décimales exactes DEVRAIENT être transmis en chaînes ou en entiers (centimes), documenté dans
  `contracts/template-format.md`.

## R3. Noms des deux sections du JSON d'entrée

- **Decision**: corps de génération `{ "data": { … }, "design": { … } }`. `data` est requis ;
  `design` est optionnel (défauts appliqués). Toute autre clé de premier niveau est refusée.
- **Rationale**: lecture directe de la spec (données métier / paramètres de design), noms
  courts et identiques côté API et côté Typst (`sys.inputs.data`, `sys.inputs.design`).
- **Alternatives considered**: `business`/`theme`, ou paramètres de design en query string
  (peu pratique pour des structures imbriquées).

## R4. Validation JSON Schema et valeurs par défaut

- **Exposition**: le `schema.json` est exposé **tel quel** (octets d'origine, constitution VI) ;
  la contrainte racine `additionalProperties: false` éventuellement ajoutée par le service ne
  s'applique qu'à la copie compilée dans le validateur.
- **Decision**: `jsonschema` 0.58.6 avec `default-features = false` (garder `idna` si besoin),
  draft 2020-12. Le `schema.json` d'un template est compilé une fois au chargement et mis en
  cache dans l'entrée du registre. Ordre à la génération :
  1. parse JSON (erreur `invalid-json`) ;
  2. **application des défauts** : parcours de `properties.design` (récursif sur les
     `properties` d'objets) et insertion des `default` absents ;
  3. validation complète (erreur `validation-failed` avec toutes les violations : chemin JSON
     Pointer `instance_path`, chemin dans le schéma, message).
- **Rationale**: `jsonschema` valide mais n'applique pas `default` (aucune API dédiée dans
  0.58.6). Appliquer avant de valider permet qu'un champ de design `required` muni d'un défaut
  passe quand l'appelant l'omet (FR-005). Les features par défaut tirent `reqwest`
  (`resolve-http`) : désactivées pour garantir qu'un `$ref` distant n'entraîne aucun accès
  réseau ; les `$ref` sont limités aux références internes au document.
- **Alternatives considered**: valider avant d'appliquer les défauts (rejette des requêtes
  légitimes) ; laisser Typst gérer les défauts (`dict.at(k, default: …)`) — conservé comme
  bonne pratique côté template mais non suffisant pour l'auto-description de l'API.

## R5. Résolution des fichiers et bac à sable

- **Decision**: le contrôle se fait en deux temps.
  - **Au chargement** (registre) : seuls les fichiers dont le chemin canonicalisé reste sous
    le dossier du template entrent dans l'instantané en mémoire ; un lien symbolique sortant
    est exclu (log `warn`). C'est le seul moment où le disque est lu (voir R9).
  - **Au rendu** : `SandboxWorld` ne lit jamais le disque. Il refuse tout `FileId` portant un
    `PackageSpec` (erreur `package imports are not supported`), normalise le chemin virtuel
    (toute composante `..` sortant de la racine → `AccessDenied`) et ne sert que les fichiers
    présents dans l'instantané (`NotFound` sinon).
  Pas de `scan-fonts`, pas d'accès réseau, pas de variables d'environnement. `World::today()`
  renvoie la date UTC courante.
- **Rationale**: principes II et IV ; un `@preview/…` serait résolu par téléchargement, il est
  donc rejeté en V1.
- **Alternatives considered**: dossier `packages/` vendored dans le template (reporté :
  utile mais non requis en V1, peut faire l'objet d'une évolution MINOR).
- **Identifiant de template**: doit correspondre à `^[a-z0-9][a-z0-9_-]{0,63}$` ; sinon `404`
  sans aucun accès disque (protection contre `../`).

## R6. Polices

- **Decision**: polices embarquées par `typst-assets` (via `typst-kit/embedded-fonts`),
  chargées une fois au démarrage, + polices du dossier `fonts/` du template (TTF/OTF/TTC),
  chargées au chargement du template. Aucune police système.
- **Rationale**: auto-suffisance, déterminisme, image reproductible.
- **Alternatives considered**: `fontdb` sur les polices système (non déterministe, dépend de
  l'image).

## R7. Déterminisme du PDF (FR-017)

- **Decision**: `PdfOptions { ident: Smart::Custom(<id>@<empreinte du template>),
  timestamp: None, .. }`.
- **Rationale**: `ident` et `timestamp` sont les deux sources de variation de l'export
  (vérifié dans `typst-pdf-0.15.1/src/lib.rs`). Avec eux fixés, deux rendus identiques sont
  identiques à l'octet près.
- **Limite documentée**: un template utilisant `datetime.today()` n'est pas déterministe d'un
  jour à l'autre ; recommandation : passer la date dans `data`.

## R8. Durée bornée de la génération (FR-016, principe IV)

- **Decision**:
  - la compilation tourne dans `tokio::task::spawn_blocking`, derrière un `Semaphore` de
    `INKPDF_MAX_CONCURRENT_RENDERS` permis (défaut : nombre de CPU) ;
  - l'acquisition du permis est bornée (`INKPDF_QUEUE_TIMEOUT_SECS`, défaut 10 s) → sinon
    `503 overloaded` ;
  - la réponse est bornée par `tokio::time::timeout(INKPDF_RENDER_TIMEOUT_SECS)` → `504
    render-timeout` ;
  - un drapeau d'annulation partagé est vérifié dans chaque appel `World::source/file/font` :
    une compilation dont le délai est dépassé échoue dès son prochain accès au monde ;
  - le permis n'est rendu qu'à la fin réelle du thread de compilation.
- **Rationale**: Typst 0.15 n'expose pas d'API d'annulation. Ce dispositif garantit que
  l'appelant reçoit toujours une réponse dans le délai, que le nombre de compilations
  simultanées est borné et que le service reste disponible.
- **Limite résiduelle (assumée)**: une boucle infinie purement calculatoire dans un template
  (sans accès au monde) continue d'occuper un thread et un permis jusqu'à la fin du processus.
  Mitigation : les templates sont des artefacts de confiance relative (déposés par l'équipe,
  pas par l'appelant) ; la métrique `renders_in_flight` et un log `render.overrun` signalent
  le cas. Voir Complexity Tracking du plan.
- **Alternatives considered**: processus de rendu isolé tuable (interdit, principe I) ;
  thread dédié tué (impossible proprement en Rust).

## R9. Découverte à chaud (US4, SC-003)

- **Decision**: registre en mémoire `ArcSwap<BTreeMap<TemplateId, Arc<TemplateEntry>>>`
  alimenté par :
  1. un scan complet au démarrage ;
  2. `notify` 8.2 (watcher récursif) avec anti-rebond de 500 ms ;
  3. un rescan périodique de secours (`INKPDF_RESCAN_INTERVAL_SECS`, défaut 2 s).
  Chaque dossier a une **empreinte** (liste triée chemin+taille+mtime de ses fichiers) ; un
  template n'est rechargé que si l'empreinte change, et seulement si elle est restée stable
  entre deux observations successives espacées d'1 s (protection contre la copie en cours).
  Pendant ce temps, l'ancienne version reste servie.
- **Instantané en mémoire**: au chargement, **tous les fichiers du template** (`main.typ`,
  fichiers importés, `assets/`, `fonts/`, `schema.json`) sont lus en mémoire dans
  `TemplateEntry` ; l'empreinte est recalculée après lecture et le chargement est abandonné
  (nouvel essai au tour suivant) si elle a changé entre-temps. Le rendu ne lit **jamais** le
  disque : `SandboxWorld` sert uniquement cet instantané. Ainsi un rendu voit toujours une
  version complète et cohérente du template (spec, cas limite « copie en cours » ; constitution
  V, cohérence du cache), et la latence de rendu ne dépend pas des I/O (SC-001).
- **Empreinte**: calculée dès le chargement initial (phase fondations), car elle sert aussi à
  l'`ident` déterministe du PDF (R7).
- **Rationale**: inotify n'est pas fiable sur NFS ni sur les ConfigMaps Kubernetes (échange
  de liens symboliques `..data`) ; rescan 2 s + confirmation à 1 s ⇒ ≤ 3,5 s au pire (< 5 s). Le remplacement atomique du
  registre évite tout état partiel côté lecteur.
- **Alternatives considered**: scan à chaque requête (coût par requête, I/O inutiles) ;
  watcher seul (non fiable sur volumes réseau).
- **Taille maximale d'un template**: `INKPDF_MAX_TEMPLATE_BYTES` (défaut 50 Mo). La somme des
  tailles est connue dès le calcul de l'empreinte, **avant** toute lecture : au-delà, le
  template est marqué `invalid` (raison : taille constatée et limite) sans être lu. La
  mémoire occupée par le registre est ainsi bornée par (nombre de templates × limite).
- **Cache Typst**: `comemo::evict(10)` après chaque génération pour borner la mémoire.

## R10. HTTP, OpenAPI, erreurs

- **Decision**: `axum` 0.8.9 + `tokio` 1.53 + `tower-http` 0.7 (`RequestBodyLimitLayer`,
  `TraceLayer`). OpenAPI 3.1 généré par `utoipa` 6.0.0 + `utoipa-axum` 0.3.0 (qui dépend bien
  d'`axum` 0.8.9, vérifié), servi sur `/openapi.json`, UI de documentation `utoipa-scalar`
  0.4.0 sur `/docs`. Erreurs au format RFC 9457 `application/problem+json`.
- **Synchronisation OpenAPI ↔ code (principe VI)**: le document est **généré depuis le code** ;
  le test `tests/contract_openapi.rs` compare le document généré au fichier versionné
  `openapi/openapi.json` et échoue en cas de divergence ; l'alignement avec la référence de
  conception `contracts/openapi.yaml` est vérifié une fois lors de la création de ce fichier ; `INKPDF_UPDATE_OPENAPI=1 cargo test` régénère le fichier.
- **Rationale**: génération depuis les types Rust = impossible d'oublier un champ ; le fichier
  versionné rend les changements de contrat visibles en revue.
- **Risque**: utoipa 6.0.0 et utoipa-axum 0.3.0 ont ~2 semaines. Versions épinglées (`=`) ;
  repli possible sur utoipa 5.x / utoipa-axum 0.2 si un défaut bloquant apparaît.
- **Alternatives considered**: `aide` (moins répandu) ; OpenAPI écrit à la main + test de
  conformité (risque de dérive plus élevé).

## R11. Configuration, logs, santé

- **Decision**: configuration par variables d'environnement (`INKPDF_*`, voir plan) ;
  `tracing` + `tracing-subscriber` en JSON sur stdout ; `GET /health` (vivacité) et
  `GET /ready` (registre initialisé). Sous-commande `inkpdf healthcheck` pour le
  `HEALTHCHECK` Docker (l'image n'a ni shell ni curl).
- **Rationale**: principe « Observabilité » de la constitution, FR-021 à FR-023 ; aucune donnée
  métier dans les logs.

## R12. Image Docker

- **Decision**: build multi-étapes `rust:1.98-bookworm` → runtime
  `gcr.io/distroless/cc-debian12:nonroot`, utilisateur non root, volume `/templates` monté en
  lecture seule. Seuil vérifié : image < 100 Mo (polices embarquées comprises).
- **Rationale**: SC-009 (Gotenberg ≈ 1,5 Go → facteur > 5 largement tenu) ; distroless sans
  shell réduit la surface.
- **Alternatives considered**: `scratch` + musl (allocateur musl plus lent, gain de taille
  marginal).

## R13. Tests et mesures

- **Decision**: `cargo test` ; tests HTTP en mémoire via `tower::ServiceExt::oneshot` ;
  `tempfile` pour des volumes jetables ; `pdf-extract` (dev) pour vérifier le texte d'un PDF ;
  `criterion` pour SC-001 (bench `benches/render.rs`) ; `oha` en manuel pour SC-007.
- **Rationale**: tout s'exécute sans Docker ni réseau ; chaque critère de succès a un moyen de
  vérification identifié.

## Hors périmètre confirmé

Licence (TODO constitutionnel, proposition `MIT OR Apache-2.0`), paquets Typst vendored,
génération par lots, métriques Prometheus (logs suffisants en V1).
