# Contrat — Utiliser un paquet dans un template

Ce contrat s'adresse aux **auteurs de templates**. Il complète le contrat
`specs/001-pdf-generation-service/contracts/template-format.md`, dont la ligne « `#import
"@preview/..."` → échec » est **remplacée** par les règles ci-dessous.

## Règles

1. **Syntaxe** : `#import "@preview/<nom>"` (recommandé) ou `#import "@preview/<nom>:<version>"`,
   éventuellement suivi de `: a, b` ou de `as x`. `#include` est également accepté.
2. **Version** :
   - **Sans version** (`@preview/zero`) : la **version par défaut** est utilisée, c'est-à-dire
     la plus récente version intégrée (champ `default` de `GET /packages`). Le template suit
     les mises à jour du service.
   - **Avec une version exacte** (`@preview/zero:0.7.1`) : cette version est utilisée, même si
     une version plus récente est intégrée ensuite. C'est utile pour figer un rendu.
   - **Version partielle** (`@preview/zero:0.7`) : refusée.

   Un template qui omet la version ne se compile pas tel quel avec l'outil Typst standard,
   qui exige une version.
3. **Seuls les paquets intégrés** sont disponibles (liste : `GET /packages` ou
   `docs/packages.md`). Aucun téléchargement n'a lieu, quel que soit le namespace.
4. **Fichiers** : un paquet ne lit que ses propres fichiers. Pour lui passer une image ou des
   données du template, transmettez-les explicitement, par exemple
   `logo: image("assets/logo.png")`, `read("data.csv")` ou `path("assets/logo.png")`.
5. **Polices** : les polices éventuellement livrées dans un paquet ne sont pas chargées.
   Utilisez les polices du service ou celles du dossier `fonts/` du template.
6. Un dossier `packages/` placé dans un template n'est **pas** utilisé pour résoudre les imports.

## Contrôle au chargement

Les imports écrits littéralement dans les fichiers `.typ` du template sont vérifiés dès le
dépôt. En cas d'erreur, le template passe à `invalid`, avec une raison composée d'une ligne
par import fautif :

```
<fichier>:<ligne>: <message>
```

| Situation | Message |
|---|---|
| Paquet absent | `package @preview/foo:1.0.0 is not bundled with inkpdf (no version of this package is bundled; see GET /packages)` |
| Version absente | `package @preview/zero:0.5.0 is not bundled with inkpdf (available versions: 0.6.1, 0.7.1)` |
| Autre namespace | `package @local/x:1.0.0 is not bundled with inkpdf (…)` |
| Paquet sans version et absent | `package @preview/foo is not bundled with inkpdf (see GET /packages)` |
| Version partielle | message du parseur Typst, par exemple `version number is missing patch version` |

Un import construit dynamiquement (par exemple `import ("@preview/" + nom + ":1.0.0")`) n'est
pas visible au chargement ; il est contrôlé au rendu, avec le même message.

Un import placé dans du code jamais exécuté est tout de même contrôlé.
