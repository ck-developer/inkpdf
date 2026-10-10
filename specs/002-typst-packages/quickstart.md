# Quickstart — Valider les paquets intégrés (002)

Ce guide sert à vérifier que la feature fonctionne de bout en bout. Les règles détaillées sont
dans les contrats : [api.md](./contracts/api.md), [lock-file.md](./contracts/lock-file.md) et
[template-imports.md](./contracts/template-imports.md).

## Prérequis

- Les mêmes que pour la V1 : Rust 1.98, ou Docker avec `docker compose`.
- **Aucun accès réseau n'est nécessaire**, ni pour construire ni pour tester : les archives
  des paquets sont dans `packages/vendor/`.

## 1. Tests automatisés

```bash
cargo test                       # inclut bundled_packages et api_packages
cargo test --test bundled_packages -- --nocapture
cargo test --release --test perf -- --ignored
```

Résultats attendus :
- `bundled_packages` : les 28 paquets s'importent, les 20 paquets mis à disposition s'utilisent,
  l'ensemble est fermé, et `docs/packages.md` est synchrone avec le lock.
- `perf` : le p95 de `sample` reste à moins de 5 % de la V1, et celui de `packages-demo` sous
  1 s.

## 2. Intégrité (SC-007)

```bash
cp packages/vendor/zero-0.7.1.tar.gz /tmp/zero.bak
printf 'x' >> packages/vendor/zero-0.7.1.tar.gz
cargo build                       # attendu : échec « zero 0.7.1: sha256 mismatch »
cp /tmp/zero.bak packages/vendor/zero-0.7.1.tar.gz
```

## 3. Lancer le service sans réseau

```bash
docker compose up --build -d
# Pour prouver l'absence de réseau : ajouter `network_mode: none` au service dans
# compose.yaml (et publier le port via un autre moyen) ou lancer les tests, qui tournent hors ligne.
```

En local sans Docker : `INKPDF_TEMPLATES_DIR=examples/templates cargo run`.

## 4. Scénarios

### P1 — Paquets disponibles (US3)

```bash
curl -s localhost:3000/packages | jq '.packages | length, .[0]'
```

Attendu : `20` (les paquets mis à disposition), puis un objet complet
(`import: "@preview/…"`, `version`, `license`…).

### P2 — Génération avec paquets (US1)

```bash
curl -s -H 'content-type: application/json' \
  -d @examples/requests/packages-demo.json \
  -o demo.pdf localhost:3000/templates/packages-demo/render
```

Le template `packages-demo` importe ses paquets **sans version** (`#import "@preview/zero"`).

Attendu : un PDF contenant un QR code, un montant formaté (« 1 234,56 ») et un graphique.
Deux appels identiques donnent des fichiers identiques (`cmp`).

### P3 — Import incorrect, détecté au dépôt (US2)

Copier dans le volume un template qui importe `@preview/zero:0.7.1`, version écrite, puis un
autre qui importe `@preview/foo`, paquet inconnu :

```bash
curl -s localhost:3000/templates/<id> | jq '.status, .reason'
```

Résultats attendus :
- statut `"invalid"` ;
- raisons respectives : `main.typ:N: remove the version: write @preview/zero (inkpdf uses its installed version)`
  et `main.typ:N: package @preview/foo is not available in inkpdf (see GET /packages)` ;
- une génération sur ces templates répond **409** `template-invalid`.

### P4 — Import construit par calcul

Avec la fixture `tests/fixtures/templates/dynamic-import`, la génération répond **500**
`render-failed` (non pris en charge, règle 4 de [template-imports.md](./contracts/template-imports.md)).

### P5 — Confinement (FR-012)

`tests/sandbox.rs` vérifie qu'un paquet ne lit pas un fichier du template sans qu'on le lui
transmette, et qu'un template ne peut pas lire `packages/…` d'un autre paquet.

## 5. Documentation

- `docs/packages.md` : liste, usage, licence et présence de WASM pour chaque paquet.
- `docs/templates.md`, section « Utiliser un paquet » : règles de [template-imports.md](./contracts/template-imports.md).
