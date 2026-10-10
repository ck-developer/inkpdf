# Feature Specification: Paquets Typst intégrés au service

**Feature Branch**: `002-typst-packages`

**Created**: 2026-10-10

**Status**: Draft

**Input**: User description: « Permettre aux templates d'utiliser des paquets Typst
(ex. `#import "@preview/cetz:0.3.4"`) sans aucun accès réseau. » Après étude
(`research/`, `synthese.md`), décision de l'utilisateur : les paquets utiles sont **intégrés à
l'application dès sa construction**, selon une **liste fixée et validée** (priorité P1 de
`research/carte-paquets.md`, sans `cmarker`) ; pas de dossier de paquets partagé, pas de
paquet de helpers maison, pas de paquets P3 pour le moment.

## User Scenarios & Testing *(mandatory)*

### Acteurs

- **Auteur de template** : écrit les templates ; utilise les paquets intégrés.
- **Mainteneur d'inkpdf** : décide quels paquets et quelles versions sont intégrés.
- **Application appelante** : demande des générations ; peut consulter les paquets disponibles.

### User Story 1 - Utiliser un paquet intégré dans un template (Priority: P1)

Un auteur veut un QR code, un graphique ou un montant bien formaté dans son document. Il
écrit dans son template l'import standard du paquet (`@preview/nom:version`), exactement comme
dans la documentation du paquet. Sans rien déposer d'autre dans le dossier du template, les
générations produisent le document attendu, sans aucun accès réseau.

**Why this priority**: c'est la raison d'être de la fonctionnalité.

**Independent Test**: déposer un template qui importe un paquet de la liste, demander une
génération réseau coupé, vérifier que le PDF contient le rendu du paquet.

**Acceptance Scenarios**:

1. **Given** un paquet présent dans la liste intégrée, **When** un template l'importe avec son
   nom et sa version exacte et qu'une génération valide est demandée, **Then** le PDF est
   produit et contient le rendu du paquet.
2. **Given** un paquet intégré qui dépend d'autres paquets, **When** il est importé, **Then** ses
   dépendances sont résolues sans action de l'auteur.
3. **Given** un template utilisant des paquets intégrés, **When** deux générations identiques
   sont demandées, **Then** les deux PDF sont identiques octet pour octet.
4. **Given** le service démarré sans aucun accès réseau, **When** des générations utilisant des
   paquets sont demandées, **Then** elles aboutissent toutes.

---

### User Story 2 - Comprendre immédiatement qu'un paquet n'est pas disponible (Priority: P2)

Un auteur importe un paquet absent de la liste, ou une version qui n'y figure pas. Il doit
comprendre l'erreur sans lire le code du service : le message nomme le paquet et la version
demandés et, si d'autres versions du même paquet sont disponibles, les cite.

**Why this priority**: la liste est volontairement restreinte ; un refus doit être explicite.

**Independent Test**: déposer un template important un paquet hors liste, puis un autre important
une version absente, et vérifier les messages.

**Acceptance Scenarios**:

1. **Given** un template qui importe un paquet hors de la liste, **When** le template est chargé,
   **Then** il est signalé comme invalide avec un diagnostic nommant le paquet et la version, et
   indiquant que seuls les paquets intégrés sont disponibles.
2. **Given** un template qui importe `@preview/zero:0.5.0` alors que seule la `0.7.1` est
   intégrée, **When** l'erreur est signalée, **Then** le message cite la version `0.7.1` comme
   disponible.
3. **Given** un import de paquet construit dynamiquement (non détectable au chargement), **When**
   une génération est demandée, **Then** elle échoue avec le même diagnostic, sans accès réseau.
4. **Given** une erreur à l'intérieur du code d'un paquet intégré, **When** elle est signalée,
   **Then** le diagnostic indique le paquet, le fichier et la ligne concernés.

---

### User Story 3 - Découvrir les paquets disponibles (Priority: P3)

Un auteur ou une application appelante consulte la liste des paquets intégrés (nom, version,
description, licence) via l'API et via la documentation, pour savoir ce qu'il peut utiliser.

**Why this priority**: utile pour écrire des templates, mais la génération fonctionne sans.

**Independent Test**: appeler la route de liste des paquets et vérifier qu'elle renvoie exactement
la liste intégrée.

**Acceptance Scenarios**:

1. **Given** le service démarré, **When** l'appelant demande la liste des paquets, **Then** il
   obtient chaque paquet intégré avec son namespace, son nom, sa version, sa description et sa
   licence, dépendances comprises.
