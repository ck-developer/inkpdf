# Écrire un template inkpdf

Ce guide s'adresse aux auteurs de templates. Le format décrit ici est un contrat versionné avec
l'API : toute rupture est une rupture majeure du service.

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

- L'identifiant du template est le nom de son dossier. Un dossier dont le nom ne respecte pas
  le format est ignoré (et journalisé au démarrage).
- Les fichiers et dossiers cachés (préfixe `.`) sont ignorés.
- Les liens symboliques sont suivis tant que leur cible reste dans le dossier du template ;
  un lien qui en sort est exclu (cas des ConfigMaps Kubernetes : les liens vers `..data`
  restent dans le dossier et sont donc suivis).
- La taille totale d'un template est limitée par `INKPDF_MAX_TEMPLATE_BYTES` (50 Mo par
  défaut) ; au-delà, le template est signalé `invalid`.
- `main.typ` peut inclure ou importer d'autres fichiers du même dossier
  (`#include "parts/footer.typ"`, `#image("assets/logo.png")`).

Un template est entièrement chargé en mémoire au moment où il est (re)découvert ; un rendu ne
lit jamais le disque et voit donc toujours une version complète et cohérente du template.

## `template.json`

```json
{
  "name": "Exemple",
  "description": "Titre et tableau de libellés/valeurs ; démonstration du format.",
  "version": "1.0.0"
}
```

| Champ | Type | Requis | Défaut |
|-------|------|--------|--------|
| `name` | chaîne (1–120) | non | identifiant du dossier |
| `description` | chaîne (≤ 2000) | non | absent |
| `version` | chaîne (≤ 64) | non | absent (SemVer recommandé) |

Toute autre clé rend le template invalide (détection des fautes de frappe).

## `schema.json`

Le corps d'une génération a deux sections :

- `data` (requise) : le contenu du document, entièrement défini par vous ;
- `layout` (optionnelle) : les réglages d'apparence (couleur, alignement, blocs affichés…).

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
    "layout": {
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

Règles :

1. La racine DOIT être `type: object` et déclarer `properties.data`.
2. Seules `data` et `layout` sont admises à la racine. Si `additionalProperties` est absent à
   la racine, le service le considère comme `false` (sans modifier le schéma exposé par
   `GET /templates/{id}/schema`, qui renvoie votre fichier octet pour octet).
3. Les propriétés de `layout` DEVRAIENT avoir un `default`. Avant la validation, le service
   initialise `layout` à `{}` s'il est absent et insère les défauts manquants, récursivement
   dans les sous-objets de `layout`. Les défauts ne sont **pas** appliqués à `data`.
4. Seules les références `$ref` internes (`#/...`, `$defs`) sont résolues ; une référence
   externe rend le template invalide.
5. Pour des valeurs décimales exactes, préférez des entiers (centimes) ou des chaînes : les
   nombres décimaux sont transmis à Typst comme flottants.

Un corps non conforme est rejeté (`422 validation-failed`) avec la liste de toutes les
violations, sans que Typst ne soit invoqué.

## Lire les données dans `main.typ`

L'entrée validée (défauts appliqués) est disponible dans `sys.inputs` :

```typst
#let data = sys.inputs.data
#let layout = sys.inputs.layout

#set text(fill: rgb(layout.primaryColor))
#let aligns = (left: left, center: center, right: right)
#align(aligns.at(layout.align))[= #data.title]
#table(columns: 2, ..data.items.map(i => (i.label, str(i.value))).flatten())
#if layout.showFooter [ #include "parts/footer.typ" ]
```

Si le schéma ne déclare pas `layout`, `sys.inputs.layout` est un dictionnaire vide.

| JSON | Typst |
|------|-------|
| objet | dictionnaire |
| tableau | tableau |
| chaîne | `str` |
| entier | `int` (au-delà de la plage 64 bits : `float`) |
| nombre décimal | `float` |
| booléen | `bool` |
| `null` | `none` |

Les chaînes ne sont **jamais** interprétées comme du code Typst : un titre
`#import "/etc/passwd"` est affiché tel quel.

## Utiliser un paquet

inkpdf intègre une sélection de paquets Typst Universe (QR codes, codes-barres, graphiques,
formatage des nombres et des dates…) : liste dans [packages.md](./packages.md) ou via
`GET /packages`.

```typst
#import "@preview/zero": num
#import "@preview/tiaoma"

Montant : #num(sys.inputs.data.amount, digits: 2, decimal-separator: ",")
#tiaoma.qrcode(sys.inputs.data.reference)
```

Règles :

1. **Un paquet s'importe par son seul nom** : `#import "@preview/<nom>"` (avec `: a, b` ou
   `as x` si besoin) ; `#include "@preview/<nom>"` suit la même règle.
2. **Pas de version** : le service utilise celle qu'il a installée (visible dans
   `GET /packages`). `@preview/zero:0.7.1` est refusé.
3. **Seuls les paquets mis à disposition** sont importables ; seul le namespace `@preview`
   existe.
4. **Écrit tel quel** : `"@preview/" + nom` (import calculé) n'est pas pris en charge.
5. **Fichiers** : un paquet ne lit que ses propres fichiers. Pour lui passer une image ou des
   données du template, transmettez-les : `image("assets/logo.png")`, `read("data.csv")` ou
   `path("assets/logo.png")`.
6. **Polices** : celles d'un paquet ne sont pas chargées ; utilisez celles du service ou de
   `fonts/`.
7. Un dossier `packages/` dans un template n'est pas utilisé.
8. Un template inkpdf ne se compile pas tel quel avec l'outil `typst` standard, qui exige une
   version.

Les imports sont vérifiés **une fois, au dépôt** du template. Un import incorrect le rend
`invalid`, avec une ligne par erreur dans `reason` :

```
main.typ:3: remove the version: write @preview/zero (inkpdf uses its installed version)
main.typ:4: package @preview/foo is not available in inkpdf (see GET /packages)
```

Une erreur à l'intérieur d'un paquet est signalée avec un fichier préfixé par le paquet, par
exemple `@preview/zero:0.7.1/src/num.typ`.

Exemple complet : [`examples/templates/packages-demo`](../examples/templates/packages-demo).

## Restrictions du bac à sable

| Interdit | Comportement |
|----------|--------------|
| paquet non mis à disposition, version écrite, autre namespace | template `invalid` (`409 template-invalid`) |
| lecture hors du dossier (`../`, chemin absolu, lien sortant) | échec (`500 render-failed`) |
| accès réseau, variables d'environnement, polices système | indisponibles |

Les erreurs de compilation sont renvoyées dans `diagnostics[]` avec le fichier (relatif au
dossier du template), la ligne, la colonne et les indications de Typst.

## Polices

Polices toujours disponibles (embarquées dans le binaire) : Libertinus Serif,
New Computer Modern, DejaVu Sans Mono. Ajoutez les vôtres dans `fonts/` (TTF, OTF, TTC) ; un
fichier de police illisible rend le template invalide.

## Déterminisme

Deux générations avec le même template (inchangé) et le même corps produisent des PDF
identiques à l'octet près. Exception : `datetime.today()` est autorisé mais rend le document
dépendant du jour de génération ; passez plutôt la date dans `data`.

## Publication et mise à jour

Copiez le dossier dans le volume : il est pris en compte en quelques secondes, sans
redémarrage. Une modification n'est appliquée que lorsque le dossier est resté stable une
seconde ; pendant une copie, la version précédente continue d'être servie. Un template
invalide est listé avec `status: invalid` et sa `reason`, sans affecter les autres.
