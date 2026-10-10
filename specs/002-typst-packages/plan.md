# Implementation Plan: Paquets Typst intégrés au service

**Branch**: `002-typst-packages` | **Date**: 2026-10-10 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/002-typst-packages/spec.md`

## Summary

Intégrer au binaire `inkpdf` un ensemble **fixe** de paquets Typst Universe : 20 paquets
**mis à disposition des templates**, chacun dans une seule version, plus leurs dépendances,
soit 28 au total (`codetastic`, incompatible avec Typst 0.15, a été retiré). Un template importe un paquet **par son seul nom**
(`#import "@preview/zero"`) ; la version installée est utilisée et écrire une version est refusé.

**Fichiers du dépôt**
- Les archives officielles sont **versionnées dans le dépôt** sous `packages/vendor/`
  (1,7 Mo compressés).
- Un **fichier de verrouillage** `packages/lock.toml` donne, pour chaque paquet, son nom, sa
  version, son empreinte sha256, sa licence et son rôle : `selected` (mis à disposition, au plus
  un par nom) ou `dependency`.

**Construction**
- Un `build.rs` vérifie les empreintes et fait échouer la compilation en cas d'écart.
- Il décompresse les archives et génère une table statique `include_bytes!` : les paquets font
  **partie du binaire**.

**Chargement d'un template** (une seule fois, jamais à chaque génération)
- Ses imports littéraux sont analysés avec le parseur Typst.
- Chaque `@preview/nom` est **réécrit** en `@preview/nom:<version installée>` dans l'instantané
  en mémoire.
- Tout autre import rend le template `invalid` : version écrite, paquet indisponible ou autre
  namespace.

**Rendu**
- `SandboxWorld` résout `VirtualRoot::Package(spec)` par correspondance exacte dans la table,
  sans vérification supplémentaire.

**API, tests et documentation**
- Une route `GET /packages` liste les paquets mis à disposition ; elle est ajoutée à l'OpenAPI
  verrouillé.
- Les tests prouvent que chaque paquet s'importe, que chaque paquet mis à disposition
  s'utilise, et que l'ensemble est **fermé**.

Détails : [research.md](./research.md).

## Technical Context

**Language/Version**: Rust stable 1.98, édition 2024 (inchangé)

**Primary Dependencies**:
- Inchangées : `typst` / `typst-pdf` / `typst-kit` / `typst-layout` 0.15.1, `axum` 0.8.9,
  `utoipa` 6.0.0. On utilise désormais `typst::syntax::package::PackageSpec` et
  `typst::syntax::{parse, ast}`.
- Nouvelles **build-dependencies** : `flate2`, `tar`, `sha2`, `toml`, `serde`.
  `flate2`, `sha2` et `toml` sont déjà dans `Cargo.lock` par transitivité, `tar` est nouveau.
- Aucune nouvelle dépendance d'exécution.

**Storage**: aucun ; paquets embarqués dans le binaire (données statiques, environ 6 Mo
décompressés)

**Testing**:
- `cargo test`, avec deux nouveaux fichiers : `tests/bundled_packages.rs` et
  `tests/api_packages.rs`.
- Extensions de `tests/sandbox.rs`, `tests/contract_openapi.rs` et `tests/perf.rs`.

**Target Platform**: Linux x86_64 / aarch64, image Docker (inchangée ; le `Dockerfile` copie
désormais `build.rs` et `packages/`)

**Project Type**: web-service (une crate `inkpdf`, lib + bin)

