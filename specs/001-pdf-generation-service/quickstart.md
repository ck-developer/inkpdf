# Quickstart & validation : Service de génération de PDF (V1)

Guide de vérification de bout en bout. Les formats sont décrits dans
[contracts/openapi.yaml](./contracts/openapi.yaml) et
[contracts/template-format.md](./contracts/template-format.md) ; ils ne sont pas répétés ici.

## Prérequis

- Rust 1.98+ (`rustup`), ou Docker 29+.
- `curl`, `jq`, `pdftotext` (poppler-utils) pour les vérifications manuelles.
- Le template de démonstration neutre `examples/templates/sample/` livré dans le dépôt (il
  sert uniquement à la vérification ; le service n'a aucune connaissance métier).

## 1. Tests automatisés

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Attendu : tout passe, y compris `contract_openapi` (document généré identique au fichier
versionné) et `sandbox` (cas malveillants refusés).

## 2. Lancer le service

En local :

```sh
INKPDF_TEMPLATES_DIR=examples/templates cargo run --release
```

Ou via Docker :

```sh
docker build -t inkpdf:dev .
docker run --rm -p 3000:3000 \
  -v "$PWD/examples/templates:/templates:ro" inkpdf:dev
```

Attendu : `GET /ready` → `200 {"status":"ok", ...}` en moins de 2 s (SC-002).

## 3. Scénarios

### S1 — Découverte (US3)

```sh
curl -s localhost:3000/templates | jq
curl -s localhost:3000/templates/sample | jq '.schema.properties | keys'
curl -s localhost:3000/openapi.json | jq '.paths | keys'
```

Attendu : `sample` listé avec `status: "valid"` ; le schéma expose `data` et `design` ;
l'UI `http://localhost:3000/docs` affiche toutes les opérations.

### S2 — Génération (US1)

```sh
curl -s -o /tmp/f.pdf -w '%{http_code} %{content_type} %{time_total}\n' \
  -H 'Content-Type: application/json' \
  -d @examples/requests/sample.json \
  localhost:3000/templates/sample/render
pdftotext /tmp/f.pdf - | grep -E 'Exemple|Alpha'
```

Attendu : `200 application/pdf`, le texte contient le titre et les libellés envoyés.

### S3 — Personnalisation (US2)

Générer deux fois avec les mêmes `data` et `design.primaryColor` différents, puis
`design.showFooter: false`.

Attendu : contenus textuels identiques ; couleur différente ; le pied de page absent quand
`showFooter` vaut `false`.

### S4 — Validation (US1, US2)

```sh
curl -s -H 'Content-Type: application/json' \
  -d '{"data":{"title":"T","items":[{"label":"a","value":-1}]},"design":{"align":"top"}}' \
  localhost:3000/templates/sample/render | jq
```

Attendu : `422`, `code: validation-failed`, deux violations
(`/data/items/0/value`, `/design/align`).

### S5 — Templates à chaud (US4)

```sh
cp -r examples/templates/sample examples/templates/sample-copy
sleep 5 && curl -s localhost:3000/templates | jq '.templates[].id'
echo '{' > examples/templates/sample-copy/schema.json
sleep 5 && curl -s localhost:3000/templates/sample-copy | jq '{status, reason}'
rm -rf examples/templates/sample-copy
```

Attendu : `sample-copy` apparaît (< 5 s, SC-003), puis passe `invalid` avec une raison, puis
disparaît ; `sample` reste `valid` tout du long. (Avec Docker, monter le volume sans `:ro`
côté hôte n'est pas nécessaire : les modifications se font sur l'hôte.)

### S6 — Erreurs et sécurité

| Requête | Attendu |
|---------|---------|
| `GET /templates/inconnu` | `404 template-not-found` |
| `GET /templates/..%2F..%2Fetc` | `404 template-not-found` |
| corps de 6 Mo | `413 payload-too-large` |
| `Content-Type: text/plain` | `415 unsupported-media-type` |
| `data.title = "#import \"/etc/passwd\""` | `200`, texte rendu littéralement |

## 4. Mesures

```sh
cargo bench --bench render                 # SC-001 : p95 < 200 ms (document 1 page, tableau de 20 lignes)
oha -n 2000 -c 20 -m POST -H 'Content-Type: application/json' \
  -d "$(cat examples/requests/sample.json)" \
  http://localhost:3000/templates/sample/render   # SC-007 : 0 erreur à 20 simultanées
docker image ls inkpdf:dev                 # SC-009 : < 100 Mo
```
