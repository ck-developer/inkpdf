# Contrat API — Paquets (002)

Ajouts à l'API V1, décrits dans l'OpenAPI généré et verrouillés par `tests/contract_openapi.rs`.
Les erreurs suivent toujours RFC 9457 (`application/problem+json`, avec un champ `code`).

## `GET /packages`

Liste les paquets Typst **mis à disposition des templates**. Les paquets présents uniquement
comme dépendances ne sont pas listés.

- **Paramètres** : aucun.
- **200** `application/json` :

```json
{
  "packages": [
    {
      "name": "zero",
      "import": "@preview/zero",
      "version": "0.7.1",
      "description": "Precise scientific number and unit formatting.",
      "license": "MIT"
    }
  ]
}
```

| Champ | Type | Contraintes |
|---|---|---|
| `packages` | tableau | trié par `name` ; 21 éléments pour la liste validée ; un seul élément par nom |
| `name` | chaîne | motif `^[a-z0-9][a-z0-9-]*$` |
| `import` | chaîne | `@preview/{name}`, la ligne à écrire dans un template |
| `version` | chaîne | version installée, motif `^\d+\.\d+\.\d+$` (information seulement) |
| `description` | chaîne | peut être vide |
| `license` | chaîne | identifiant ou expression SPDX |

La réponse est constante pour un binaire donné.

## Diagnostics de paquet (routes existantes)

### Chargement d'un template : `GET /templates`, `GET /templates/{id}`

Un template dont un import est incorrect a le statut `invalid`. Sa raison suit le format de
[template-imports.md](./template-imports.md), par exemple :

```
main.typ:3: remove the version: write @preview/zero (inkpdf uses its installed version)
```

### Rendu : `POST /templates/{id}/render`

- Si le template est invalide à cause d'un import, la réponse est inchangée par rapport à la V1 :
  **409** `template-invalid`.
- Un import construit par calcul n'est pas pris en charge : **500** `render-failed`, avec
  l'erreur de Typst (`package specification is missing version`).
- Une erreur dans le code d'un paquet donne **500** `render-failed`. Le champ `file` est alors
  préfixé par la référence du paquet :

```json
{ "message": "…", "file": "@preview/cetz:0.5.2/src/draw.typ", "line": 120 }
```

Aucun nouveau code d'erreur n'est introduit.
