# Data Model: Service de génération de PDF à partir de templates (V1)

**Feature**: `001-pdf-generation-service` | **Date**: 2026-10-09

Le service est sans état : aucune base de données. Toutes les entités ci-dessous vivent en
mémoire et sont dérivées du volume de templates ou d'une requête. Le format sur disque d'un
template est défini dans [contracts/template-format.md](./contracts/template-format.md) ; la
forme exposée par l'API est définie dans [contracts/openapi.yaml](./contracts/openapi.yaml).

## TemplateId

Identifiant d'un template = nom de son dossier dans le volume.

| Règle | Détail |
|-------|--------|
| Format | `^[a-z0-9][a-z0-9_-]{0,63}$` |
| Hors format | traité comme « introuvable » (`404 template-not-found`) sans accès disque |
| Unicité | garantie par le système de fichiers |

## Template (entrée du registre)

| Champ | Type | Origine | Notes |
|-------|------|---------|-------|
| `id` | TemplateId | nom du dossier | |
| `name` | texte | `template.json` → `name` | défaut : `id` |
| `description` | texte, optionnel | `template.json` → `description` | |
| `version` | texte, optionnel | `template.json` → `version` | libre (SemVer recommandé) |
| `status` | TemplateStatus | calculé au chargement | voir ci-dessous |
| `schema` | JSON Schema | `schema.json` | octets d'origine, exposés tels quels (constitution VI) |
| `validator` | validateur compilé | dérivé de `schema` (+ `additionalProperties: false` à la racine si absent) | interne, non exposé |
| `files` | instantané chemin relatif → octets | tous les fichiers du dossier | interne ; seule source lue par le rendu (jamais le disque) |
| `fonts` | liste de polices | `fonts/**` (depuis `files`) | interne |
| `fingerprint` | empreinte | (chemin, taille, mtime) de tous les fichiers | interne, sert à détecter les changements |
| `loaded_at` | horodatage | moment du (re)chargement | exposé dans le détail |

**Relations** : un Template possède exactement un Schéma ; il est référencé par N Requêtes de
génération (aucune persistance de ce lien).

### Règles de validité (au chargement)

Un template est **valide** si et seulement si :

1. `main.typ` existe et est lisible en UTF-8 ;
2. `schema.json` existe, est un JSON valide et un JSON Schema valide (draft 2020-12) ;
3. la racine du schéma est `type: object`, déclare `properties.data`, et ses seules propriétés
   autorisées sont `data` et `design` (`additionalProperties: false` appliqué par le validateur
   s'il est absent, sans modifier le schéma exposé) ;
4. `template.json`, s'il existe, est un JSON valide conforme au format du manifeste ;
5. chaque police de `fonts/` est lisible (une police illisible rend le template invalide) ;
6. la taille totale des fichiers du dossier ne dépasse pas `INKPDF_MAX_TEMPLATE_BYTES`
   (défaut 50 Mo) ; vérifiée avant toute lecture, à partir de l'empreinte.

Le fichier Typst n'est **pas** compilé au chargement (une compilation nécessite des données) ;
ses erreurs apparaissent à la génération (`500 render-failed`).

## TemplateStatus

Valeurs : `valid` | `invalid` (avec `reason`, texte destiné à l'auteur du template).

### Transitions (registre)

```text
              dépôt d'un dossier (empreinte stable sur 2 observations)
   (absent) ───────────────────────────────────────────────▶ chargement
                                                              │
                                ┌─────────── règles OK ───────┴──── règles KO ──────────┐
                                ▼                                                       ▼
                             valid ◀──── modification (empreinte stable) + règles OK ── invalid
                                │  ───── modification (empreinte stable) + règles KO ──▶  │
                                │                                                        │
                                └───────────── suppression du dossier ───────────────────┴──▶ (absent)
```

- Le chargement lit tous les fichiers en mémoire puis recalcule l'empreinte ; si elle a changé
  pendant la lecture, le chargement est abandonné et retenté au tour suivant.
- Pendant qu'une modification est en cours (empreinte instable), la **version précédente**
  reste servie.
- Le passage `valid → invalid` ne retire pas le template de la liste ; il devient non
  utilisable pour générer (`409 template-invalid`).

## Schéma de template

JSON Schema dont la racine est un objet à deux sections :

| Section | Requise dans la requête | Contenu |
|---------|------------------------|---------|
| `data` | oui | données métier, libres, décrites par le template |
| `design` | non | paramètres d'apparence ; chaque propriété DEVRAIT avoir un `default` |

Règles d'application des défauts (avant validation) :

- si `design` est absent de la requête, il est initialisé à `{}` ;
- pour chaque propriété de `properties.design.properties` absente et munie d'un `default`, la
  valeur par défaut est insérée ; la règle s'applique récursivement aux sous-objets ;
- les défauts ne sont pas appliqués à `data` (les données métier sont fournies explicitement).

## Requête de génération (GenerationRequest)

| Champ | Type | Règles |
|-------|------|--------|
| `templateId` | TemplateId | chemin de l'URL |
| corps | objet JSON `{ data, design? }` | `Content-Type: application/json` ; taille ≤ `INKPDF_MAX_BODY_BYTES` |

Cycle de traitement :

```text
reçue → (taille OK ?) → (JSON valide ?) → (template existe ? valide ?) → défauts appliqués
      → (schéma OK ?) → en file (permis) → compilation → export PDF → réponse
```

Chaque flèche en échec produit l'erreur correspondante (voir ci-dessous) ; aucune étape
postérieure n'est exécutée.

## Document généré

| Attribut | Valeur |
|----------|--------|
| Type | `application/pdf` |
| Nom suggéré | `Content-Disposition: inline; filename="<templateId>.pdf"` |
| Conservation | aucune (renvoyé puis oublié) |
| Déterminisme | identique à l'octet pour mêmes template (empreinte) + corps (hors `datetime.today()`) |

## Erreur (Problem Details, RFC 9457)

| Champ | Type | Notes |
|-------|------|-------|
| `type` | URI | `https://github.com/ck-developer/inkpdf/errors/<code>` |
| `title` | texte | libellé stable du code |
| `status` | entier | code HTTP |
| `detail` | texte | explication lisible |
| `code` | texte | code machine (tableau ci-dessous) |
| `templateId` | texte, optionnel | quand applicable |
| `violations` | liste, optionnel | pour `validation-failed` |
| `diagnostics` | liste, optionnel | pour `render-failed` : message, fichier, ligne, colonne |

### Violation

| Champ | Type | Exemple |
|-------|------|---------|
| `path` | JSON Pointer dans la requête | `/data/lines/2/quantity` |
| `schemaPath` | JSON Pointer dans le schéma | `/properties/data/properties/lines/items/properties/quantity/minimum` |
| `message` | texte | `-1 is less than the minimum of 0` |

### Codes d'erreur (FR-019)

| Code | HTTP | Origine | Cas |
|------|------|---------|-----|
| `template-not-found` | 404 | appelant | identifiant inconnu ou hors format |
| `invalid-json` | 400 | appelant | corps non JSON ou sans objet racine |
| `unsupported-media-type` | 415 | appelant | `Content-Type` différent de `application/json` |
| `validation-failed` | 422 | appelant | corps non conforme au schéma |
| `payload-too-large` | 413 | appelant | corps > limite |
| `template-invalid` | 409 | template | template présent mais invalide (`reason` dans `detail`) |
| `render-failed` | 500 | template/interne | erreur de compilation ou d'export |
| `render-timeout` | 504 | interne | durée max dépassée |
| `overloaded` | 503 | interne | aucun créneau de rendu libre dans le délai d'attente |