2. **Given** la documentation des auteurs, **When** un auteur la consulte, **Then** il y trouve la
   liste des paquets intégrés avec, pour chacun, ce qu'il fait et un exemple d'import.

---

### User Story 4 - Faire évoluer la liste en toute sécurité (Priority: P2)

Le mainteneur ajoute un paquet ou une nouvelle version d'un paquet existant. Il modifie la liste
fixée dans le dépôt ; la construction de l'application vérifie l'intégrité de chaque paquet et
qu'il fonctionne avec le moteur, sinon elle échoue. Les templates qui utilisent une ancienne
version continuent de fonctionner.

**Why this priority**: sans règle d'évolution, la liste se dégrade ou casse les templates.

**Independent Test**: ajouter une version à la liste, construire, vérifier qu'elle est disponible
et que l'ancienne l'est toujours ; altérer une empreinte, vérifier que la construction échoue.

**Acceptance Scenarios**:

1. **Given** la liste fixée, **When** le mainteneur ajoute une nouvelle version d'un paquet déjà
   intégré, **Then** les deux versions sont disponibles après construction.
2. **Given** une archive de paquet dont le contenu ne correspond pas à l'empreinte notée,
   **When** l'application est construite, **Then** la construction échoue en nommant le paquet.
3. **Given** un paquet de la liste qui ne fonctionne pas avec la version du moteur, **When** les
   vérifications de construction s'exécutent, **Then** elles échouent en nommant le paquet.

---

### Edge Cases

- Un import sans version (`@preview/zero`) ou avec une version partielle (`0.7`) : refusé avec un
  diagnostic, aucune version n'est choisie à la place de l'auteur.
- Un import avec un autre namespace que `@preview` (ex. `@local/...`) : refusé comme un paquet
  absent.
- Un paquet intégré tente de lire un fichier hors de son propre dossier (template, autre paquet,
  système) : refusé. Un template peut en revanche transmettre explicitement une image ou des
  données à un paquet.
- Un paquet dépend de deux versions différentes d'un autre paquet : les deux versions sont
  intégrées et coexistent.
- Un template embarque lui-même un dossier `packages/` : il n'est pas utilisé pour résoudre les
  imports (seuls les paquets intégrés le sont).
- Un paquet contient des polices : elles ne sont pas chargées (seules les polices du service et
  celles du dossier `fonts/` du template le sont).
