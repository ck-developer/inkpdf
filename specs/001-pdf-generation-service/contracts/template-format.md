# Contrat : format d'un template inkpdf (V1)

Ce document est le contrat entre les **auteurs de templates** et le service. Il est versionné
avec l'API : toute rupture de ce format est une rupture MAJOR du service.

## Arborescence

```text
<volume>/                       # INKPDF_TEMPLATES_DIR, monté en lecture seule
└── sample/                     # identifiant du template : ^[a-z0-9][a-z0-9_-]{0,63}$
    ├── main.typ                # REQUIS — point d'entrée Typst
    ├── schema.json             # REQUIS — JSON Schema (draft 2020-12) de l'entrée
    ├── template.json           # optionnel — métadonnées
    ├── fonts/                  # optionnel — .ttf / .otf / .ttc, chargées automatiquement
    └── assets/                 # optionnel — images et fichiers lus par main.typ
```

- Les dossiers dont le nom ne respecte pas le format d'identifiant sont ignorés (et journalisés).
- Les fichiers et dossiers cachés (préfixe `.`) sont ignorés, sauf la cible des liens
  `..data` créés par les ConfigMaps Kubernetes, qui sont suivis s'ils restent dans le volume.
- La taille totale d'un dossier de template est limitée (`INKPDF_MAX_TEMPLATE_BYTES`, défaut
  50 Mo) ; au-delà, le template est signalé `invalid`.
- `main.typ` peut importer d'autres fichiers `.typ` du même dossier (`#include "parts/footer.typ"`).

## `template.json` (manifeste)

```json
{
  "name": "Exemple",
  "description": "Titre et tableau de libellés/valeurs ; démonstration du format.",
  "version": "1.0.0"
}
```

| Champ | Type | Requis | Défaut |
|-------|------|--------|--------|
| `name` | string (1–120) | non | identifiant du dossier |
| `description` | string (≤ 2000) | non | absent |
| `version` | string (≤ 64) | non | absent |

Toute autre clé rend le template invalide (détection des fautes de frappe).

## `schema.json`

Racine obligatoire :

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["data"],
  "additionalProperties": false,
  "properties": {
    "data": {
      "type": "object",
      "required": ["title", "items"],
      "properties": {
        "title": { "type": "string", "minLength": 1 },
        "items": {
          "type": "array",
          "minItems": 1,
          "items": {
            "type": "object",
            "required": ["label", "value"],
            "properties": {
              "label": { "type": "string" },
              "value": { "type": "number", "minimum": 0 }
            }
          }
        }
      }
    },
    "design": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "primaryColor": { "type": "string", "pattern": "^#[0-9a-fA-F]{6}$", "default": "#1f4e79" },
        "align":        { "enum": ["left", "center", "right"], "default": "left" },
        "showFooter":   { "type": "boolean", "default": true }
      }
    }
  }
}
```

Le contenu de `data` et `design` est entièrement libre : le service n'en connaît rien et se
contente de le valider puis de le transmettre. L'exemple ci-dessus n'est qu'une illustration.

Règles :

1. La racine DOIT être `type: object` et déclarer `properties.data`.
2. Seules `data` et `design` sont admises à la racine ; si `additionalProperties` est absent à
   la racine, le service le considère comme `false`.
3. Les propriétés de `design` DEVRAIENT avoir un `default` ; le service applique les défauts
   (récursivement sur les sous-objets de `design`) **avant** la validation.
4. Seules les références `$ref` internes (`#/...`, `$defs`) sont résolues ; une référence
   externe rend le template invalide.
5. Pour des valeurs décimales exactes, préférer des entiers ou des chaînes : les nombres
   flottants sont transmis tels quels à Typst.

## Accès aux données depuis `main.typ`

Le service fournit l'entrée (défauts appliqués, validée) dans `sys.inputs` :

```typst
#let data = sys.inputs.data
#let design = sys.inputs.design

#set text(fill: rgb(design.primaryColor))
#let aligns = (left: left, center: center, right: right)
#align(aligns.at(design.align))[= #data.title]
#table(columns: 2, ..data.items.map(i => (i.label, str(i.value))).flatten())
#if design.showFooter [ #include "parts/footer.typ" ]
```

Correspondance des types : objet → dictionnaire, tableau → tableau, chaîne → `str`,
entier → `int`, nombre décimal → `float`, booléen → `bool`, `null` → `none`.
Les chaînes ne sont **jamais** interprétées comme du code Typst.

## Restrictions du bac à sable

| Interdit | Comportement |
|----------|--------------|
| `#import "@preview/..."` ou tout paquet | échec de génération (`render-failed`) |
| lecture hors du dossier du template (`../`, chemin absolu, lien sortant) | échec (`render-failed`) |
| accès réseau, variables d'environnement, polices système | indisponibles |
| `datetime.today()` | autorisé, mais rend le PDF non déterministe ; préférer une date dans `data` |

Polices disponibles : polices embarquées du service (familles Libertinus Serif, New Computer
Modern, DejaVu Sans Mono) + celles du dossier `fonts/` du template.
