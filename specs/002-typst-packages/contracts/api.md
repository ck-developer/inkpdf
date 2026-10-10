# Contrat API — Paquets (002)

Ajouts à l'API V1, décrits dans l'OpenAPI généré et verrouillés par `tests/contract_openapi.rs`.
Les erreurs suivent toujours RFC 9457 (`application/problem+json`, avec un champ `code`).

## `GET /packages`

Liste les paquets Typst intégrés au service.

- **Paramètres** : aucun.
- **200** `application/json` :

```json
{
  "packages": [
    {
      "namespace": "preview",
      "name": "zero",
      "version": "0.7.1",
      "import": "@preview/zero:0.7.1",
      "description": "Precise scientific number and unit formatting.",
      "license": "MIT",
      "role": "selected"
    }
  ]
}
```

| Champ | Type | Contraintes |
|---|---|---|
| `packages` | tableau | trié par `name`, puis par `version` (ordre SemVer) ; 29 éléments pour la liste validée |
| `namespace` | chaîne | `"preview"` |
| `name` | chaîne | motif `^[a-z0-9][a-z0-9-]*$` |
| `version` | chaîne | motif `^\d+\.\d+\.\d+$` |
| `import` | chaîne | `@{namespace}/{name}:{version}` |
| `description` | chaîne | peut être vide |
| `license` | chaîne | identifiant ou expression SPDX |
| `role` | énumération | `selected` \| `dependency` |

La réponse est constante pour un binaire donné et ne dépend pas du volume de templates.

## Diagnostics de paquet (routes existantes)

### Chargement d'un template : `GET /templates`, `GET /templates/{id}`

Un template qui importe un paquet non intégré a le statut `invalid`. Sa raison suit le format
de [template-imports.md](./template-imports.md), par exemple :

```
main.typ:3: package @preview/zero:0.5.0 is not bundled with inkpdf (available versions: 0.6.1, 0.7.1)
```

### Rendu : `POST /templates/{id}/render`

- Si le template est invalide à cause d'un import, la réponse est inchangée par rapport à la V1 :
  **409** `template-invalid`.
- Un import non résolu au rendu (import calculé dynamiquement) donne **500** `render-failed`.
  Le diagnostic a la forme suivante :

```json
{ "message": "package @local/x:1.0.0 is not bundled with inkpdf (no version of this package is bundled; see GET /packages)",
  "file": "main.typ", "line": 4 }
```

- Une erreur dans le code d'un paquet donne **500** `render-failed`. Le champ `file` est alors
  préfixé par la référence du paquet :

```json
{ "message": "…", "file": "@preview/cetz:0.5.2/src/draw.typ", "line": 120 }
```

Aucun nouveau code d'erreur n'est introduit.