**Performance Goals**:
- Le template de démonstration existant ne doit pas régresser de plus de 5 % (SC-005).
- Le template d'exemple avec paquets doit se générer en moins de 1 s au p95 (SC-006).
- Le démarrage reste inchangé (aucune décompression à l'exécution).

**Constraints**:
- Aucun accès réseau au fonctionnement ni pendant `cargo build` et `cargo test`
  (les archives sont dans le dépôt).
- PDF déterministe ; bornes de rendu inchangées ; aucune nouvelle dépendance d'exécution.

**Scale/Scope**:
- 28 paquets, environ 6 Mo décompressés, environ 1 000 fichiers.
- 1 nouvelle route.
- Nouveaux modules : un `build.rs`, `src/packages/`, `src/template/imports.rs` et
  `src/api/packages.rs`.

Aucune inconnue restante : décisions R1–R15 dans [research.md](./research.md).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Principe | Statut | Comment le design le respecte |
|---|----------|--------|-------------------------------|
| I | Moteur Typst embarqué, sans navigateur | ✅ | Les paquets sont du code Typst exécuté par le moteur embarqué. Les plugins WASM de certains paquets s'exécutent **dans le processus** (interpréteur `wasmi` de Typst) : ni binaire externe ni sous-processus (R1, R12). |
| II | Template auto-suffisant | ✅ | Les paquets sont des « ressources embarquées par le binaire » au même titre que les polices par défaut, ce que le principe autorise explicitement. Rien n'est téléchargé : la résolution d'un template ne demande jamais d'accès réseau. Un template ne lit toujours aucun fichier hors de son dossier (R1, R5). |
| III | Entrée à double dimension, validée | ✅ | Inchangé : `data` et `design` sont validés avant compilation. |
| IV | Sécurité par construction | ✅ (dérogation V1 inchangée) | Aucun code de l'appelant. Les paquets sont choisis par le mainteneur, figés par empreinte et vérifiés à la compilation (R2, R3). Chaque paquet ne lit que sa propre racine (isolation native, R5). `cmarker`, qui pouvait exécuter du code venu des données, est exclu. **Durée** : la dérogation V1 s'applique toujours (un calcul long n'est pas interruptible), et elle est désormais plus probable avec les plugins WASM. Elle est documentée, sans nouveau mécanisme (R12). |
| V | Templates à chaud, binaire figé | ✅ | L'ensemble des paquets change seulement avec une nouvelle version du binaire, ce qui est cohérent avec « binaire figé ». Les templates restent ajoutés à chaud, et un template invalide (import non intégré) ne bloque ni le démarrage ni les autres templates (R8). |
| VI | API REST auto-descriptive | ✅ | `GET /packages` est décrit dans l'OpenAPI généré par utoipa, et l'instantané `openapi/openapi.json` est régénéré et verrouillé par le test de contrat (contracts/api.md). |
| VII | Simplicité et périmètre maîtrisé | ✅ | La fonctionnalité est spécifiée explicitement (spec 002). Ni base de données ni état. Aucune nouvelle dépendance d'exécution. Pas de dossier partagé, de helpers ni de téléchargement. |
| Workflow | fmt, clippy, tests ; OpenAPI et doc dans le même changement | ✅ | Les tests bac à sable sont étendus (import non intégré refusé, paquet confiné à sa racine). `docs/packages.md` et `docs/templates.md` sont mis à jour, avec un test de synchronisation (R11). |

**Re-check post-design (Phase 1)** : ✅ — les contrats ([api.md](./contracts/api.md),
[lock-file.md](./contracts/lock-file.md) et [template-imports.md](./contracts/template-imports.md))
n'introduisent ni accès réseau, ni lecture hors des racines, ni état. Le seul point de
vigilance reste la durée des plugins WASM, couverte par la dérogation existante.

## Project Structure

### Documentation (this feature)

```text
specs/002-typst-packages/
├── spec.md
├── synthese.md            # résumé non technique des décisions
├── plan.md                # ce fichier
├── research.md            # R1–R15
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── api.md             # GET /packages + diagnostics de paquet
│   ├── lock-file.md       # format de packages/lock.toml et règles d'évolution
│   └── template-imports.md# règles d'import pour les auteurs
├── research/              # études préalables (typst.md, landscape.md, carte-paquets.md…)
├── checklists/requirements.md
└── tasks.md               # /speckit-tasks
```

### Source Code (repository root)

```text
build.rs                       # NOUVEAU : vérifie sha256, extrait, génère la table statique
packages/
├── lock.toml                  # NOUVEAU : liste fixée (28 entrées)
└── vendor/                    # NOUVEAU : archives officielles <nom>-<version>.tar.gz
scripts/
└── add-package.sh             # NOUVEAU : télécharge une archive + écrit l'entrée du lock
src/
├── packages/
│   └── mod.rs                 # NOUVEAU : BundledPackage, index (spec → paquet), versions_of
├── template/
│   └── imports.rs             # NOUVEAU : analyse et réécriture des imports `@preview/nom` (R8, R15)
├── registry/loader.rs         # MODIFIÉ : réécrit les imports ; template invalide si import incorrect
├── render/world.rs            # MODIFIÉ : résolution VirtualRoot::Package, diagnostics, cache
├── render/mod.rs              # MODIFIÉ : chemin de diagnostic « @preview/nom:ver/… »
├── api/packages.rs            # NOUVEAU : GET /packages
└── api/mod.rs                 # MODIFIÉ : route + schémas OpenAPI
tests/
├── bundled_packages.rs        # NOUVEAU : import de chaque paquet, usage des 20, fermeture, doc
├── api_packages.rs            # NOUVEAU : contrat de GET /packages
├── sandbox.rs                 # MODIFIÉ : import non intégré refusé, confinement des paquets
├── fixtures/package-smoke/    # NOUVEAU : un .typ minimal par paquet sélectionné
└── fixtures/templates/        # NOUVEAU : unknown-package, version-written, dynamic-import
examples/templates/packages-demo/  # NOUVEAU : QR code + montant formaté + graphique (FR-018)
docs/packages.md               # NOUVEAU : paquets disponibles, usages, licences
docs/templates.md              # MODIFIÉ : section « Utiliser un paquet »
openapi/openapi.json           # RÉGÉNÉRÉ
Dockerfile                     # MODIFIÉ : COPY build.rs packages/
```

**Structure Decision**: on garde la crate unique de la V1. Les paquets sont une donnée de
construction (`packages/`), transformée en code statique par `build.rs`. La logique
d'exécution tient dans un petit module `src/packages/` consommé par le `World`, le chargeur et
l'API.

## Complexity Tracking

Aucune nouvelle violation. La dérogation V1 sur la durée (principe IV) reste valable et est
étendue aux plugins WASM des paquets intégrés (R12).

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| `build.rs` avec génération de code | Les paquets doivent être dans le binaire, vérifiés par empreinte, et lisibles sans décompression à l'exécution | Décompresser au démarrage ajouterait deux dépendances d'exécution (`flate2`, `tar`) et un coût au démarrage. Copier les fichiers dans l'image Docker casserait `cargo test` sans Docker et le principe « binaire unique » (R1). |

## Extension du périmètre (2026-10-10)

La feature 002 inclut aussi les quatre ajouts suivants.

| Ajout | Décision |
|---|---|
| Téléchargement | `?download=true&filename=…` sur la route de rendu (R18) |
| Métadonnées | `metadata` facultatif, appliqué au document compilé, auteur par défaut configurable (`INKPDF_DEFAULT_AUTHOR`) (R17) |
| Renommage | `design` devient `layout`, avec amendement PATCH de la constitution (R16) |
| Exemples | collection Bruno et facture de situation BTP multipage avec plusieurs requêtes (R19) |

**Constitution Check (extension)** : ✅
- **III** : renommage seulement, le principe est inchangé. Amendement 1.0.2.
- **IV** : les métadonnées sont des données, appliquées hors du code Typst ; le nom de fichier
  est nettoyé.
- **VI** : nouveaux paramètres et nouveau code d'erreur décrits dans l'OpenAPI.
- **VII** : ni état ni cache, et `src/` reste agnostique, car tout le contenu métier de la
  facture vit dans `examples/`.

**Fichiers touchés en plus** :
- **code** : `src/render/metadata.rs` (nouveau), `src/render/mod.rs` (`compile_pdf`
  applique les métadonnées), `src/api/render.rs` (paramètres, `metadata`, en-tête),
  `src/template/schema.rs` et `src/render/world.rs` (`layout`), `src/config.rs`
  (`default_author`), `src/error.rs` (`invalid-parameter`) ;
- **constitution** : `.specify/memory/constitution.md` ;
- **exemples et tests** : `examples/templates/progress-invoice/`,
  `examples/requests/progress-invoice/`, `examples/bruno/`, `tests/api_metadata.rs`,
  `tests/examples.rs` ;
- **documentation** : `docs/templates.md`, README.
