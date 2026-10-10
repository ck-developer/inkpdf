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
- `bundled_packages` : les 29 paquets s'importent, les 21 paquets sélectionnés s'utilisent,
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
docker compose exec inkpdf true 2>/dev/null || true   # image distroless : pas de shell
# Facultatif : couper le réseau du conteneur
docker network disconnect "$(docker compose ps -q inkpdf | xargs docker inspect -f '{{range $k,$v := .NetworkSettings.Networks}}{{$k}}{{end}}')" "$(docker compose ps -q inkpdf)" || true
```

En local sans Docker : `INKPDF_TEMPLATES_DIR=examples/templates cargo run`.

## 4. Scénarios

### P1 — Paquets disponibles (US3)

```bash
curl -s localhost:3000/packages | jq '.packages | length, .[0]'
```

Attendu : `29`, puis un objet complet (`import`, `license`, `role`…).

### P2 — Génération avec paquets (US1)

```bash
curl -s -H 'content-type: application/json' \
  -d @examples/requests/packages-demo.json \
  -o demo.pdf localhost:3000/templates/packages-demo/render
```

Attendu : un PDF contenant un QR code, un montant formaté (« 1 234,56 ») et un graphique.
Deux appels identiques donnent des fichiers identiques (`cmp`).

### P3 — Paquet non intégré, détecté au dépôt (US2)

Copier un template qui importe `@preview/zero:0.5.0` dans le volume, puis :

```bash
curl -s localhost:3000/templates/<id> | jq '.status, .reason'
```

Attendu : `"invalid"`, avec la raison
`main.typ:N: package @preview/zero:0.5.0 is not bundled with inkpdf (available versions: 0.6.1, 0.7.1)`.
Une génération sur ce template répond **409** `template-invalid`.

### P4 — Import dynamique (US2)

Avec la fixture `tests/fixtures/templates/dynamic-import`, la génération répond **500**
`render-failed`, avec un diagnostic qui nomme le paquet.

### P5 — Confinement (FR-012)

`tests/sandbox.rs` vérifie qu'un paquet ne lit pas un fichier du template sans qu'on le lui
transmette, et qu'un template ne peut pas lire `packages/…` d'un autre paquet.

## 5. Documentation

- `docs/packages.md` : liste, usage, licence et présence de WASM pour chaque paquet.
- `docs/templates.md`, section « Utiliser un paquet » : règles de [template-imports.md](./contracts/template-imports.md).
