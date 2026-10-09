# Implementation Plan: Service de génération de PDF à partir de templates (V1)

**Branch**: `001-pdf-generation-service` | **Date**: 2026-10-09 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-pdf-generation-service/spec.md`

## Summary

Construire `inkpdf`, un service **générique et agnostique du contenu** (FR-024) : un binaire Rust unique qui expose une API REST de découverte et de
génération de PDF. Les templates (dossier `main.typ` + `schema.json` + ressources) sont
découverts à chaud dans un volume monté ; le corps `{ data, design }` reçoit les valeurs par
défaut de `design`, est validé par JSON Schema, puis injecté dans Typst via `sys.inputs`
(jamais par interpolation de source) et compilé en PDF par le moteur Typst embarqué, dans un
`World` bac à sable sans réseau ni accès hors du dossier du template. Le document OpenAPI est
généré depuis le code et verrouillé par un test de contrat. Chaque template est chargé en
mémoire (instantané cohérent) : le rendu ne lit jamais le disque. Détails : [research.md](./research.md).

## Technical Context

**Language/Version**: Rust stable 1.98, édition 2024

**Primary Dependencies**: `typst` / `typst-pdf` / `typst-kit` 0.15.1 (feature
`embedded-fonts` seulement), `axum` 0.8.9, `tokio` 1.53, `tower-http` 0.7, `utoipa` 6.0.0 +
`utoipa-axum` 0.3.0 + `utoipa-scalar` 0.4.0, `jsonschema` 0.58.6 (`default-features = false`),
`notify` 8.2, `arc-swap`, `serde`/`serde_json`, `tracing` + `tracing-subscriber`, `thiserror`

**Storage**: aucune base ; volume de templates en lecture seule ; registre en mémoire

**Testing**: `cargo test` (unitaires + intégration HTTP en mémoire via `tower::ServiceExt`),
`tempfile`, `pdf-extract` (texte des PDF), `criterion` (bench SC-001), `oha` (charge, manuel)

**Target Platform**: Linux x86_64 / aarch64, image Docker `ghcr.io/ck-developer/inkpdf`
(distroless, non root)

**Project Type**: web-service (une crate `inkpdf`, lib + bin)

**Performance Goals**: p95 < 200 ms pour un document 1 page / tableau de 20 lignes (SC-001) ; prêt en
< 2 s avec 50 templates (SC-002) ; nouveau template visible en < 5 s (SC-003) ; 20 rendus
simultanés sans erreur (SC-007)

**Constraints**: aucun sous-processus ni navigateur ; aucun accès réseau ; aucun fichier lu
hors du dossier du template ; corps ≤ 5 Mo et rendu ≤ 30 s par défaut ; PDF déterministe ;
image < 100 Mo (SC-009)

**Scale/Scope**: dizaines de templates par volume, trafic interne (dizaines de req/s),
7 endpoints

Aucune inconnue restante : toutes les décisions sont résolues dans [research.md](./research.md)
(R1–R13).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Principe | Statut | Comment le design le respecte |
|---|----------|--------|-------------------------------|
| I | Moteur Typst embarqué, sans navigateur | ✅ | Crates `typst`/`typst-pdf` liées au binaire ; aucune dépendance de processus ; `typst-kit` sans `packages`/`http-server` (R1) |
| II | Template auto-suffisant | ✅ | `SandboxWorld` ne sert que l'instantané en mémoire des fichiers du template (chemins normalisés, aucun accès disque au rendu) ; imports de paquets rejetés ; polices = embarquées + `fonts/` (R5, R6) |
| III | Entrée à double dimension, validée | ✅ | Corps `{data, design}` ; défauts `design` appliqués puis validation JSON Schema complète avant compilation ; violations avec JSON Pointer (R3, R4) |
| IV | Sécurité par construction | ⚠️ dérogation approuvée (2026-10-09) | JSON seul, injection via `sys.inputs` ; identifiants filtrés par regex ; `jsonschema` sans résolution HTTP ; taille bornée. **Durée** : réponse bornée et concurrence bornée, mais une boucle purement calculatoire ne peut être interrompue → voir Complexity Tracking (R8) |
| V | Templates à chaud, binaire figé | ✅ | Watcher `notify` + rescan 2 s + empreinte stable ; registre `ArcSwap` ; template invalide listé `invalid` sans bloquer les autres (R9) |
| VI | API REST auto-descriptive | ✅ | 7 endpoints ; OpenAPI 3.1 généré par `utoipa`, servi sur `/openapi.json`, verrouillé par `tests/contract_openapi.rs` ; `schema.json` exposé tel quel (octets d'origine) ; erreurs RFC 9457 avec `code` (R10) |
| VII | Simplicité, périmètre V1 | ✅ | Ni auth, ni base, ni upload, ni lots ; une seule crate ; sans état |
| — | Contraintes techniques | ✅ | Config `INKPDF_*`, logs JSON sans données métier, `/health` + `/ready`, Docker distroless (R11, R12) |
| — | Workflow qualité | ✅ | fmt + clippy `-D warnings` + tests ; tests obligatoires (validation, contrat, e2e, sandbox) prévus ; template de démonstration `sample` (neutre, sans domaine métier) |

**Note** : `TODO(LICENSE)` de la constitution reste ouvert ; il ne bloque pas le
développement mais DOIT être tranché avant la publication de la crate et de l'image
(`MIT OR Apache-2.0` proposé).

**Re-check post-design (Phase 1)** : ✅ — data-model, contrats et quickstart ne réintroduisent
aucune violation ; le seul écart (IV, durée) reste celui justifié ci-dessous.

## Project Structure

### Documentation (this feature)

```text
specs/001-pdf-generation-service/
├── plan.md              # Ce fichier
├── research.md          # Phase 0
├── data-model.md        # Phase 1
├── quickstart.md        # Phase 1
├── contracts/
│   ├── openapi.yaml     # Contrat REST (référence de conception)
│   └── template-format.md  # Contrat auteurs de templates
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
Cargo.toml                  # crate `inkpdf` (lib + bin), versions épinglées
Dockerfile                  # multi-étapes → distroless nonroot, HEALTHCHECK `inkpdf healthcheck`
.dockerignore
openapi/
└── openapi.json            # document généré, versionné, vérifié par test
src/
├── main.rs                 # CLI : `serve` (défaut) | `healthcheck`
├── lib.rs                  # `build_app(state) -> Router`
├── config.rs               # variables INKPDF_*
├── error.rs                # ApiError → application/problem+json (codes du data-model)
├── api/
│   ├── mod.rs              # routeur utoipa-axum, /docs, /openapi.json
│   ├── templates.rs        # GET /templates, /templates/{id}, /templates/{id}/schema
│   ├── render.rs           # POST /templates/{id}/render
│   └── health.rs           # GET /health, /ready
├── registry/
│   ├── mod.rs              # Registry (ArcSwap), lookup, snapshot
│   ├── loader.rs           # chargement + validation d'un dossier → TemplateEntry
│   ├── fingerprint.rs      # empreinte (chemin, taille, mtime)
│   └── watcher.rs          # notify + rescan périodique + anti-rebond
├── template/
│   ├── id.rs               # TemplateId (regex)
│   ├── manifest.rs         # template.json
│   └── schema.rs           # compilation, défauts design, validation → Violations
└── render/
    ├── mod.rs              # pipeline : sémaphore, timeout, annulation, export PDF
    ├── world.rs            # SandboxWorld (impl typst::World)
    ├── fonts.rs            # polices embarquées + fonts/ du template
    └── value.rs            # serde_json::Value → typst Value
