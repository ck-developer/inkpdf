# Carte des paquets Typst Universe pour inkpdf

Relevé du 2026-10-10 : **827 bibliothèques** (les 838 modèles de documents sont écartés d'office). Pour chaque paquet : ce qu'il fait, à quel besoin il répond, et une priorité d'intégration.

| Priorité | Sens | Nombre |
|---|---|---|
| **P1** | indispensable pour des documents métier : intégré et testé en premier | 22 |
| **P2** | utile pour certains documents : intégré dans un second temps | 84 |
| **P3** | spécialisé (maths, slides, musique…), sans danger : intégrable sans garantie | 619 |
| **X** | exclu (exécute un autre langage, date du jour, GPL/AGPL, obsolète, police système…) | 102 |

Classement fait à partir des descriptions officielles puis relu ; à ajuster librement. Versions, licences et tailles détaillées : `universe-catalogue.md`.

## 1. Vue d'ensemble par besoin

| Besoin | P1 | P2 | P3 | X |
|---|---|---|---|---|
| Documents métier | 2 | 10 | 7 | 0 |
| Codes-barres et QR | 5 | 2 | 2 | 1 |
| Nombres, montants et unités | 4 | 8 | 9 | 1 |
| Dates et langues | 2 | 6 | 48 | 7 |
| Tableaux | 2 | 14 | 2 | 1 |
| Graphiques et diagrammes | 4 | 6 | 51 | 14 |
| Mise en page et blocs | 1 | 18 | 65 | 5 |
| Texte et typographie | 1 | 4 | 51 | 7 |
| Icônes et images | 0 | 7 | 26 | 13 |
| Données et formats | 1 | 9 | 12 | 3 |
| Sciences et maths | 0 | 0 | 136 | 15 |
| Code et informatique | 0 | 0 | 55 | 9 |
| Présentations | 0 | 0 | 20 | 2 |
| Enseignement et examens | 0 | 0 | 44 | 2 |
| Musique et loisirs | 0 | 0 | 31 | 11 |
| Outils pour auteurs | 0 | 0 | 57 | 3 |
| Autre | 0 | 0 | 3 | 8 |

## 2. P1 et P2 par besoin

### Documents métier

| Prio | Paquet | Ce qu'il fait | Taille | Remarque |
|---|---|---|---|---|
| P1 | `modern-mailmerge` 0.1.0 | Publipostage : lettres, attestations, étiquettes, badges, enveloppes. | 14 Ko |  |
| P1 | `payqr-swiss` 0.5.0 | Bulletin de paiement suisse QR-facture. | 14 Ko | LGPL |
| P2 | `ampelmark` 0.1.2 | Ajoute les mentions de confidentialité TLP (code couleur) sur un document. | 2 Ko |  |
| P2 | `cgreet` 0.2.1 | Compose les formules de politesse allemandes avec les titres. | 7 Ko |  |
| P2 | `nutrition-label-nam` 0.2.0 | Étiquettes de valeurs nutritionnelles au format américain. | 5 Ko |  |
| P2 | `olaii-upn-qr` 0.0.2 | Bulletin de paiement slovène UPN avec code QR. | 668 Ko |  |
| P2 | `rubrol-invoice` 0.1.0 | Modèle de facture B2B conforme à la facturation électronique européenne. | 7 Ko | modèle complet, plutôt un exemple qu'une brique |
| P2 | `satz` 0.1.1 | Modèle unifié pour lettres, factures, rapports et dossiers. | 28 Ko |  |
| P2 | `simple-receipt-printer` 2.0.0 | Met en page des tickets pour imprimante thermique. | 2 Ko |  |
| P2 | `typhorm` 0.1.0 | Aide à construire des formulaires complexes. | 3 Ko |  |
| P2 | `typsium-ghs` 0.1.2 | Affiche pictogrammes et mentions de danger chimique GHS. | 46 Ko |  |
| P2 | `typsium-iso-7010` 0.1.2 | Affiche les panneaux de sécurité ISO 7010. | 627 Ko |  |

### Codes-barres et QR

| Prio | Paquet | Ce qu'il fait | Taille | Remarque |
|---|---|---|---|---|
| P1 | `codetastic` 0.2.2 | Génère codes-barres (EAN, Code128…) et QR codes. | 22 Ko |  |
| P1 | `qrypst` 0.1.1 | Dessine des QR codes avec une taille d'impression exacte. | 22 Ko |  |
| P1 | `sepay` 0.1.1 | Génère le QR code de virement SEPA (EPC) pour les factures. | 5 Ko | dépendances EUPL |
| P1 | `tiaoma` 0.3.0 | Génère codes-barres et QR codes de nombreux formats. | 451 Ko | WASM |
| P1 | `zebra` 0.1.0 | Génère QR codes et Data Matrix en dessin natif. | 57 Ko |  |
| P2 | `clatter` 0.1.0 | Génère des codes-barres PDF417. | 402 Ko |  |
| P2 | `rustycure` 0.2.0 | Génère rapidement des QR codes. | 28 Ko |  |

### Nombres, montants et unités

| Prio | Paquet | Ce qu'il fait | Taille | Remarque |
|---|---|---|---|---|
| P1 | `frogst` 1.0.0 | Écrit les nombres en toutes lettres en français (montants en lettres). | 5 Ko |  |
| P1 | `ibanator` 0.1.0 | Vérifie et met en forme les numéros IBAN. | 19 Ko | EUPL |
| P1 | `oxifmt` 1.0.0 | Formate textes et nombres (décimales, séparateurs, alignement). | 20 Ko |  |
| P1 | `zero` 0.7.1 | Formate précisément nombres et unités (séparateurs, arrondis). | 35 Ko |  |
| P2 | `ea-nasir` 0.1.0 | Comptabilité multi-devises et rapports à partir de fichiers beancount. | 9 Ko |  |
| P2 | `fancy-units` 0.1.1 | Met en forme nombres et unités de mesure. | 17 Ko |  |
| P2 | `metro` 0.3.0 | Nombres et unités de mesure bien formatés. | 17 Ko |  |
| P2 | `num2words` 0.2.0 | Écrit les nombres en toutes lettres, plusieurs langues. | 12 Ko |  |
| P2 | `numeris-scribere` 0.1.3 | Écrit les nombres en toutes lettres, plusieurs langues. | 23 Ko |  |
| P2 | `sigfig` 0.1.0 | Arrondit des nombres selon les chiffres significatifs et l'incertitude. | 4 Ko |  |
| P2 | `sprintf` 0.1.1 | Formate des textes et nombres façon printf. | 224 Ko |  |
| P2 | `unify` 0.8.1 | Formate nombres, unités et intervalles. | 9 Ko |  |

### Dates et langues

| Prio | Paquet | Ce qu'il fait | Taille | Remarque |
|---|---|---|---|---|
| P1 | `datify` 1.3.0 | Formate les dates dans toutes les langues. | 7 Ko |  |
| P1 | `linguify` 0.5.0 | Charge les textes traduits selon la langue du document. | 86 Ko | WASM |
| P2 | `datehog` 0.1.1 | Lit des dates et fait des calculs de durée. | 17 Ko |  |
| P2 | `datify-core` 2.1.0 | Données de langues pour formater les dates, utilisées par datify. | 104 Ko |  |
| P2 | `fy-docs-i18n` 0.1.0 | Choisit le texte traduit selon la langue du document. | 13 Ko |  |
| P2 | `icu-datetime` 0.2.2 | Formate dates et heures selon la langue (ex. « 10 octobre 2026 »). | 1.2 Mo | 4 Mo de WASM ; datify suffit |
| P2 | `transl` 0.2.1 | Traduit mots et expressions selon la langue du document. | 122 Ko |  |
| P2 | `weeklendar` 0.1.0 | Dessine des calendriers hebdomadaires. | 7 Ko |  |

### Tableaux

| Prio | Paquet | Ce qu'il fait | Taille | Remarque |
|---|---|---|---|---|
| P1 | `tablem` 0.3.0 | Écrit des tableaux simplement, comme en Markdown. | 6 Ko |  |
| P1 | `tabut` 1.0.2 | Affiche des données sous forme de tableau. | 9 Ko |  |
| P2 | `akatable` 0.1.0 | Tableaux sobres aux formats de publication prédéfinis. | 4 Ko |  |
| P2 | `baler` 0.1.0 | Équilibre automatiquement la largeur des colonnes d'un tableau. | 11 Ko |  |
| P2 | `biaoge` 0.1.1 | Regroupe, fusionne et totalise des données CSV en tableaux. | 9 Ko |  |
| P2 | `booktabs` 0.0.4 | Style de tableaux professionnel avec filets horizontaux. | 4 Ko |  |
| P2 | `booktyps` 0.1.0 | Style de tableaux professionnel avec filets, très configurable. | 12 Ko |  |
| P2 | `dining-table` 0.1.0 | Définit de grands tableaux colonne par colonne depuis des données. | 584 Ko |  |
| P2 | `fittable` 0.0.1 | Répartit l'espace libre entre les colonnes d'un tableau. | 3 Ko |  |
| P2 | `pillar` 0.3.3 | Définit rapidement l'alignement des colonnes d'un tableau. | 5 Ko |  |
| P2 | `rowmantic` 0.5.0 | Écrit des tableaux ligne par ligne avec séparateurs simples. | 13 Ko |  |
| P2 | `tabbyterms` 0.1.0 | Présente une liste de termes sous forme de tableau. | 10 Ko |  |
| P2 | `tableframe` 0.1.0 | Manipule des données en tableau façon tableur. | 11 Ko |  |
| P2 | `tada` 0.2.0 | Trie, filtre et calcule sur des données tabulaires. | 16 Ko |  |
| P2 | `tbl` 0.1.1 | Écrit des tableaux complexes de façon concise. | 15 Ko |  |
| P2 | `tblr` 0.5.0 | Aide à créer et aligner des tableaux. | 14 Ko |  |

### Graphiques et diagrammes

| Prio | Paquet | Ce qu'il fait | Taille | Remarque |
|---|---|---|---|---|
| P1 | `cetz` 0.5.2 | Bibliothèque de dessin : formes, schémas et graphiques. | 214 Ko | WASM ; LGPL |
| P1 | `cetz-plot` 0.1.4 | Trace des courbes et des graphiques à barres ou camemberts. | 56 Ko | LGPL |
| P1 | `lilaq` 0.6.0 | Graphiques de données (courbes, barres, nuages de points) de qualité. | 101 Ko | 6 dépendances |
| P1 | `primaviz` 0.11.0 | Trace plus de 50 types de graphiques (barres, courbes, camemberts…) sans dépendance. | 96 Ko |  |
| P2 | `fletcher` 0.5.8 | Diagrammes de boîtes reliées par des flèches. | 49 Ko |  |
| P2 | `gantty` 0.5.1 | Diagrammes de Gantt à partir de dates pour plannings de projet. | 14 Ko |  |
| P2 | `gribouille` 0.7.0 | Graphiques statistiques élégants selon la « grammaire des graphiques ». | 397 Ko |  |
| P2 | `sitdown` 1.0.0 | Dessine des plans de table et de placement. | 10 Ko |  |
| P2 | `timeliney` 0.4.0 | Crée des diagrammes de Gantt. | 6 Ko |  |
| P2 | `zeitline` 0.1.1 | Trace des frises chronologiques. | 3 Ko |  |

### Mise en page et blocs

| Prio | Paquet | Ce qu'il fait | Taille | Remarque |
|---|---|---|---|---|
| P1 | `showybox` 2.0.4 | Crée des encadrés colorés et personnalisables. | 9 Ko |  |
| P2 | `alertoni` 1.0.0 | Crée des encadrés d'alerte personnalisables (info, attention, erreur). | 10 Ko |  |
| P2 | `badgery` 0.1.1 | Affiche des badges et étiquettes colorés dans le texte. | 2 Ko |  |
| P2 | `calloutly` 1.2.0 | Encadrés de remarque façon Markdown, personnalisables. | 33 Ko |  |
| P2 | `cheq` 0.4.0 | Écrit facilement des cases à cocher. | 5 Ko |  |
| P2 | `chic-hdr` 0.5.0 | Crée des en-têtes et pieds de page élégants. | 7 Ko |  |
| P2 | `colorful-boxes` 1.4.3 | Encadrés colorés prédéfinis. | 4 Ko |  |
| P2 | `faboxyst` 0.1.0 | Encadrés colorés avec titre, très complets. | 362 Ko |  |
| P2 | `gentle-clues` 1.3.1 | Encadrés colorés d'information, d'avertissement ou de conseil. | 71 Ko |  |
| P2 | `hydra` 0.6.3 | Affiche le titre de la section en cours dans l'en-tête. | 9 Ko |  |
| P2 | `markly` 0.4.0 | Traits de coupe et fonds perdus pour l'impression. | 9 Ko |  |
| P2 | `meander` 0.4.4 | Habillage de texte autour des images, texte sur plusieurs zones. | 25 Ko |  |
| P2 | `note-me` 0.6.0 | Encadrés d'alerte style GitHub (note, attention…). | 5 Ko |  |
| P2 | `oasis-align` 0.4.1 | Place deux contenus côte à côte à hauteur égale. | 9 Ko |  |
| P2 | `ose-pic` 0.1.2 | Images de fond ou de premier plan par page. | 2 Ko |  |
| P2 | `shadowed` 0.4.0 | Ajoute des ombres portées aux blocs. | 12 Ko |  |
| P2 | `sheetwise` 0.1.0 | Dispose plusieurs éléments à imprimer sur une même planche. | 17 Ko |  |
| P2 | `typearea` 0.2.0 | Règle zone de texte et marges façon KOMA-Script. | 2 Ko |  |
| P2 | `wrap-it` 0.1.1 | Fait couler le texte autour d'une image. | 5 Ko |  |

### Texte et typographie

| Prio | Paquet | Ce qu'il fait | Taille | Remarque |
|---|---|---|---|---|
| P1 | `framefit` 0.1.0 | Ajuste la taille du texte pour qu'il tienne dans un cadre. | 6 Ko |  |
| P2 | `chomp` 0.1.0 | Raccourcit un texte trop long pour qu'il tienne dans sa place. | 10 Ko |  |
| P2 | `curvly` 0.1.0 | Écrit du texte en arc de cercle, par exemple pour un tampon. | 4 Ko |  |
| P2 | `one-liner` 0.3.0 | Ajuste la taille du texte pour tenir sur une ligne. | 2 Ko |  |
| P2 | `zheyan` 0.1.0 | Masque une partie d'un texte (ex. numéro de carte). | 2 Ko |  |

### Icônes et images

| Prio | Paquet | Ce qu'il fait | Taille | Remarque |
|---|---|---|---|---|
| P2 | `adequate-iso-7000` 0.1.1 | Fournit les pictogrammes normalisés ISO 7000 pour équipements et étiquettes. | 328 Ko |  |
| P2 | `booticons` 0.0.1 | Fournit les icônes Bootstrap. | 218 Ko |  |
| P2 | `ficons` 0.1.0 | Icônes vectorielles libres. | 14 Ko |  |
| P2 | `heroic` 0.1.2 | Ajoute les icônes Heroicons (vectorielles) au document. | 145 Ko |  |
| P2 | `octique` 0.1.1 | Icônes GitHub Octicons. | 50 Ko |  |
| P2 | `sicons` 16.0.0 | Logos vectoriels de marques connues. | 2.2 Mo |  |
| P2 | `tessera` 0.1.0 | Assemble plusieurs images en mosaïque ou grille. | 40 Ko |  |

### Données et formats

| Prio | Paquet | Ce qu'il fait | Taille | Remarque |
|---|---|---|---|---|
| P1 | `cmarker` 0.1.10 | Convertit du texte Markdown en contenu mis en forme. | 140 Ko | ⚠ désactiver `raw-typst` si le Markdown vient de l'appelant |
| P2 | `exmllent` 0.1.0 | Convertit un tableau Excel XML en tableau. | 3 Ko |  |
| P2 | `gairm-import` 0.9.0 | Vérifie des données JSON selon un schéma et les normalise. | 35 Ko |  |
| P2 | `jsonpath` 0.1.0 | Extrait des valeurs de données JSON par un chemin. | 7 Ko |  |
| P2 | `rexllent` 0.4.1 | Transforme un fichier Excel (xlsx) en tableau. | 956 Ko |  |
| P2 | `spreet` 0.2.0 | Lit des tableurs (Excel, ODS, CSV) pour en extraire les données. | 1.2 Mo |  |
| P2 | `to-stuff` 1.1.0 | Convertit du texte en valeurs Typst (longueurs, couleurs…). | 15 Ko |  |
| P2 | `uuidkit` 0.1.0 | Lit et génère des identifiants uniques reproductibles. | 24 Ko |  |
| P2 | `valkyrie` 0.2.2 | Vérifie que les données reçues ont le bon format. | 10 Ko | doublon : inkpdf valide déjà par JSON Schema |
| P2 | `xmlit` 0.1.3 | Génère et valide des documents XML. | 259 Ko |  |

## 3. Exclus (X)

| Paquet | Ce qu'il fait | Raison |
|---|---|---|
| `abbrev` | Gère abréviations, acronymes et glossaire. | licence GPL ou AGPL (copyleft fort) |
| `auto-jrubby` | Ajoute automatiquement la lecture des caractères japonais. | licence GPL ou AGPL (copyleft fort) |
| `basalt-lib` | Outils de prise de notes façon Zettelkasten. | licence GPL ou AGPL (copyleft fort) |
| `board-n-pieces` | Affiche des échiquiers. | licence GPL ou AGPL (copyleft fort) |
| `brain-transplant` | Convertit du code Brainfuck en Typst. | blague ou démo sans usage |
| `cades` | Génère des QR codes. | exécute du JavaScript (via jogs) |
| `calepin` | Exécute du code R, Python et Julia dans le document. | exécute un autre langage (R, Python, Julia) |
| `callisto` | Lit et exécute des notebooks Jupyter. | exécute un autre langage (Python) |
| `cloudy` | Crée des nuages de mots. | licence GPL ou AGPL (copyleft fort) |
| `color-my-agda` | Coloration syntaxique du langage Agda. | licence GPL ou AGPL (copyleft fort) |
| `combo` | Calculs de combinatoire. | licence GPL ou AGPL (copyleft fort) |
| `coordy` | Déplace un repère visuel avec des commandes clavier. | blague ou démo sans usage |
| `cross-circle` | Implémentation du morpion. | blague ou démo sans usage |
| `crossregex` | Jeu de mots croisés à expressions régulières. | blague ou démo sans usage |
| `ctxjs` | Exécute du JavaScript. | exécute un autre langage (JavaScript) |
| `curli` | Ligatures fantaisistes. | blague ou démo sans usage |
| `dati-basati` | Dessine des diagrammes entité-relation. | licence GPL ou AGPL (copyleft fort) |
| `deckz` | Dessine des cartes à jouer. | licence GPL ou AGPL (copyleft fort) |
| `dovenv` | Charge des variables depuis un fichier .env. | ressources externes, inutile pour inkpdf |
| `echarm` | Graphiques ECharts via JavaScript. | exécute un autre langage (JavaScript) |
| `embiggen` | Taille des parenthèses façon LaTeX. | licence GPL ou AGPL (copyleft fort) |
| `energy-dia` | Diagrammes de niveaux d'énergie. | licence GPL ou AGPL (copyleft fort) |
| `erna` | Dispose des images en rangées façon magazine. | licence GPL ou AGPL (copyleft fort) |
| `esotefy` | Interpréteur Brainfuck. | blague ou démo sans usage |
| `example` | Paquet d'exemple. | blague ou démo sans usage |
| `ez-today` | Affiche la date du jour. | utilise la date du jour (non déterministe) |
| `fauxreilly` | Couvertures parodiques façon O'Reilly. | licence GPL ou AGPL (copyleft fort) |
| `feyndrawgram` | Affiche des diagrammes de Feynman exportés. | licence GPL ou AGPL (copyleft fort) |
| `fitchcraft` | Preuves logiques style Fitch. | licence GPL ou AGPL (copyleft fort) |
| `fleck` | Ajoute des taches de café décoratives. | blague ou démo sans usage |
| `flupke-headstamp` | Affiche des informations Git dans le document. | ressources externes (dépôt Git) |
| `fontawesome` | Icônes Font Awesome. | exige des polices installées sur la machine |
| `formal-homework` | Modèle de devoirs à rendre. | licence GPL ou AGPL (copyleft fort) |
| `fruitify` | Remplace les lettres des formules par des emojis de fruits. | blague sans usage |
| `game-theoryst` | Mise en forme de jeux en théorie des jeux. | licence GPL ou AGPL (copyleft fort) |
| `genealotree` | Dessine des arbres généalogiques. | licence GPL ou AGPL (copyleft fort) |
| `griddle` | Conçoit et affiche des grilles de mots croisés. | licence GPL ou AGPL (copyleft fort) |
| `hamnosys-includer` | Affiche la notation HamNoSys de langue des signes. | exige des polices installées sur la machine |
| `handy-dora` | Affiche des tuiles de mahjong. | très lourd pour un intérêt quasi nul |
| `haw-hamburg` | Modèle de rapport ou thèse de la HAW Hambourg. | licence GPL ou AGPL (copyleft fort) |
| `herodot` | Dessine des frises chronologiques linéaires. | licence GPL ou AGPL (copyleft fort) |
| `iconic-salmon-fa` | Liens vers réseaux sociaux avec icônes Font Awesome. | exige des polices installées sur la machine |
| `iconify` | Accès à plus de 200 000 icônes Iconify. | accès à des ressources externes |
| `identified` | Crée des diagrammes de processus IDEF. | licence GPL ou AGPL (copyleft fort) |
| `indenta` | Corrige le retrait du premier paragraphe. | obsolète, remplacé par une fonction native de Typst |
| `jlyfish` | Exécute du code Julia dans le document. | exécute un autre langage (Julia) |
| `jogs` | Exécute du JavaScript dans le document. | exécute un autre langage (JavaScript) |
| `kantan` | Crée des tableaux kanban. | licence GPL ou AGPL (copyleft fort) |
| `kino` | Prototype pour créer des animations. | démo sans usage |
| `leetify` | Transforme le texte en « leet speak ». | blague sans usage |
| `lemmify` | Mise en forme de théorèmes. | licence GPL ou AGPL (copyleft fort) |
| `libra` | Équilibre la longueur des lignes d'un paragraphe. | licence GPL ou AGPL (copyleft fort) |
| `lucide` | Icônes Lucide. | exige des polices installées sur la machine |
| `luzid-checkbox` | Listes de cases à cocher avec icônes. | licence GPL ou AGPL (copyleft fort) |
| `m-jaxon` | Rend des formules LaTeX via MathJax. | exécute un autre langage (JavaScript) |
| `magnifying-glass` | Effet loupe sur une partie d'image. | licence GPL ou AGPL (copyleft fort) |
| `mastermind` | Diagrammes de classes UML. | licence GPL ou AGPL (copyleft fort) |
| `materially` | Icônes Google Material Symbols. | exige des polices installées sur la machine |
| `minitoc` | Table des matières d'une seule section. | licence GPL ou AGPL (copyleft fort) |
| `moodular` | Génère du contenu pour cours Moodle. | licence GPL ou AGPL (copyleft fort) |
| `muchpdf` | Insère des pages PDF comme images. | licence GPL ou AGPL (copyleft fort) |
| `name-it` | Écrit les nombres en toutes lettres en anglais. | licence GPL ou AGPL (copyleft fort) |
| `nerd-icons` | Icônes Nerd Font. | exige des polices installées sur la machine |
| `nulite` | Graphiques décrits en Vega-Lite. | exécute un autre langage (JavaScript) |
| `outrageous` | Personnalise la table des matières. | licence GPL ou AGPL (copyleft fort) |
| `owlbear` | Fiches pour Donjons et Dragons. | licence GPL ou AGPL (copyleft fort) |
| `pintorita` | Diagrammes (séquence, Gantt, cartes mentales) via Pintora. | exécute un autre langage (JavaScript) |
| `pintorita-neo` | Diagrammes via Pintora. | exécute un autre langage (JavaScript) |
| `plotst` | Graphiques et courbes. | obsolète, remplacé par lilaq ou CeTZ |
| `polytonoi` | Convertit des lettres latines en grec polytonique. | licence GPL ou AGPL (copyleft fort) |
| `povrayst` | Images 3D par lancer de rayons. | licence GPL ou AGPL (copyleft fort) |
| `prefigure` | Diagrammes mathématiques PreFigure. | licence GPL ou AGPL (copyleft fort) |
| `prooftrees` | Dessine des arbres de preuve logique. | Obsolète, remplacé par curryst |
| `proteograph` | Visualise des données de protéomique. | Licence GPL |
| `pyrunner` | Exécute du code Python dans le document. | Exécute un autre langage (Python) |
| `realhats` | Remplace les accents circonflexes par de vrais chapeaux. | Blague sans usage |
| `ribon` | Visualise la structure secondaire de l'ARN. | Licence GPL |
| `rubby` | Ajoute des annotations de lecture (furigana) au texte. | Licence AGPL |
| `rufish` | Génère du faux texte russe de remplissage. | Très lourd pour intérêt quasi nul |
| `scriptie` | Modèle de scénario de film. | Licence GPL |
| `socialhub-fa` | Liens vers réseaux sociaux avec icônes Font Awesome. | Exige des polices installées |
| `stargazing` | Met en forme des tableaux de résultats de régression. | Licence GPL |
| `stash` | Met du contenu de côté pour l'afficher plus tard. | Licence GPL |
| `staunton` | Diagrammes et parties d'échecs. | Licence en partie GPL |
| `stonewall` | Couleurs des drapeaux des fiertés pour dégradés. | Licence GPL |
| `suboutline` | Affiche une table des matières limitée à une section. | Licence GPL |
| `tableau-icons` | Insère les icônes Tabler. | Exige des polices installées |
| `tablex` | Tableaux avancés personnalisables. | Obsolète, remplacé par les tableaux natifs |
| `testyfy` | Vérifie des conditions dans le document. | Licence GPL |
| `text-dirr` | Donne le sens d'écriture effectif du texte. | Licence AGPL |
| `tiny-bingo` | Génère des grilles de bingo aléatoires. | Licence AGPL |
| `touying2video` | Transforme des diapositives en vidéo commentée. | Licence AGPL, outil externe |
| `twig` | Crée des arbres à partir de listes. | Licence GPL |
| `typart` | Diagrammes pour affiches et présentations. | Licence GPL |
| `typshade` | Visualise des alignements de séquences biologiques. | Licence GPL |
| `typsqlite` | Interroge une base SQLite à la compilation. | Exécute un autre langage (SQL) |
| `use-academicons` | Icônes académiques. | Exige des polices installées |
| `use-tabler-icons` | Icônes Tabler via police. | Exige des polices installées |
| `vonsim` | Coloration syntaxique pour VonSim. | Licence AGPL |
| `xarrow` | Flèches de longueur variable. | Licence GPL |
| `yak` | Filtre des données avec des requêtes jq. | Exécute un autre langage (jq) |
| `zhconv` | Convertit le chinois traditionnel et simplifié. | Licence GPL |

## 4. P3 — spécialisés, par besoin

### Documents métier (7)

| Paquet | Ce qu'il fait |
|---|---|
| `abntyp` | Met en forme des documents universitaires selon les normes brésiliennes ABNT. |
| `clean-ats-cv` | Modèle de CV sobre et lisible par les logiciels de recrutement. |
| `dsek` | Modèles de documents d'une association étudiante suédoise. |
| `fancy-affil` | Gère les affiliations des auteurs. |
| `folio` | Génère des documents de gestion de projet depuis une source unique. |
| `hagakiii` | Imprime des adresses sur des cartes postales japonaises. |
| `jurz` | Numéros de paragraphes en marge pour textes juridiques allemands. |

### Codes-barres et QR (2)

| Paquet | Ce qu'il fait |
|---|---|
| `materialize` | Codes d'appairage Matter pour objets connectés. |
| `vanilla-aruco` | Génère des marqueurs ArUco pour la vision par ordinateur. |

### Nombres, montants et unités (9)

| Paquet | Ce qu'il fait |
|---|---|
| `a2c-nums` | Écrit un nombre en caractères chinois. |
| `big-rati` | Calculs exacts sur de très grandes fractions. |
| `frackable` | Écrit des fractions typographiques (½, ¾). |
| `fractus` | Calculs sur les fractions. |
| `indic-numerals` | Convertit les chiffres arabes en chiffres indiens et inversement. |
| `nth` | Ordinaux anglais (1st, 2nd, 3rd). |
| `statastic` | Calcule des statistiques sur des séries de nombres. |
| `suiji` | Génère des nombres pseudo-aléatoires à partir d'une graine. |
| `zero-calc` | Calcule et affiche des grandeurs formatées par zero. |

### Dates et langues (48)

| Paquet | Ce qu'il fait |
|---|---|
| `aoran` | Choisit l'article anglais « a » ou « an ». |
| `ascii-ipa` | Convertit une notation simplifiée en alphabet phonétique international. |
| `asian-power` | Force la mise en gras ou italique des caractères asiatiques. |
| `auto-bidi` | Gère automatiquement le sens d'écriture de l'hébreu, l'arabe et le persan. |
| `auto-bihua` | Ordre et nombre de traits des caractères chinois. |
| `auto-canto` | Transcrit le chinois en romanisation cantonaise. |
| `auto-mando` | Transcrit le chinois en romanisation mandarine. |
| `auto-pinyin` | Convertit le chinois en pinyin. |
| `basho` | Écrit le japonais verticalement. |
| `bidi-flow` | Détecte le sens d'écriture dans les textes mêlant arabe, hébreu et latin. |
| `calendaring` | Dessine la grille d'un mois de calendrier. |
| `canto-parser` | Affiche le cantonais avec sa prononciation. |
| `chiandiau` | Affiche la prononciation tonale de langues chinoises. |
| `cineca` | Crée un calendrier avec des événements. |
| `cjk-spacer` | Améliore l'espacement du texte japonais. |
| `cjk-unbreak` | Supprime les espaces parasites dans le texte chinois ou japonais. |
| `cjk-unshrink` | Garde la pleine largeur de la ponctuation asiatique. |
| `conjak` | Écrit les nombres selon les conventions chinoises, japonaises, coréennes. |
| `ctyp` | Aide à la typographie chinoise. |
| `easy-pinyin` | Écrit facilement le pinyin chinois. |
| `eggs` | Exemples linguistiques numérotés avec gloses. |
| `furiruby` | Annotations de prononciation (furigana) au-dessus du texte japonais. |
| `glotter` | Détecte la langue d'un fragment de texte. |
| `hundouk` | Affiche du chinois classique annoté pour lecture japonaise. |
| `hy-dro-gen` | Coupe les mots en fin de ligne selon la langue. |
| `koty` | Grammaire coréenne et nombres en coréen. |
| `leipzig-glossing` | Gloses linguistiques interlinéaires. |
| `lingotree` | Arbres syntaxiques linguistiques. |
| `linphon` | Notations de phonologie. |
| `naifs-islamic-research-toolkit` | Affiche le texte coranique avec polices dédiées. |
| `pannotyp` | Prise en charge du hongrois. |
| `penpo` | Correcteur pour la langue toki pona. |
| `phonokit` | Représentations phonologiques. |
| `roster-cjk` | Aligne des listes de noms chinois ou japonais en colonnes. |
| `se-jyutcitzi` | Affiche les caractères cantonais Jyutcitzi. |
| `synkit` | Représentations syntaxiques pour la linguistique. |
| `syntree` | Dessine des arbres syntaxiques linguistiques. |
| `thaibreak` | Coupe correctement les lignes de texte thaï. |
| `tieflang` | Traductions pour les modèles Tief. |
| `tricorder` | Aligne des noms chinois sur trois caractères. |
| `tyipa` | Écrit des transcriptions phonétiques (API). |
| `unidep` | Dessine des arbres de dépendances linguistiques. |
| `vitis` | Coupe les lignes des langues écrites sans espaces. |
| `vlna` | Ajoute les espaces insécables du tchèque. |
| `vtzone` | Écriture verticale pour textes chinois et japonais. |
| `yaml-dadaism` | Calendrier mensuel rempli depuis un fichier YAML. |
| `zh-format` | Gras, italique et souligné adaptés au chinois. |
| `zh-kit` | Prise en charge de base du chinois (polices, etc.). |

### Tableaux (2)

| Paquet | Ce qu'il fait |
|---|---|
| `cap-able` | Tableaux et figures académiques avec légendes bilingues. |
| `easytable` | Création simplifiée de tableaux. |

### Graphiques et diagrammes (51)

| Paquet | Ce qu'il fait |
|---|---|
| `aa-draw` | Transforme des dessins en caractères ASCII en schémas vectoriels. |
| `arch-plotter` | Dessins techniques d'architecture et de géomètre en 2D. |
| `autofletcher` | Simplifie la création de diagrammes avec fletcher. |
| `autograph` | Place automatiquement les nœuds d'un graphe fletcher. |
| `blockcell` | Schémas de blocs pour formats de données et registres. |
| `bob-draw` | Transforme des dessins ASCII en schémas vectoriels. |
| `cetz-venn` | Dessine des diagrammes de Venn à deux ou trois ensembles. |
| `chalks` | Dessins et annotations au style crayon ou craie. |
| `chronos` | Dessine des diagrammes de séquence. |
| `circuiteria` | Dessine des schémas de circuits logiques. |
| `confy` | Dessine des matrices de confusion. |
| `corkscrew` | Génère des dégradés de couleurs cubehelix. |
| `diagraph` | Dessine des graphes avec Graphviz. |
| `diagraph-layout` | Calcule la disposition de graphes avec Graphviz. |
| `facade` | Schémas de blocs pour vues d'architecture. |
| `fast-layout` | Calcule rapidement la disposition de graphes. |
| `fractusist` | Dessine des fractales. |
| `gradslide` | Affiche une valeur entre 0 et 1 sur une jauge en dégradé. |
| `graph-gen` | Affiche le graphe des liens entre notes. |
| `gviz` | Dessine des graphes décrits en langage Graphviz (dot). |
| `kebab-chart` | Graphique de périodes dans le temps. |
| `kip` | Dessine des diagrammes décrits en langage Pikchr. |
| `larnt` | Dessins 3D au trait. |
| `larrow` | Dessine des flèches entre éléments du document. |
| `matofletcher` | Simplifie les diagrammes de flèches. |
| `mercator` | Affiche des cartes géographiques (GeoJSON). |
| `merman` | Affiche des diagrammes Mermaid. |
| `mmdr` | Affiche des diagrammes Mermaid. |
| `neoplot` | Graphiques réalisés avec Gnuplot. |
| `nibart` | Calligraphie, ornements et entrelacs. |
| `ornamentalyst` | Dessine des ornements décoratifs. |
| `oxdraw` | Affiche des diagrammes Mermaid. |
| `paiagram` | Graphiques horaires de transports. |
| `perlit` | Graphes de notes Obsidian. |
| `pivot` | Diagrammes d'analyse de cybermenaces. |
| `proxim` | Dessine des schémas en blocs reliés par des flèches. |
| `qualitree` | Dessine et compare des matrices « maison de la qualité ». |
| `railynx` | Dessine des schémas de voies ferrées. |
| `ribbony` | Dessine des diagrammes de Sankey et en cordes. |
| `scenery` | Dessine des scènes en 2D et 3D. |
| `scrawl` | Dessine des formes à l'allure dessinée à la main. |
| `simple-plot` | Trace des courbes de fonctions mathématiques. |
| `simpleplot` | Trace des courbes très simplement. |
| `sprig` | Dessine des cartes mentales autour d'un polygone central. |
| `tdtr` | Dessine des arbres bien ordonnés. |
| `tiago` | Dessine des diagrammes décrits en langage D2. |
| `tidymind` | Dessine des cartes mentales horizontales. |
| `tiptoe` | Pointes de flèches et marques pour les traits. |
| `typograph` | Dessine des diagrammes avec nœuds et liens. |
| `upsetter` | Diagrammes UpSet d'intersections d'ensembles. |
| `vote-card` | Affiche des résultats de vote avec barres de progression. |

### Mise en page et blocs (65)

| Paquet | Ce qu'il fait |
|---|---|
| `abyss-book` | Modèle de livre au thème sombre. |
| `admon-blk` | Ajoute des encadrés d'avertissement ou de remarque. |
| `anti-matter` | Numérote séparément les pages d'introduction et d'annexe. |
| `ape` | Ensemble de mises en page prêtes pour rapports. |
| `babble-bubbles` | Crée des encadrés de remarque. |
| `beautitled` | Styles de titres de chapitres et de sommaire. |
| `biceps` | Dispose des éléments en ligne avec retour automatique. |
| `bookletic` | Prépare des livrets imprimables pliés. |
| `catppuccin` | Applique un thème de couleurs pastel. |
| `concise-history` | Mise en page de livres d'histoire chinois. |
| `deixis` | Notes, flèches et surlignages reliés au texte. |
| `discount` | Crée des sous-compteurs liés aux titres. |
| `dorodango` | Dessine des rectangles aux coins très arrondis. |
| `edgeframe` | Réglages rapides de page. |
| `efilrst` | Listes dont on peut citer les éléments. |
| `ega-numbering` | Numérotation de style EGA. |
| `gridlock` | Aligne le texte sur une grille de lignes régulière. |
| `gruvy` | Palette de couleurs Gruvbox. |
| `headcount` | Numérote figures ou tableaux selon le numéro de section. |
| `i-figured` | Numérote figures et équations par section. |
| `keepsake` | Cartes pliables imprimables pour occasions. |
| `lasagna` | Calques de contenu activables par étiquettes. |
| `litfass` | Mise en page d'affiches par tuiles. |
| `marge` | Notes dans la marge. |
| `marginalia` | Notes de marge et blocs pleine largeur. |
| `modpattern` | Corrige les bords des motifs de remplissage. |
| `monthweave` | Calendriers mensuels et annuels imprimables. |
| `na-arabox` | Encadrés pour texte arabe (droite à gauche). |
| `niram-css` | Les 147 noms de couleurs CSS. |
| `non-unlabeled` | Ne numérote que les éléments référencés. |
| `nordic` | Palette de couleurs Nord. |
| `notionly` | Donne au document l'apparence de Notion. |
| `numberingx` | Styles de numérotation étendus (romains, lettres…). |
| `numblex` | Aide à définir des numérotations. |
| `numbly` | Numérotation différente par niveau de titre. |
| `numera` | Numérotation des figures et équations par chapitre. |
| `nup` | Aperçu de plusieurs pages sur une seule. |
| `ornacover` | Pages de couverture ornementées pour documents scolaires. |
| `outline-summaryst` | Table des matières avec résumé par chapitre. |
| `parize` | Intègre des blocs dans un paragraphe. |
| `pigmentpedia` | Grande bibliothèque de couleurs nommées. |
| `pillole` | Étiquettes bicolores en forme de pastille. |
| `pinit` | Positionne des éléments par rapport à des repères. |
| `postercise` | Affiches de recherche académique. |
| `qcm` | Fournit des palettes de couleurs pour distinguer des catégories. |
| `ratchet` | Améliore la numérotation des figures, tableaux et équations. |
| `rich-counters` | Crée des compteurs qui dépendent d'autres compteurs. |
| `riffle` | Change les marges du texte en milieu de page. |
| `rose-pine` | Applique le thème de couleurs Rosé Pine. |
| `run-liners` | Crée des listes écrites sur une seule ligne. |
| `slate-geometry` | Formats de page adaptés à des liseuses et tablettes. |
| `smartaref` | Regroupe les renvois consécutifs (figures 1 à 3). |
| `songting-book` | Crée des livres au style chinois. |
| `splash` | Collection de palettes de couleurs. |
| `subpar` | Crée des sous-figures numérotées. |
| `synapse` | Relie chaque mention d'un concept à sa définition. |
| `treet` | Crée des listes en arborescence. |
| `tschich` | Calcule des proportions de page harmonieuses. |
| `tsinswreng-auto-heading` | Gère automatiquement les niveaux de titres. |
| `typewind` | Couleurs de la palette Tailwind CSS. |
| `typhoon` | Styles Tailwind CSS pour l'export HTML. |
| `typpuccino` | Palette de couleurs Catppuccin. |
| `umbra` | Ajoute des ombres simples. |
| `varioref` | Renvois avec numéro de page si nécessaire. |
| `yemianfengge` | Gère les styles de page. |

### Texte et typographie (51)

| Paquet | Ce qu'il fait |
|---|---|
| `ab-annotate` | Affiche résumés et notes sous chaque référence d'une bibliographie. |
| `abbr` | Gère les abréviations et leur liste dans un document. |
| `acrostiche` | Gère les acronymes et leurs définitions. |
| `acrotastic` | Gère les acronymes et leurs définitions. |
| `alexandria` | Permet plusieurs bibliographies dans un même document. |
| `anatomy` | Visualise les dimensions et métriques d'une police. |
| `arnoptical` | Choisit la variante de la police Arno Pro selon la taille. |
| `babel` | Masque un texte en le remplaçant par des caractères aléatoires. |
| `blindex` | Crée un index des citations bibliques. |
| `blinky` | Transforme les titres de la bibliographie en liens. |
| `citegeist` | Lit une bibliographie BibTeX comme des données. |
| `citesugar` | Raccourcis pour citer des références. |
| `citrus` | Met en forme une bibliographie selon les styles CSL. |
| `cuti` | Simule gras, italique et petites capitales absents d'une police. |
| `decasify` | Met les majuscules aux titres selon la langue. |
| `droplet` | Lettrines en début de paragraphe. |
| `easy-typography` | Réglages typographiques par défaut raisonnables. |
| `enja-bib` | Bibliographies BibTeX en anglais et japonais. |
| `gb7714-bilingual` | Bibliographie au format chinois GB/T 7714, bilingue. |
| `gloss-awe` | Crée un glossaire des termes du document. |
| `glossarium` | Glossaire personnalisable avec renvois vers les définitions. |
| `glossy` | Glossaire simple avec présentation personnalisable. |
| `hes-so-core` | Base commune des modèles de la haute école HES-SO. |
| `hidden-bib` | Bibliographie avec références non citées dans le texte. |
| `i-am-acro` | Gère les sigles et acronymes, avec plusieurs langues. |
| `in-dexter` | Crée un index des mots choisis en fin de document. |
| `itemize` | Personnalise l'apparence des listes à puces et numérotées. |
| `its-scripted` | Mise en forme de scénarios de film ou théâtre. |
| `jurlstify` | Coupe proprement les longues adresses web en fin de ligne. |
| `kinase` | Styles différents selon le type de lien (mail, web). |
| `koma-labeling` | Listes à étiquettes alignées, façon KOMA-Script. |
| `latex-lookalike` | Donne au document l'apparence d'un document LaTeX. |
| `linkify` | Génère des liens vers des contenus web. |
| `metalogo` | Logos LaTeX, XeLaTeX, etc. |
| `mtret` | Modèle de rapport pour étudiants japonais. |
| `orchid` | Identifiants chercheur ORCID avec logo. |
| `palimpsest` | Révisions de manuscrit et lettres de réponse aux relecteurs. |
| `palimset` | Montre les différences entre deux documents. |
| `pergamon` | Gestion de bibliographie façon BibLaTeX. |
| `pointless-size` | Tailles de police chinoises traditionnelles. |
| `quan` | Affiche des numéros entourés d'un cercle. |
| `rfc-vibe` | Met en forme les mots-clés normatifs style RFC (DOIT, PEUT…). |
| `roremu` | Génère du faux texte japonais de remplissage. |
| `swank-tex` | Affiche les logos TeX et LaTeX. |
| `symbolx` | Définit des symboles personnalisés plus puissants. |
| `tasteful-pairings` | Associations de polices choisies avec soin. |
| `titleize` | Met des majuscules aux mots d'un titre (anglais). |
| `typarium` | Crée des fiches de présentation de polices. |
| `unichar` | Informations sur les caractères Unicode. |
| `untypsignia` | Imite les logos de logiciels de composition. |
| `verseatile` | Met en forme de la poésie. |

### Icônes et images (26)

| Paquet | Ce qu'il fait |
|---|---|
| `attributionistic` | Liste automatiquement les crédits des images utilisées. |
| `automosaic` | Dispose automatiquement des photos en mosaïque. |
| `boxr` | Crée des patrons de boîtes en carton à découper. |
| `bulb` | Applique un effet de tramage aux images. |
| `ccicons` | Fournit les icônes des licences Creative Commons. |
| `chromo` | Génère des pages de test couleur pour imprimantes. |
| `cntopo` | Icônes de topologie de réseaux informatiques. |
| `computer-framer` | Place un contenu dans un cadre de fenêtre d'ordinateur. |
| `exiftract` | Lit les métadonnées Exif des photos. |
| `fancy-tiling` | Motifs de remplissage : rayures, damiers, nids d'abeille. |
| `flagada` | Drapeaux des pays selon leur code ISO. |
| `grayness` | Retouche simple d'images : niveaux de gris, recadrage. |
| `iconic-salmon-svg` | Liens vers réseaux sociaux avec icônes vectorielles intégrées. |
| `jxl-loader` | Lit les images au format JPEG XL. |
| `keyless` | Rend transparentes certaines couleurs d'une image. |
| `maquette` | Affiche des modèles 3D en image. |
| `nine-patch` | Images extensibles sans déformer les bords. |
| `piclabeler` | Annote des images avec étiquettes et flèches. |
| `pixel-family` | Petits personnages en pixel art dans le texte. |
| `polario-frame` | Cadres photo décoratifs. |
| `roumnd` | Fournit de petites icônes arrondies dessinées. |
| `scienceicons` | Icônes pour articles scientifiques ouverts. |
| `socialyst` | Imite des publications de réseaux sociaux. |
| `spryst` | Découpe une planche de sprites en images séparées. |
| `svgalpha` | Rend une image SVG semi-transparente. |
| `typixel` | Affiche du pixel art depuis images ou grilles. |

### Données et formats (12)

| Paquet | Ce qu'il fait |
|---|---|
| `based` | Encode et décode du texte en base64, base32 ou base16. |
| `digestify` | Calcule des empreintes cryptographiques (hash). |
| `exemel` | Convertit des données en XML et flux Atom. |
| `inv-cmarker` | Convertit du contenu Typst en texte Markdown. |
| `jsonschemeyst` | Valide des données selon un schéma JSON. |
| `kuddle` | Lit des fichiers au format KDL. |
| `matador` | Lit des fichiers de données MATLAB (.mat). |
| `packrat` | Décode et extrait des données compressées ou encodées. |
| `pdf-decorating` | Mise en forme de contenus Markdown style web. |
| `pleast` | Lit des fichiers de configuration plist. |
| `tenv` | Lit le contenu d'un fichier .env. |
| `yats` | Sérialise des données Typst. |

### Sciences et maths (136)

| Paquet | Ce qu'il fait |
|---|---|
| `adaptive-dots` | Place les points de suspension correctement dans les formules mathématiques. |
| `aimesymb` | Ajoute des symboles mathématiques de style AMS dans les formules. |
| `alchemist` | Dessine des formules chimiques développées. |
| `alterlang` | Traduit les noms d'opérateurs mathématiques selon la langue. |
| `amlos` | Produit une liste des symboles utilisés dans le document. |
| `arborly` | Dessine des arbres syntaxiques de linguistique. |
| `astro` | Dessine des schémas d'astronomie. |
| `atomic` | Dessine des atomes avec leurs couches d'électrons. |
| `atostate` | Écrit les états atomiques en physique. |
| `auto-div` | Pose automatiquement une division de polynômes. |
| `axiom` | Raccourcis de notations mathématiques courantes. |
| `axodendron` | Visualise la forme de neurones à partir de fichiers scientifiques. |
| `beam` | Dessine des montages d'optique expérimentale. |
| `beautiframe` | Encadrés stylés pour théorèmes et définitions. |
| `bone` | Dessine des schémas de mécanique des structures. |
| `boxproof` | Présente des preuves logiques encadrées. |
| `breather` | Évite le chevauchement de lignes causé par les grandes formules. |
| `caletz` | Visualise des variétés de Calabi-Yau. |
| `cetz-feynman-lite` | Dessine des diagrammes de Feynman. |
| `cetz-fields` | Dessine des champs électriques. |
| `chem-par` | Écrit des formules et noms chimiques. |
| `chemformula` | Met en forme des formules chimiques. |
| `clanker-slop-nicematrix` | Dessine des matrices avec pointillés et blocs. |
| `commute` | Dessine des diagrammes commutatifs. |
| `consketcher` | Dessine des schémas-blocs d'automatique. |
| `ctheorems` | Environnements numérotés pour théorèmes et preuves. |
| `ctz-euclide` | Constructions de géométrie euclidienne. |
| `curryst` | Arbres de règles d'inférence logique. |
| `czbloch` | Dessine des sphères de Bloch. |
| `delimitizer` | Ajuste la taille des parenthèses en maths. |
| `derive-it` | Preuves de déduction naturelle style Fitch. |
| `dgs` | Géométrie dynamique façon GeoGebra. |
| `distro` | Lois de probabilité et leurs propriétés. |
| `diverential` | Écrit facilement des différentielles. |
| `down` | Prolonge les indices des sommes et intégrales à la ligne. |
| `drawmatrix` | Visualise des matrices dans les formules. |
| `drawstring` | Dessine des diagrammes de cordes mathématiques. |
| `epsilon` | Résolution numérique d'équations. |
| `eqalc` | Transforme des formules en fonctions calculables. |
| `eqrun` | Calcule des équations et réutilise les résultats. |
| `equate` | Améliorations pour les équations mathématiques. |
| `ezchem` | Dessine des structures atomiques et ioniques. |
| `fibber` | Schémas de procédés de microfabrication. |
| `finite` | Dessine des automates finis. |
| `flautomat` | Dessine des automates à partir de données JSON. |
| `frame-it` | Cadres personnalisés pour théorèmes et environnements. |
| `frederic` | Preuves de déduction naturelle style Fitch. |
| `genotypst` | Analyse et visualisation de données de bio-informatique. |
| `great-theorems` | Blocs théorème et démonstration pour documents mathématiques. |
| `h-graph` | Dessine des graphes mathématiques avec une notation simple. |
| `implicplot` | Trace précisément des courbes définies par des équations. |
| `inknertia` | Dessine des schémas de physique (Feynman, espace-temps). |
| `intextual` | Alignement et étiquettes de lignes dans les équations. |
| `intl-math` | Traduit les noms de fonctions mathématiques selon la langue. |
| `invaria` | Fournit des constantes physiques avec leurs métadonnées. |
| `ionio-illustrate` | Trace des spectres de masse annotés. |
| `irif` | Méthodes numériques : intégrales, dérivées, recherche de racines. |
| `k-mapper` | Dessine des tableaux de Karnaugh (logique binaire). |
| `kalt` | Évalue des expressions mathématiques imbriquées. |
| `kangaroo` | Notation de preuves en cryptographie. |
| `karnaugh-express` | Tableaux de Karnaugh personnalisables. |
| `km` | Tableaux de Karnaugh simples. |
| `komet` | Calculs numériques rapides. |
| `lasaveur` | Raccourcis d'écriture mathématique inspirés de LaTeX. |
| `laserly` | Schémas de montages optiques laser. |
| `lemming` | Environnements mathématiques (théorèmes, définitions). |
| `linkst` | Dessine des nœuds mathématiques. |
| `mannot` | Surligne et annote des formules mathématiques. |
| `materia` | Visualise des structures cristallines. |
| `matset` | Évaluateur d'expressions mathématiques. |
| `mechanical-system-cetz-34j` | Schémas de systèmes mécaniques (ressorts, masses). |
| `mesa` | Schémas de composants semi-conducteurs. |
| `minienvs` | Environnements théorème minimalistes. |
| `mitex` | Utilise des formules écrites en LaTeX. |
| `modiagram` | Diagrammes d'orbitales moléculaires. |
| `molchemist` | Affiche des structures chimiques. |
| `molfig` | Affiche des structures moléculaires en 3D. |
| `natrix` | Matrices mathématiques homogènes. |
| `neural-netz` | Schémas d'architectures de réseaux de neurones. |
| `neural-viz` | Schémas de réseaux de neurones. |
| `nomos` | Liste des symboles avec unités et descriptions. |
| `numty` | Calcul sur matrices et vecteurs. |
| `ouset` | Symboles placés au-dessus ou sous d'autres en maths. |
| `pardioid` | Dessine des courbes paramétriques. |
| `pariman` | Calculs d'ingénierie avec unités. |
| `patatrac` | Schémas de physique. |
| `pavemat` | Mise en couleur de matrices. |
| `peano` | Outils mathématiques : fractions, arithmétique. |
| `pedigrypst` | Arbres généalogiques médicaux (pedigrees). |
| `physica` | Notations mathématiques pour sciences et ingénierie. |
| `pi-games` | Diagrammes de théorie des jeux. |
| `plotsy-3d` | Graphiques 3D de surfaces. |
| `prismath` | Colore les parenthèses des formules mathématiques pour mieux les lire. |
| `probabilitree` | Dessine des arbres de probabilités. |
| `prooflists` | Compose des arbres de preuve logique avec une syntaxe simple. |
| `ptable-amat` | Affiche le tableau périodique des éléments. |
| `pull-eh` | Dessine des systèmes de poulies. |
| `pythagorean-spiral` | Dessine la spirale de Théodore avec ses longueurs exactes. |
| `qec-thrust` | Dessine des codes quantiques de correction d'erreurs. |
| `quadrille` | Trace fonctions, points et vecteurs sur papier quadrillé. |
| `quick-maths` | Ajoute des raccourcis pour écrire plus vite les formules. |
| `quick-vertex` | Trace les régions admissibles de programmes linéaires à deux variables. |
| `quill` | Dessine des circuits quantiques. |
| `quonom` | Pose des divisions synthétiques de polynômes. |
| `riesketcher` | Dessine des sommes de Riemann. |
| `scribe` | Écrit des formules mathématiques en notation ASCII. |
| `slashion` | Écrit des fractions avec une barre oblique. |
| `stair-division` | Pose des divisions de polynômes par la méthode de Ruffini. |
| `statementsp` | Encadrés d'énoncés avec renvois croisés. |
| `super-suboptimal` | Utilise les exposants et indices Unicode dans les formules. |
| `symbolica` | Effectue des calculs symboliques et numériques. |
| `symbolist` | Crée une liste des symboles utilisés. |
| `teig` | Calcule les valeurs propres de matrices. |
| `theofig` | Environnements de théorèmes simples. |
| `theoframe` | Environnements de théorèmes encadrés. |
| `theoretic` | Met en forme théorèmes, lemmes et preuves. |
| `theorion` | Environnements de théorèmes multilingues personnalisables. |
| `thmbox` | Encadrés de théorèmes élégants. |
| `trivial` | Met en forme théorèmes et preuves. |
| `trompet` | Dessine des diagrammes lambda de Tromp. |
| `truthfy` | Crée des tables de vérité. |
| `tybloch` | Place des états sur une sphère de Bloch. |
| `typcas` | Calcul symbolique avec étapes détaillées. |
| `typed-physics` | Diagrammes de mécanique et circuits électriques. |
| `typed-smiles` | Dessine des molécules à partir de notation SMILES. |
| `typograph-zx` | Notation ZX-calculus pour typograph. |
| `typsium` | Écrit formules et réactions chimiques. |
| `typsium-atomic` | Dessine atomes et configurations électroniques. |
| `vartable` | Crée des tableaux de variations. |
| `vmesh` | Visualise des maillages Gmsh 2D et 3D. |
| `voronay` | Triangulations de Delaunay et diagrammes de Voronoï. |
| `whalogen` | Écrit des formules chimiques. |
| `wicked` | Contractions de Wick en physique. |
| `xyzrender-rustyp` | Dessine des structures moléculaires XYZ. |
| `ytableausp` | Crée des tableaux de Young. |
| `zap` | Dessine des circuits électroniques normalisés. |

### Code et informatique (55)

| Paquet | Ce qu'il fait |
|---|---|
| `algo` | Met en forme des algorithmes. |
| `algol-code` | Met en forme des algorithmes en pseudo-code. |
| `algorithmic` | Met en forme du pseudo-code à la manière de LaTeX. |
| `algorythmst` | Affiche des blocs de pseudo-code soignés. |
| `ansi-render` | Affiche du texte de terminal avec ses couleurs. |
| `asciim` | Affiche la table ASCII sous forme de grille. |
| `bytefield` | Dessine des en-têtes de protocoles réseau et des registres. |
| `codedis` | Affiche du code source. |
| `codeforth` | Coloration syntaxique du langage Forth. |
| `codegds` | Coloration syntaxique du langage GDScript. |
| `codelst` | Affiche du code source avec numéros de ligne. |
| `codez` | Annote des blocs de code avec marques et formes. |
| `codly` | Présentation soignée du code avec numérotation et surlignage. |
| `codly-languages` | Icônes et réglages de langages pour codly. |
| `crudo` | Extrait des lignes d'un bloc de code. |
| `cvssc` | Calcule les scores de vulnérabilité CVSS. |
| `digidraw` | Dessine des chronogrammes de signaux numériques. |
| `dtree` | Affiche une arborescence de dossiers. |
| `ez-algo` | Écrit des algorithmes facilement. |
| `fervojo` | Dessine des diagrammes de syntaxe en rail. |
| `forensix` | Éléments de rapports d'investigation numérique. |
| `hyperscript` | Génère du HTML pour l'export web de Typst. |
| `hypraw` | Blocs de code allégés pour l'export HTML. |
| `iridis` | Colore les parenthèses correspondantes dans le code. |
| `itemplate` | Modèle de page HTML expérimental. |
| `iversymbols` | Symboles des langages de programmation APL et apparentés. |
| `jumble` | Fonctions de hachage (empreintes) de textes. |
| `keyle` | Affiche des raccourcis clavier stylés. |
| `lambdabe` | Analyse et affiche des expressions de lambda-calcul. |
| `lambdabus` | Analyse et simplifie des expressions de lambda-calcul. |
| `lamportian-dramatis` | Diagrammes d'échanges de messages entre systèmes répartis. |
| `lovelace` | Algorithmes en pseudo-code. |
| `lure` | Analyse et normalise des adresses web. |
| `mandolin` | Convertit des pages de manuel Unix en PDF. |
| `nassi` | Diagrammes de structure de programmes (Nassi-Shneiderman). |
| `nutthead-ebnf` | Affiche des grammaires formelles (EBNF). |
| `ott-ng` | Affiche des définitions de langages de programmation (Ott). |
| `percencode` | Encode et décode les caractères dans les adresses web. |
| `polly-style` | Preuves pour un cours de programmation (CMU). |
| `relescope` | Extrait et affiche une partie précise d'un code source. |
| `rivet` | Dessine des schémas de registres et d'instructions processeur. |
| `salsa-dip` | Crée des étiquettes de broches pour puces électroniques. |
| `siddhi-syntax` | Colore la syntaxe du langage Siddhi. |
| `simple-csp` | Symboles d'opérateurs pour la théorie des processus CSP. |
| `simplebnf` | Met en forme des grammaires BNF. |
| `slr8` | Visualise les étapes d'un analyseur syntaxique SLR. |
| `sourcecraft` | Dessine des diagrammes de classes UML. |
| `sourcerer` | Blocs de code source personnalisables. |
| `stack-pointer` | Visualise pas à pas l'exécution d'un programme. |
| `structogrammer` | Dessine des structogrammes (Nassi-Shneiderman). |
| `typed-dsa` | Diagrammes de structures de données et d'algorithmes. |
| `vhdl-parse` | Extrait des informations d'un fichier VHDL. |
| `vuln-calc` | Calcule des scores de vulnérabilité CVSS 4.0. |
| `wavy` | Dessine des chronogrammes numériques avec WaveDrom. |
| `zebraw` | Affiche du code avec numéros de ligne et surlignage. |

### Présentations (20)

| Paquet | Ce qu'il fait |
|---|---|
| `animo` | Crée des présentations animées en HTML et PDF. |
| `chalkdeck` | Présentations au style tableau noir ou cahier. |
| `lineal` | Diapositives de présentation élégantes. |
| `minideck` | Diapositives simples. |
| `monet-touying-cdu` | Thème de diapositives pour l'université CDU. |
| `mosaic` | Diapositives avec thèmes et composants. |
| `navigator` | Outils de navigation pour présentations. |
| `polylux` | Création de diapositives de présentation. |
| `presentate` | Crée des diapositives qui s'affichent progressivement. |
| `presio` | Ajoute notes d'orateur et médias aux diapositives PDF pour presio.xyz. |
| `sanor` | Crée des diapositives animées précises. |
| `shiroa` | Crée des livres en ligne modernes. |
| `shiroa-mdbook` | Thème façon mdbook pour les livres en ligne Shiroa. |
| `shiroa-starlight` | Thème façon Starlight pour les livres en ligne Shiroa. |
| `shuimu-touying-zen` | Modèle de diapositives de physique de l'université Tsinghua. |
| `slipst` | Crée des présentations qui défilent façon slipshow. |
| `touying` | Crée des présentations de diapositives. |
| `typsite` | Bibliothèque standard pour sites web Typsite. |
| `typstage` | Présentations HTML animées et support PDF. |
| `yap` | Ajoute vidéos et notes d'orateur aux documents. |

### Enseignement et examens (44)

| Paquet | Ce qu'il fait |
|---|---|
| `ankify` | Génère des cartes de révision Anki depuis un document. |
| `answerly` | Note et met en forme les réponses d'exercices. |
| `arabic-exam-kit` | Mises en page d'examens de mathématiques en arabe. |
| `blockst` | Dessine des blocs de programmation Scratch pour l'enseignement. |
| `cartao` | Crée des cartes de révision imprimables. |
| `codepoint` | Outils pour TP et examens de programmation. |
| `conic-toan` | Figures, courbes et tableaux de variations pour les maths. |
| `diorama` | Illustrations de scènes pour problèmes de maths. |
| `ditto` | Composants pour fiches d'exercices de maths. |
| `dol-theme` | Modèle de sujets pour l'Olympiade allemande de linguistique. |
| `ergo` | Environnements pour notes de cours et devoirs. |
| `examit` | Modèle d'examen inspiré de LaTeX exam. |
| `examy` | Examens et quiz avec numérotation et barème automatiques. |
| `exercise-bank` | Banque d'exercices avec solutions et filtres. |
| `exercism` | Organise des exercices et reporte les solutions. |
| `ferrmat` | Boîte à outils visuelle en portugais pour cours et examens. |
| `figchild` | Illustrations colorées pour activités d'enfants. |
| `functable` | Tableaux de signes et de variations de fonctions, style lycée français. |
| `geomtools` | Dessine règles, équerres, rapporteurs et compas sur une figure. |
| `intsketcher` | Illustre les sommes de Riemann pour les cours d'intégrales. |
| `kanjimo` | Fiches d'entraînement à l'écriture des kanjis. |
| `longops` | Opérations posées et calculs détaillés pour l'école. |
| `mcx` | Examens QCM avec questions mélangées. |
| `meshpad` | Fonds quadrillés, lignés ou pointillés pour fiches imprimables. |
| `moustaches` | Statistiques scolaires françaises : tableaux et diagrammes. |
| `oak-grove` | Recueils d'exercices avec solutions. |
| `profmaquette-minimal` | Crée des fiches d'exercices avec corrigés affichables ou non. |
| `quick-cards` | Crée des cartes mémoire (flashcards) personnalisables. |
| `quiztime` | Crée de petits quiz. |
| `ratsch-bmim` | Modèles de documents universitaires d'UMIT Tirol. |
| `sang-math` | Macros d'examens vietnamiens en maths, physique et chimie. |
| `scrutinize` | Construit des examens et contrôles. |
| `sdust` | Pages de garde aux couleurs de l'université SDU. |
| `shuxuejuan` | Crée des quiz et examens au format chinois. |
| `siefken-syllabus` | Modèle de plan de cours universitaire. |
| `solving-physics` | Présente la résolution d'un problème de physique. |
| `stacked` | Dessine des empilements de cubes et des dés. |
| `taskize` | Dispose des exercices en colonnes. |
| `tinyset` | Modèle de devoir de mathématiques. |
| `ttt-utils` | Outils pour faciliter le travail des enseignants. |
| `tutor` | Utilitaires pour créer des examens. |
| `typ2anki` | Convertit des fichiers Typst en cartes Anki. |
| `v-exam` | Compose et mélange des examens au format vietnamien. |
| `wordc` | Crée des fiches de vocabulaire et synonymes. |

### Musique et loisirs (31)

| Paquet | Ce qu'il fait |
|---|---|
| `bar-point` | Dessine des positions de backgammon. |
| `burik` | Illustre les algorithmes du Rubik's Cube. |
| `chordx` | Écrit des paroles de chansons avec accords. |
| `codex-woltiensis` | Mise en page de recueils de chansons étudiantes. |
| `conchord` | Paroles avec accords, diagrammes et tablatures de guitare. |
| `dragonling` | Contenus de jeu Donjons et Dragons. |
| `fretwork` | Tablatures de guitare de qualité professionnelle. |
| `hane` | Dessine des diagrammes de parties de go. |
| `lets-go` | Dessine des plateaux de jeu de go. |
| `magic-cubes` | Représente des Rubik's cubes. |
| `mazed` | Génère des labyrinthes. |
| `messeji` | Met en page des conversations de messagerie. |
| `mino` | Affiche des grilles de Tetris. |
| `mise-en-place` | Diagrammes de recettes de cuisine. |
| `nonodraw` | Grilles de logigrammes (nonogrammes). |
| `ourchat` | Imite des messages de messagerie. |
| `pf2e-style` | Mise en page pour jeu de rôle Pathfinder. |
| `quetta` | Écrit en alphabet elfique Tengwar. |
| `riichinator` | Affiche des mains et parties de mahjong riichi. |
| `scorify` | Affiche des partitions de musique. |
| `scoryst` | Grave des partitions depuis plusieurs formats musicaux. |
| `songb` | Crée des recueils de chansons avec accords. |
| `staves` | Dessine des clés et armures musicales. |
| `sudokyst` | Affiche et analyse des grilles de sudoku. |
| `swaralipi` | Notation de la musique classique indienne. |
| `tierpist` | Crée des classements par niveaux (tier lists). |
| `typed-scores` | Grave des partitions de musique. |
| `underhell` | Modèle de documents pour jeux de rôle. |
| `wubrg` | Affiche les symboles de mana de Magic. |
| `yi` | Dessine des hexagrammes du Yi Jing. |
| `yinsh-record` | Dessine des parties du jeu Yinsh. |

### Outils pour auteurs (57)

| Paquet | Ce qu'il fait |
|---|---|
| `backtrack` | Détecte la version de Typst utilisée. |
| `basalt-backlinks` | Génère des renvois inverses entre parties d'un document. |
| `big-todo` | Insère des repères « à faire » bien visibles. |
| `bullseye` | Adapte le style selon la sortie HTML ou PDF. |
| `caterpillar` | Définit ses propres règles de syntaxe dans le texte. |
| `colophon` | Produit un PDF d'audit avec nombre de mots et figures. |
| `contexture` | Base technique pour documents liés : glossaire, index, annexes. |
| `dashy-todo` | Affiche des notes « à faire » dans la marge. |
| `debug-city` | Aide à corriger les entrées de bibliographie. |
| `deep-dish` | Fusionne des dictionnaires de données imbriqués. |
| `defined` | Compilation conditionnelle selon des variables. |
| `diffst` | Compare deux documents côte à côte. |
| `drafting` | Notes en marge et positionnement libre. |
| `eeaabb` | Mesure la taille et la position des éléments. |
| `elembic` | Cadre pour créer ses propres éléments personnalisés. |
| `fig-plucker` | Exporte séparément les figures d'un document. |
| `funarray` | Fonctions pratiques pour manipuler des listes dans les templates. |
| `hallon` | Petites fonctions utilitaires pour écrire des templates. |
| `idwtet` | Montre côte à côte du code Typst et son rendu. |
| `ipsum` | Génère du faux texte pour tester une mise en page. |
| `jiexi` | Générateur d'analyseurs syntaxiques pour templates avancés. |
| `kauderwelsch` | Génère du faux texte multilingue pour tester une mise en page. |
| `kleene` | Outil d'analyse syntaxique de texte. |
| `kouhu` | Génère du faux texte en chinois. |
| `latedef` | Utiliser une valeur avant de la définir. |
| `latex-compat` | Commandes de compatibilité avec LaTeX. |
| `layout-ltd` | Limite les passes de mise en page pour déboguer. |
| `loom` | Moteur de données réactives pour templates complexes. |
| `melt` | Inspecte les caractéristiques des polices. |
| `mephistypsteles` | Exécute Typst à l'intérieur de Typst. |
| `nexus-tools` | Fonctions utilitaires pour développer des templates. |
| `oicana` | Outil pour templates PDF utilisés depuis plusieurs langages. |
| `parsely` | Analyse des équations en arbres d'expressions. |
| `patstdlib` | Collection de petits outils variés. |
| `prequery` | Extrait des métadonnées pour un prétraitement externe. |
| `pubmatter` | Gère et affiche les auteurs et affiliations d'une publication. |
| `retrofit` | Ajoute des renvois cliquables depuis la bibliographie vers les citations. |
| `sanity` | Repère figures non citées, sources non citées et étiquettes perdues. |
| `scaffolder` | Affiche les contours des zones de page pour la mise au point. |
| `sela` | Simplifie la sélection d'éléments pour les règles de style. |
| `self-example` | Affiche un code Typst et son résultat côte à côte. |
| `sertyp` | Convertit des contenus Typst vers un format binaire et inversement. |
| `showman` | Affiche un code Typst avec son résultat. |
| `simple-todo` | Ajoute des marqueurs « à faire » et leur liste. |
| `t4t` | Utilitaires pour auteurs de paquets Typst. |
| `tally` | Gère automatiquement les « à faire » du document. |
| `tidy` | Génère la documentation de paquets Typst. |
| `typsy` | Outils de programmation avancés pour Typst. |
| `typwire` | Encodeur de données pour plugins Typst. |
| `uniwarn` | Permet aux paquets d'émettre des avertissements. |
| `visual-cetz` | Montre un dessin CeTZ et son code côte à côte. |
| `weave` | Aide à enchaîner des fonctions. |
| `wordometer` | Compte les mots et donne des statistiques du document. |
| `wrap-indent` | Applique une fonction au contenu par simple indentation. |
| `xodec` | Donne le nom Typst des symboles. |
| `zebra-notes` | Notes de relecture numérotées et tableau récapitulatif. |
| `zettyp-lsp` | Déclare des informations pour l'éditeur (LSP). |

### Autre (3)

| Paquet | Ce qu'il fait |
|---|---|
| `checkitoff` | Remplit des listes de contrôle pour articles médicaux. |
| `dice` | Génère des nombres aléatoires reproductibles. |
| `tiefbubbles` | Dessine des bulles de discussion façon messagerie. |

