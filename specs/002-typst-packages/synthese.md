# Synthèse : où on en est, ce qu'on fait, dans quel ordre

*2026-10-10 — résumé des échanges après la V1, à lire sans connaître Rust ni Typst.*

## 1. Où on en est

La **V1 d'inkpdf est livrée** (PR #1, fusionnée). Le service :
- lit des **templates** posés dans un dossier (un template = un dossier avec le document `main.typ`, la description des données attendues `schema.json`, et un nom `template.json`) ;
- reçoit des **données JSON** (le contenu du document + des réglages d'apparence), les vérifie, et **renvoie un PDF** en quelques dizaines de millisecondes ;
- prend en compte un template ajouté ou modifié **sans redémarrer** ;
- n'a **aucun accès à Internet** pendant la génération (règle de sécurité).

**La limite actuelle** : un template ne peut utiliser que les fonctions de base de Typst (texte, tableaux, images, formes). Pas de QR code, pas de graphique, pas de montant formaté « 1 234,56 € », pas de nombre en toutes lettres. Ces fonctions existent dans l'écosystème Typst sous forme de **paquets** (1 665 sur Typst Universe), mais inkpdf les refuse aujourd'hui car ils se téléchargent normalement depuis Internet.

## 2. Ce qu'on a décidé

| Sujet | Décision |
|---|---|
| Philosophie | Avancer par **briques simples**, une à la fois ; pas de « framework de design » pour l'instant. |
| Paquets | Les paquets utiles sont **intégrés à l'application dès sa construction** : un template écrit `#import "@preview/zero:0.7.1"` et ça marche, sans rien déposer, sans Internet. |
| Liste | La liste des paquets intégrés est **fixée** (nom + version exacte + empreinte de sécurité) dans le dépôt, après validation par toi. |
| Taille | La taille de l'image n'est **pas un critère** : le service doit surtout générer vite. |
| Versions | Mettre à jour un paquet = **ajouter** la nouvelle version à côté de l'ancienne ; un template existant ne casse jamais. |
| Helpers maison | Plus tard (voir étape 5). Ce sera un paquet comme un autre. |
| Aperçu / document final | `render` devient l'aperçu ; une nouvelle route produira le document final. Sujet séparé (étape 4). |

## 3. Comment ça marchera (image simple)

```
Construction de l'image (une fois, avec Internet)
  liste fixée ──► téléchargement + vérification ──► paquets rangés DANS l'image
                                                     + test : chaque paquet doit
                                                       compiler, sinon refusé

Fonctionnement (sans Internet)
  template « facture » ──import "@preview/tiaoma:0.3.0"──► paquet trouvé dans l'image ──► PDF
```

## 4. La carte des paquets

Fichier : `research/carte-paquets.md`. Les 827 bibliothèques sont classées par **besoin** et par **priorité** :

- **P1 — 22 paquets, à faire en premier** :
  - *Codes-barres / QR* : `tiaoma` (tous formats), `zebra`, `qrypst`, `codetastic`, `sepay` (QR de virement SEPA)
  - *Montants et nombres* : `zero`, `oxifmt` (formatage), `frogst` (montant en toutes lettres en français), `ibanator` (IBAN)
  - *Dates et langues* : `datify` (dates en français), `linguify` (textes multilingues)
  - *Tableaux* : `tabut` (tableau depuis des données), `tablem`
  - *Graphiques* : `cetz`, `cetz-plot`, `lilaq`, `primaviz`
  - *Mise en page* : `showybox` (encadrés), `framefit` (texte qui s'adapte à son cadre)
  - *Documents* : `modern-mailmerge` (publipostage), `payqr-swiss` (QR-facture suisse), `cmarker` (Markdown → document, avec une précaution de sécurité)
- **P2 — 84 paquets** : utiles pour certains documents (styles de tableaux, encadrés, icônes, Gantt, Excel → tableau, nombres en lettres multilingues…).
- **P3 — 619 paquets** : spécialisés (maths, chimie, slides, musique, outils de développeur) ; sans danger.
- **X — 102 exclus** : exécutent un autre langage (JavaScript, Python…), utilisent la date du jour (PDF non reproductible), licence GPL/AGPL, obsolètes, ou exigent des polices installées.

## 5. Ce qu'on va faire, dans l'ordre

| Étape | Quoi | Pour toi, concrètement |
|---|---|---|
| **1** | **Feature 002 — paquets intégrés (P1)** : l'application sait trouver un paquet intégré ; la liste P1 est fixée et chaque paquet est testé automatiquement ; erreurs claires si un template demande un paquet absent ; une route d'API liste les paquets disponibles ; doc pour les auteurs de templates. | Un template peut faire des QR codes, des graphiques, des montants bien formatés. |
| **2** | Élargir à **P2** (et éventuellement P3) : on ajoute des lignes à la liste, le test automatique écarte ceux qui ne compilent pas. | Plus de briques, sans nouveau développement. |
| **3** | **Dossier `packages/` partagé** à côté des templates, pour les paquets non intégrés ou faits maison, sans reconstruire l'application. Demande une petite modification de la constitution. | Ajouter un paquet = déposer un dossier. |
| **4** | **Aperçu / document final** : `render` = aperçu ; nouvelle route = document final (à préciser : PDF/A pour l'archivage ? filigrane « APERÇU » ?). | Deux usages distincts. |
| **5** | **Helpers maison** (`@inkpdf/…`) : fonctions communes à tous les templates (montants, dates, adresses…), puis plus tard des composants dont l'apparence suit les réglages `design`. | Écrire un template devient plus rapide. |

**Améliorations repérées, à placer plus tard** : données d'exemple par template (aperçu sans rien envoyer), PDF/A et PDF/UA (archivage, accessibilité), date du jour maîtrisée, limite de durée pour certains paquets lourds (WASM), Factur-X (facture électronique, demande un traitement en plus de Typst).

## 6. Ce qu'on NE fait PAS maintenant

Framework de design ou de composants, génération par lots, stockage des PDF, téléchargement de paquets pendant le fonctionnement, Factur-X.

## 7. Ce que tu dois décider

1. **Valider la liste P1** (retirer ou ajouter des paquets) — c'est le point de départ de l'étape 1.
2. **P3** : les intégrer aussi dès l'étape 2 (ils ne coûtent que de la taille, le test automatique écarte ceux qui ne marchent pas) ou les laisser de côté ?
3. **`cmarker`** (Markdown) : le garder en imposant le mode sécurisé, ou l'exclure ?

Ensuite : la spécification 002 (actuellement écrite pour « un dossier de paquets par template ») sera **réécrite** autour de l'intégration, puis on suit le cycle habituel : plan → tâches → implémentation.

## Petit lexique

- **Template** : un modèle de document (ex. facture) = un dossier.
- **Paquet** : une brique de code Typst réutilisable (ex. « dessiner un QR code »), écrite par la communauté.
- **Image** : l'application emballée pour être déployée (Docker).
- **Version figée** : on note `tiaoma 0.3.0` exactement ; la brique ne change jamais sans qu'on le décide.
- **WASM** : du code compilé à l'intérieur de certains paquets ; rapide, mais impossible à interrompre s'il tourne trop longtemps.
