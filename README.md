# inkpdf

Service de génération de PDF à partir de templates [Typst](https://typst.app), sans navigateur.

inkpdf est **générique et agnostique du contenu** : il ne connaît aucun type de document. Un
template est un dossier (`main.typ` + `schema.json` + ressources) déposé dans un volume ;
l'appelant envoie un JSON `{ "data": …, "design": … }`, validé contre le schéma du template,
puis injecté dans Typst comme données (`sys.inputs`) et compilé en PDF par le moteur Typst
embarqué dans le binaire.

- Un binaire Rust unique, aucun sous-processus ni navigateur.
- Templates découverts **à chaud** (ajout, modification, suppression sans redémarrage).
- Bac à sable : ni réseau, ni lecture hors du dossier du template.
- Paquets Typst intégrés (QR codes, codes-barres, graphiques, nombres, dates…) : importés par
  leur seul nom (`#import "@preview/zero"`), jamais téléchargés ([docs/packages.md](docs/packages.md)).
- API REST auto-descriptive : OpenAPI 3.1 servi sur `/openapi.json`, documentation sur `/docs`.
- PDF déterministes : mêmes template et corps ⇒ mêmes octets.

## Démarrage

Avec Docker :

```sh
docker run --rm -p 3000:3000 \
  -v "$PWD/examples/templates:/templates:ro" \
  ghcr.io/ck-developer/inkpdf

curl -s -H 'Content-Type: application/json' \
  -d @examples/requests/sample.json \
  -o sample.pdf localhost:3000/templates/sample/render
```

Avec Docker Compose (image construite depuis les sources, idéal pour tester) :

```sh
docker compose up --build                       # http://localhost:3000/docs
INKPDF_TEMPLATES=./mes-templates docker compose up --build   # autre dossier de templates
INKPDF_PORT=8080 docker compose up --build      # autre port
```

Les templates du dossier monté sont rechargés à chaud : copiez-y un dossier de template et il
apparaît dans `GET /templates` en quelques secondes.

En local (Rust 1.98+) :

```sh
INKPDF_TEMPLATES_DIR=examples/templates cargo run --release
```

## Configuration

| Variable | Défaut | Rôle |
|----------|--------|------|
| `INKPDF_TEMPLATES_DIR` | `/templates` | volume des templates |
| `INKPDF_LISTEN` | `0.0.0.0:3000` | adresse d'écoute |
| `INKPDF_MAX_BODY_BYTES` | `5242880` | taille max du corps d'une requête |
| `INKPDF_RENDER_TIMEOUT_SECS` | `30` | durée max d'un rendu (`504` au-delà) |
| `INKPDF_MAX_CONCURRENT_RENDERS` | nb de CPU | rendus simultanés |
| `INKPDF_QUEUE_TIMEOUT_SECS` | `10` | attente max d'un créneau de rendu (`503` au-delà) |
| `INKPDF_RESCAN_INTERVAL_SECS` | `2` | rescan de secours du volume |
| `INKPDF_MAX_TEMPLATE_BYTES` | `52428800` | taille max d'un template (chargé en mémoire) |
| `INKPDF_LOG_FORMAT` | `json` | `json` ou `pretty` |
| `RUST_LOG` | `info` | niveau de logs |

Une valeur mal formée empêche le démarrage avec un message explicite.

## API

| Méthode | Chemin | Rôle |
|---------|--------|------|
| `GET` | `/templates` | liste des templates (`valid` / `invalid` avec `reason`) |
| `GET` | `/templates/{templateId}` | détail d'un template, schéma compris |
| `GET` | `/templates/{templateId}/schema` | `schema.json` brut (`application/schema+json`) |
| `POST` | `/templates/{templateId}/render` | génération : corps `{ data, design }` → `application/pdf` |
| `GET` | `/packages` | paquets Typst disponibles pour les templates |
| `GET` | `/health` | vivacité |
| `GET` | `/ready` | disponibilité (scan initial terminé) |
| `GET` | `/openapi.json` | description OpenAPI 3.1 |
| `GET` | `/docs` | documentation interactive |

Les erreurs suivent la RFC 9457 (`application/problem+json`) avec un champ `code` :
`template-not-found` (404), `invalid-json` (400), `unsupported-media-type` (415),
`validation-failed` (422, avec `violations[]`), `payload-too-large` (413),
`template-invalid` (409), `render-failed` (500, avec `diagnostics[]`), `render-timeout` (504),
`overloaded` (503).

Le service ne gère ni authentification ni utilisateurs : il est destiné à un réseau privé.

## Écrire un template

Voir [docs/templates.md](docs/templates.md) et l'exemple neutre
[`examples/templates/sample`](examples/templates/sample) ; utilisation des paquets intégrés :
[`examples/templates/packages-demo`](examples/templates/packages-demo).

## Développement

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --release --test perf -- --ignored   # garde-fou de latence (p95 < 200 ms)
cargo bench --bench render
INKPDF_UPDATE_OPENAPI=1 cargo test --test contract_openapi   # régénère openapi/openapi.json
scripts/add-package.sh <nom> <version>   # ajoute ou change un paquet Typst intégré
```

Le document OpenAPI est généré depuis le code ; `tests/contract_openapi.rs` vérifie qu'il est
identique au fichier versionné `openapi/openapi.json`.

### Limite connue

Typst n'offre pas d'annulation : une compilation qui dépasse son délai est interrompue au
prochain accès du template à un fichier ou à une police. Une boucle purement calculatoire
continue d'occuper un créneau de rendu jusqu'à sa fin (signalée par le log `render.overrun`) ;
l'appelant reçoit néanmoins `504` dans le délai et les autres rendus restent bornés par
`INKPDF_MAX_CONCURRENT_RENDERS`. Il en va de même pour les plugins WASM de certains paquets
intégrés (signalés dans [docs/packages.md](docs/packages.md)) : un appel de plugin ne peut pas
être interrompu avant sa fin.

En build debug (`cargo test`), l'abandon d'une compilation annulée se manifeste par un message
`comemo: found differing return values` sur le thread de rendu : l'assertion de pureté de
`comemo` n'existe qu'en debug ; le créneau est libéré normalement. En release, la compilation
se termine simplement en erreur.

## Licence

TODO(LICENSE) — `MIT OR Apache-2.0` proposé, à trancher avant la première publication.
