# Contrat — Utiliser un paquet dans un template

Ce contrat s'adresse aux **auteurs de templates**. Il complète le contrat
`specs/001-pdf-generation-service/contracts/template-format.md`, dont la ligne « `#import
"@preview/..."` → échec » est **remplacée** par les règles ci-dessous.

## Règles

1. **Un paquet s'importe par son seul nom** : `#import "@preview/zero"`, éventuellement suivi de
   `: a, b` ou de `as x`. `#include "@preview/<nom>"` suit la même règle.
2. **Pas de version.** Le service utilise la version qu'il a installée, visible dans
   `GET /packages`. Un import qui écrit une version (`@preview/zero:0.7.1`) est refusé.
3. **Seuls les paquets mis à disposition** sont importables (liste : `GET /packages` ou
   `docs/packages.md`). Les paquets présents uniquement comme dépendances d'autres paquets ne
   le sont pas. Seul le namespace `@preview` existe.
4. **Écrit tel quel** : l'import est une chaîne écrite directement. Un import construit par
   calcul (`"@preview/" + nom`) n'est pas pris en charge.
5. **Fichiers** : un paquet ne lit que ses propres fichiers. Pour lui passer une image ou des
   données du template, transmettez-les explicitement : `logo: image("assets/logo.png")`,
   `read("data.csv")` ou `path("assets/logo.png")`.
6. **Polices** : les polices livrées dans un paquet ne sont pas chargées. Utilisez les polices
   du service ou celles du dossier `fonts/` du template.
7. Un dossier `packages/` placé dans un template n'est **pas** utilisé.
8. Un template qui suit ces règles ne se compile pas tel quel avec l'outil Typst standard, qui
   exige une version. Il est fait pour inkpdf.

## Contrôle au chargement

Les imports sont résolus **une fois**, au chargement du template (dépôt ou modification), et
jamais à chaque génération. En cas d'erreur, le template passe à `invalid`. La raison contient
une ligne par import fautif, au format `<fichier>:<ligne>: <message>` :

| Situation | Message |
|---|---|
| Paquet non disponible | `package @preview/foo is not available in inkpdf (see GET /packages)` |
| Version écrite | `remove the version: write @preview/zero (inkpdf uses its installed version)` |
| Autre namespace | `only @preview packages are available: @local/x` |
| Import mal formé | `invalid package import "@preview/"` |

Un import construit par calcul n'est pas visible au chargement. La génération échoue alors avec
l'erreur de Typst (`package specification is missing version`) ; la documentation renvoie à la
règle 4.
