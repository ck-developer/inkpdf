# Paquets Typst intégrés

inkpdf intègre une liste fixe de paquets [Typst Universe](https://typst.app/universe) : ils font
partie du binaire, aucun téléchargement n'a jamais lieu. La liste est aussi disponible via
`GET /packages`.

## Utilisation dans un template

Un paquet s'importe **par son seul nom**, sans version : le service utilise la version qu'il a
installée.

```typst
#import "@preview/zero": num
#import "@preview/tiaoma"

Montant : #num("1234.56", decimal-separator: ",")
#tiaoma.qrcode("INV-2026-0042")
```

Règles complètes : [templates.md](./templates.md#utiliser-un-paquet).

## Paquets disponibles

| Import | Ce qu'il fait | Version | Licence | WASM |
|---|---|---|---|---|
| `@preview/cetz` | Bibliothèque de dessin : formes, schémas et graphiques. | 0.5.2 | LGPL-3.0-or-later | oui |
| `@preview/cetz-plot` | Trace des courbes et des graphiques à barres ou camemberts. | 0.1.4 | LGPL-3.0-or-later |  |
| `@preview/datify` | Formate les dates dans toutes les langues. | 1.3.0 | MIT |  |
| `@preview/framefit` | Ajuste la taille du texte pour qu'il tienne dans un cadre. | 0.1.0 | MIT |  |
| `@preview/frogst` | Écrit les nombres en toutes lettres en français (montants en lettres). | 1.0.0 | MIT |  |
| `@preview/ibanator` | Vérifie et met en forme les numéros IBAN. | 0.1.0 | EUPL-1.2 | oui |
| `@preview/lilaq` | Graphiques de données (courbes, barres, nuages de points) de qualité. | 0.6.0 | MIT |  |
| `@preview/linguify` | Charge les textes traduits selon la langue du document. | 0.5.0 | MIT | oui |
| `@preview/modern-mailmerge` | Publipostage : lettres, attestations, étiquettes, badges, enveloppes. | 0.1.0 | MIT |  |
| `@preview/oxifmt` | Formate textes et nombres (décimales, séparateurs, alignement). | 1.0.0 | MIT OR Apache-2.0 |  |
| `@preview/payqr-swiss` | Bulletin de paiement suisse QR-facture. | 0.5.0 | LGPL-3.0-only |  |
| `@preview/primaviz` | Trace plus de 50 types de graphiques (barres, courbes, camemberts…) sans dépendance. | 0.11.0 | MIT |  |
| `@preview/qrypst` | Dessine des QR codes avec une taille d'impression exacte. | 0.1.1 | Unlicense AND MIT | oui |
| `@preview/sepay` | Génère le QR code de virement SEPA (EPC) pour les factures. | 0.1.1 | MIT |  |
| `@preview/showybox` | Crée des encadrés colorés et personnalisables. | 2.0.4 | MIT |  |
| `@preview/tablem` | Écrit des tableaux simplement, comme en Markdown. | 0.3.0 | MIT |  |
| `@preview/tabut` | Affiche des données sous forme de tableau. | 1.0.2 | MIT |  |
| `@preview/tiaoma` | Génère codes-barres et QR codes de nombreux formats. | 0.3.0 | MIT | oui |
| `@preview/zebra` | Génère QR codes et Data Matrix en dessin natif. | 0.1.0 | MIT | oui |
| `@preview/zero` | Formate précisément nombres et unités (séparateurs, arrondis). | 0.7.1 | MIT |  |

**WASM** : le paquet contient un plugin compilé. Il s'exécute dans le moteur, sans réseau ni
accès aux fichiers, mais un calcul très long ne peut pas être interrompu avant sa fin : la
réponse part en `504` au délai prévu, et le créneau de rendu reste occupé jusqu'à la fin réelle
du calcul (voir README, limites).

## Licences

Tous les paquets intégrés, y compris les dépendances internes (non importables par les
templates). Le texte de chaque licence est conservé dans le binaire avec le paquet.

| Paquet | Version | Rôle | Licence |
|---|---|---|---|
| cetz | 0.5.2 | mis à disposition | LGPL-3.0-or-later |
| cetz-plot | 0.1.4 | mis à disposition | LGPL-3.0-or-later |
| datify | 1.3.0 | mis à disposition | MIT |
| datify-core | 2.1.0 | dépendance | MIT |
| elembic | 1.1.1 | dépendance | MIT OR Apache-2.0 |
| framefit | 0.1.0 | mis à disposition | MIT |
| frogst | 1.0.0 | mis à disposition | MIT |
| ibanator | 0.1.0 | mis à disposition | EUPL-1.2 |
| komet | 0.1.0 | dépendance | MIT |
| komet | 0.2.0 | dépendance | MIT |
| lilaq | 0.6.0 | mis à disposition | MIT |
| linguify | 0.5.0 | mis à disposition | MIT |
| modern-mailmerge | 0.1.0 | mis à disposition | MIT |
| oxifmt | 1.0.0 | mis à disposition | MIT OR Apache-2.0 |
| payqr-swiss | 0.5.0 | mis à disposition | LGPL-3.0-only |
| primaviz | 0.11.0 | mis à disposition | MIT |
| qrypst | 0.1.1 | mis à disposition | Unlicense AND MIT |
| rustycure | 0.2.0 | dépendance | EUPL-1.2 |
| sepay | 0.1.1 | mis à disposition | MIT |
| showybox | 2.0.4 | mis à disposition | MIT |
| suiji | 0.5.1 | dépendance | MIT |
| tablem | 0.3.0 | mis à disposition | MIT |
| tabut | 1.0.2 | mis à disposition | MIT |
| tiaoma | 0.3.0 | mis à disposition | MIT |
| tiptoe | 0.4.0 | dépendance | MIT |
| zebra | 0.1.0 | mis à disposition | MIT |
| zero | 0.6.1 | dépendance | MIT |
| zero | 0.7.1 | mis à disposition | MIT |

## Mainteneurs : faire évoluer les paquets

La liste est définie par `packages/lock.toml` (nom, version, empreinte sha256, licence, rôle) et
les archives officielles de `packages/vendor/`. Contrat complet :
`specs/002-typst-packages/contracts/lock-file.md`.

- **Ajouter un paquet** : `scripts/add-package.sh <nom> <version>`, puis ses dépendances avec
  `--dependency` jusqu'à ce que `cargo test --test bundled_packages` passe (test de fermeture).
  Ajouter une fixture `tests/fixtures/package-smoke/<nom>.typ` et une ligne ci-dessus.
- **Changer de version** : relancer le script avec la nouvelle version ; l'entrée mise à
  disposition est remplacée et l'ancienne archive supprimée si plus rien ne l'utilise. Tous les
  templates passent à la nouvelle version au prochain déploiement : le noter dans la PR.
- **Retirer un paquet** : rupture pour les templates qui l'importent (ils deviennent invalides) ;
  décision explicite uniquement.
- **Paquet incompatible avec la version de Typst du service** : il est retiré, jamais corrigé
  localement (ex. `codetastic` 0.2.2, retiré à l'intégration initiale).
- La construction vérifie les empreintes : une archive altérée ou non listée fait échouer
  `cargo build` en nommant le paquet.
