# Paquets Typst intégrés à inkpdf — liste à valider

**Statut** : proposition, en attente de validation · **Date** : 2026-10-10 · **Source** : index https://packages.typst.org/preview/index.json, archives téléchargées et inspectées (taille, WASM, imports).

## Décision de principe

- Les paquets retenus sont **intégrés à l'image** à la construction : tous les templates peuvent les importer (`#import "@preview/zero:0.7.1"`) sans rien déposer. Aucun accès réseau au rendu (le téléchargement n'a lieu qu'au build, vérifié par empreinte).
- La constitution le permet déjà (principe II : « ressources embarquées par le binaire, comme les polices par défaut ») : **pas d'amendement**.
- **Fixer** = un fichier de verrouillage dans le dépôt (`packages.lock` : `@preview/nom:version`, empreinte sha256, licence). Le build Docker télécharge et vérifie chaque empreinte.
- **Versions** : un import nomme toujours une version exacte. Mettre à jour un paquet = **ajouter** une version, jamais remplacer : les templates existants continuent de fonctionner.
- **Compatibilité** : le champ `compiler` n'est qu'un minimum. Chaque paquet retenu devra passer un rendu de test sur Typst 0.15.1 avant d'être définitivement verrouillé (0.15 a supprimé `path` comme forme, `pattern`, `pdf.embed`, `*.decode`).

## Pourquoi pas « tous les paquets »

Typst Universe compte **1 665 paquets** (4 923 versions) :
- **838 sont des modèles de documents** (thèses, CV, posters, présentations) : inutiles pour un service, chaque template inkpdf a son propre `main.typ` ;
- **~70 sont sous licence copyleft forte** (GPL, AGPL) : les distribuer dans l'image impose des obligations ;
- la plupart n'ont jamais été testés sur Typst 0.15 ; la taille totale se compterait en centaines de Mo ;
- il reste ~800 bibliothèques, dont une large part pour les maths, la chimie, les slides ou la musique, sans rapport avec la génération de documents.

## Sélection proposée (22 paquets, 32 avec dépendances, 13,7 Mo)

Colonnes : taille décompressée ; WASM = contient un plugin compilé (voir risques).

### Codes-barres et QR codes

| Paquet | Version | Licence | Taille | WASM | Dépendances | Pourquoi |
|---|---|---|---|---|---|---|
| `tiaoma` | 0.3.0 | MIT | 979 Ko | 858 Ko | — | Codes-barres (EAN, Code 128…) et QR, moteur Zint ; le plus complet |
| `zebra` | 0.1.0 | MIT | 121 Ko | 110 Ko | — | QR et DataMatrix, dessin natif |
| `qrypst` | 0.1.1 | Unlicense AND MIT | 52 Ko | 41 Ko | — | QR avec taille de module exacte à l'impression |
| `sepay` | 0.1.1 | MIT | 16 Ko | non | ibanator:0.1.0, rustycure:0.2.0 | QR de virement SEPA (EPC) pour factures |

### Graphiques et dessin

| Paquet | Version | Licence | Taille | WASM | Dépendances | Pourquoi |
|---|---|---|---|---|---|---|
| `cetz` | 0.5.2 | LGPL-3.0-or-later | 679 Ko | 335 Ko | oxifmt:1.0.0 | Dessin vectoriel (base de nombreux paquets) |
| `cetz-plot` | 0.1.4 | LGPL-3.0-or-later | 233 Ko | non | cetz:0.5.2 | Courbes, barres, camemberts sur cetz |
| `lilaq` | 0.6.0 | MIT | 389 Ko | non | elembic:1.1.1, komet:0.1.0, komet:0.2.0, suiji:0.5.1, tiptoe:0.4.0, zero:0.6.1 | Graphiques de données, plus simple que cetz-plot |

### Nombres, unités, dates, langues

| Paquet | Version | Licence | Taille | WASM | Dépendances | Pourquoi |
|---|---|---|---|---|---|---|
| `zero` | 0.7.1 | MIT | 118 Ko | non | — | Formatage des nombres : séparateurs de milliers, décimales (manque natif de Typst) |
| `oxifmt` | 1.0.0 | MIT OR Apache-2.0 | 63 Ko | non | — | Formatage de chaînes façon Rust (dépendance de cetz et hydra) |
| `unify` | 0.8.1 | MIT | 33 Ko | non | — | Nombres avec unités |
| `datify` | 1.3.0 | MIT | 20 Ko | non | datify-core:2.1.0 | Dates localisées (noms de mois en français) |
| `linguify` | 0.5.0 | MIT | 205 Ko | 190 Ko | — | Textes multilingues |

### Mise en page et blocs