- Un paquet effectue un calcul très long : la génération reste soumise à la durée maximale ; la
  limite connue (certains calculs internes ne s'interrompent pas immédiatement) est documentée.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Le service DOIT mettre à disposition de tous les templates un ensemble fixe de
  paquets Typst, intégré à l'application lors de sa construction.
- **FR-002**: Cet ensemble DOIT comprendre les 21 paquets validés (P1, voir Assumptions) et
  toutes leurs dépendances, chacun dans une version exacte.
- **FR-003**: Un import `@preview/nom:version` DOIT être résolu uniquement parmi les paquets
  intégrés, avec une correspondance exacte du nom et de la version.
- **FR-004**: Le service NE DOIT jamais télécharger de paquet ni accéder au réseau pendant son
  fonctionnement ; aucun paquet ne doit être lu ailleurs que dans l'ensemble intégré.
- **FR-005**: La liste des paquets intégrés DOIT être définie dans un fichier versionné du dépôt
  indiquant pour chaque paquet son nom, sa version, une empreinte d'intégrité et sa licence.
- **FR-006**: La construction de l'application DOIT vérifier l'empreinte de chaque paquet et
  échouer si l'une ne correspond pas.
- **FR-007**: Les vérifications automatiques du projet DOIVENT prouver que chaque paquet intégré
  s'importe et produit un document avec la version du moteur utilisée ; un paquet qui échoue ne
  peut pas figurer dans la liste.
- **FR-008**: Faire évoluer un paquet DOIT se faire par ajout d'une version ; retirer une version
  de la liste est une décision explicite, signalée comme incompatible pour les templates qui
  l'utilisent.
- **FR-009**: Au chargement d'un template, les imports de paquets écrits littéralement DOIVENT
  être contrôlés ; un import vers un paquet ou une version non intégrés rend le template invalide
  avec un diagnostic nommant le paquet, la version et, le cas échéant, les versions disponibles.
- **FR-010**: Au rendu, un import non résolu (y compris construit dynamiquement) DOIT faire
  échouer la génération avec le même diagnostic.
- **FR-011**: Un diagnostic d'erreur survenant dans le code d'un paquet DOIT indiquer le paquet,
  le fichier et la ligne.
- **FR-012**: Un paquet NE DOIT pouvoir lire que ses propres fichiers ; un template peut lui
  transmettre explicitement une image, des données ou un chemin vers ses propres fichiers.
- **FR-013**: Le service DOIT exposer une route listant les paquets intégrés (namespace, nom,
  version, description, licence), décrite dans le document OpenAPI.
- **FR-014**: Les PDF produits avec des paquets DOIVENT rester déterministes.
- **FR-015**: Les générations utilisant des paquets DOIVENT rester soumises aux bornes existantes
  (taille de requête, durée, concurrence).
- **FR-016**: Les licences des paquets intégrés DOIVENT être conservées dans l'application et
  listées dans la documentation.
- **FR-017**: La documentation des auteurs DOIT présenter les paquets intégrés (ce que fait
  chacun, exemple d'import) et les limites (version exacte, pas d'autres paquets, accès aux
  fichiers).
- **FR-018**: Le dépôt DOIT fournir au moins un template d'exemple utilisant des paquets intégrés
  (au minimum un QR code et un montant formaté), servant de référence et de test.

### Key Entities *(include if feature involves data)*

- **Paquet intégré** : paquet Typst figé dans l'application ; identifié par namespace, nom et
  version ; porte une description, une licence et une empreinte d'intégrité.
- **Liste des paquets** : fichier versionné du dépôt qui définit exactement les paquets intégrés ;
  seule source de vérité.
- **Référence de paquet** : `@namespace/nom:version` écrit dans un template ou un paquet ; résolue
  à l'identique ou refusée.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100 % des paquets de la liste s'importent et produisent un document lors des
  vérifications automatiques.
- **SC-002**: 100 % des générations utilisant des paquets aboutissent sans aucune tentative
  d'accès réseau.
- **SC-003**: 100 % des imports non disponibles produisent un message nommant le paquet et la
  version en cause.
- **SC-004**: Un auteur ajoute un QR code ou un montant formaté à un template en moins de
  5 minutes en suivant la documentation.
- **SC-005**: Le temps de génération du template de démonstration existant (sans paquet) ne se
  dégrade pas de plus de 5 %.
- **SC-006**: Le template d'exemple utilisant des paquets se génère en moins d'une seconde dans
  95 % des cas.
- **SC-007**: Une archive de paquet altérée est détectée à 100 % lors de la construction.

## Assumptions

- **Liste validée (2026-10-10)** — 21 paquets, tous dans leur dernière version publiée à cette
  date (`research/carte-paquets.md`) : `tiaoma`, `zebra`, `qrypst`, `codetastic`, `sepay`,
  `zero`, `oxifmt`, `frogst`, `ibanator`, `datify`, `linguify`, `tabut`, `tablem`, `cetz`,
  `cetz-plot`, `lilaq`, `primaviz`, `showybox`, `framefit`, `modern-mailmerge`, `payqr-swiss`.
  Avec leurs 8 dépendances (`datify-core`, `elembic`, `komet` ×2, `rustycure`, `suiji`, `tiptoe`,
  `zero` 0.6.1) : 29 paquets, environ 6 Mo. Si un paquet échoue aux vérifications avec le moteur
  actuel, il est retiré de la liste et signalé (pas de correctif local du paquet).
- `cmarker` est exclu (il peut exécuter du code contenu dans le Markdown). Les paquets P2 pourront
  être ajoutés plus tard par simple évolution de la liste ; les P3 ne sont pas intégrés.
- Seul le namespace `@preview` (Typst Universe) est servi.
- Les paquets sont téléchargés uniquement au moment de la construction de l'application ;
  l'accès réseau pendant la construction est acceptable.
- La taille de l'application n'est pas un critère ; la rapidité de génération l'est.
- Conformité à la constitution : les paquets sont des ressources embarquées par le binaire
  (principe II), rien n'est téléchargé au fonctionnement (principes II et IV), et la liste ne
  change qu'avec une nouvelle version de l'application, les templates restant modifiables à
  chaud (principe V). Aucun amendement n'est nécessaire.
- Hors périmètre : dossier de paquets partagé dans le volume, paquets embarqués dans un template,
  paquet de helpers maison, paquets P3, séparation aperçu / document final, PDF/A, Factur-X.