tests/
├── common/mod.rs           # volume temporaire, app en mémoire, helpers PDF
├── contract_openapi.rs     # document généré == openapi/openapi.json
├── api_templates.rs        # US3 : liste, détail, schéma, 404
├── api_render.rs           # US1/US2 : génération, défauts, design, erreurs 4xx/5xx
├── hot_reload.rs           # US4 : ajout, modification, suppression, invalide, copie en cours
├── sandbox.rs              # traversée, paquets, lecture hors dossier, injection de code
└── fixtures/templates/     # templates de test (valides, invalides, malveillants)
benches/
└── render.rs               # criterion, SC-001
examples/
├── templates/sample/       # template de démonstration neutre (main.typ, schema.json, template.json)
└── requests/sample.json    # corps d'exemple
```

**Structure Decision**: projet unique (une crate lib + bin). La lib porte toute la logique
pour être testée en mémoire sans réseau ; le bin se limite au démarrage. Les modules suivent
les quatre responsabilités du service : API, registre, template, rendu.

### Configuration

| Variable | Défaut | Rôle |
|----------|--------|------|
| `INKPDF_TEMPLATES_DIR` | `/templates` | volume des templates |
| `INKPDF_LISTEN` | `0.0.0.0:3000` | adresse d'écoute |
| `INKPDF_MAX_BODY_BYTES` | `5242880` | taille max du corps |
| `INKPDF_RENDER_TIMEOUT_SECS` | `30` | durée max d'un rendu |
| `INKPDF_MAX_CONCURRENT_RENDERS` | nb de CPU | rendus simultanés |
| `INKPDF_QUEUE_TIMEOUT_SECS` | `10` | attente max d'un créneau de rendu |
| `INKPDF_RESCAN_INTERVAL_SECS` | `2` | rescan de secours du volume |
| `INKPDF_MAX_TEMPLATE_BYTES` | `52428800` | taille max d'un template (instantané en mémoire) |
| `INKPDF_LOG_FORMAT` | `json` | `json` ou `pretty` |
| `RUST_LOG` | `info` | niveau de logs |

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Principe IV « génération bornée » partiellement tenu — **dérogation approuvée par le mainteneur le 2026-10-09** (cf. Governance : justifiée ici et approuvée) : une boucle infinie purement calculatoire dans un template occupe un thread et un créneau de rendu jusqu'à l'arrêt du processus, même si l'appelant reçoit `504` dans le délai | Typst 0.15.1 n'offre pas d'annulation ; un sous-processus tuable est interdit par le principe I. Mitigations : délai de réponse garanti, concurrence bornée par sémaphore, annulation coopérative à chaque accès au `World`, log `render.overrun`, `503 overloaded` si tous les créneaux sont pris ; les templates sont déposés par l'équipe, pas par l'appelant | Processus de rendu séparé (viole I) ; arrêt forcé d'un thread (impossible sans risque en Rust) ; ne rien borner (viole IV et exposerait tout le service). À réévaluer si Typst expose une annulation |