| Paquet | Version | Licence | Taille | WASM | Dépendances | Pourquoi |
|---|---|---|---|---|---|---|
| `showybox` | 2.0.4 | MIT | 34 Ko | non | — | Encadrés personnalisables |
| `gentle-clues` | 1.3.1 | MIT | 477 Ko | non | linguify:0.5.0 | Encadrés d'information / avertissement |
| `meander` | 0.4.4 | MIT | 93 Ko | non | hy-dro-gen:0.1.1 | Texte qui contourne les images |
| `wrap-it` | 0.1.1 | Unlicense | 14 Ko | non | — | Habillage simple autour d'une figure |
| `hydra` | 0.6.3 | MIT | 30 Ko | non | oxifmt:1.0.0 | En-têtes courants (titre de section en haut de page) |

### Tableaux

| Paquet | Version | Licence | Taille | WASM | Dépendances | Pourquoi |
|---|---|---|---|---|---|---|
| `tabut` | 1.0.2 | MIT | 44 Ko | non | — | Tableau généré depuis une liste de données |
| `tablem` | 0.3.0 | MIT | 17 Ko | non | — | Tableaux écrits comme en Markdown |

### Contenu et formats

| Paquet | Version | Licence | Taille | WASM | Dépendances | Pourquoi |
|---|---|---|---|---|---|---|
| `cmarker` | 0.1.10 | MIT | 354 Ko | 314 Ko | — | Markdown → Typst (texte riche venant de data) — voir risque |
| `xmlit` | 0.1.3 | MIT | 677 Ko | 618 Ko | — | Génération de XML (base pour Factur-X plus tard) |
| `sicons` | 16.0.0 | MIT | 5329 Ko | 5325 Ko | — | Icônes de marques (SVG) |

### Dépendances ajoutées automatiquement

| Paquet | Version | Licence | Taille | WASM | Tiré par |
|---|---|---|---|---|---|
| `datify-core` | 2.1.0 | MIT | 1152 Ko | non | datify |
| `elembic` | 1.1.1 | MIT OR Apache-2.0 | 289 Ko | non | lilaq |
| `hy-dro-gen` | 0.1.1 | MIT | 1188 Ko | 1183 Ko | meander |
| `ibanator` | 0.1.0 | EUPL-1.2 | 38 Ko | 9 Ko | sepay |
| `komet` | 0.1.0 | MIT | 186 Ko | 173 Ko | lilaq |
| `komet` | 0.2.0 | MIT | 191 Ko | 174 Ko | lilaq |
| `rustycure` | 0.2.0 | EUPL-1.2 | 61 Ko | 44 Ko | sepay |
| `suiji` | 0.5.1 | MIT | 176 Ko | 137 Ko | lilaq |
| `tiptoe` | 0.4.0 | MIT | 58 Ko | non | lilaq |
| `zero` | 0.6.1 | MIT | 77 Ko | non | lilaq |

Total : 32 paquets (deux versions de `zero` et de `komet` coexistent), **13,7 Mo** décompressés, dont ~10 Mo de WASM.

## Points à trancher

1. **`cmarker` (Markdown)** : par défaut (`raw-typst: true`) il exécute le code Typst contenu dans le Markdown. Si le Markdown vient des données de l'appelant, c'est une **injection de code** contraire au principe IV. À n'utiliser qu'avec `raw-typst: false` — le documenter, voire l'exclure.
2. **Licences** : `cetz`, `cetz-plot` sont LGPL-3.0 (copyleft faible : redistribution du source non modifié avec sa licence, acceptable) ; `ibanator` et `rustycure` (tirés par `sepay`) sont EUPL-1.2. Exclure `sepay` supprime l'EUPL.
3. **`sicons`** pèse 5,3 Mo à lui seul (WASM) pour des icônes de marques : à exclure sauf besoin réel.
4. **Doublons** : trois paquets QR (`tiaoma`, `zebra`, `qrypst`). `tiaoma` couvre tout (codes-barres + QR) ; `zebra` est le seul sans moteur lourd. Garder un ou deux.
5. **`lilaq` vs `cetz-plot`** : deux approches des graphiques ; `lilaq` tire 6 dépendances dont deux versions de `komet`.

## Exclus et pourquoi

| Paquet | Raison |
|---|---|
| `tablex` | obsolète depuis le `table` natif (0.11) |
| `fletcher` | fige `cetz:0.3.4`, ancien, risque de rupture sur 0.15 |
| `icu-datetime` | 4 Mo de WASM ; `datify` suffit |
| `fontawesome` | exige des polices installées sur la machine |
| `ez-today` | affiche la date du jour : casse le déterminisme |
| `jogs`, `cades`, `pyrunner` | exécutent du JavaScript ou du Python : surface d'attaque inutile |
| `name-it`, `outrageous`, `suboutline` | GPL-3.0 |
| modèles (`invoice-pro`, `letter-pro`, `rubrol-invoice`…) | ce sont des documents complets, pas des briques ; à étudier comme exemples |

## Risques

- **WASM sans limite d'exécution** : 14 des 32 paquets contiennent un plugin. Un appel long ne peut pas être interrompu par le délai de rendu (voir `research/typst.md` §7.4). Préférer les paquets sans WASM à besoin égal.
- **Évolution** : les paquets intégrés ne changent qu'avec une nouvelle version de l'image ; pour un paquet absent de la liste, il faudra le dossier `packages/` partagé (brique ultérieure).
