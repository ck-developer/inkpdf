# Feature Specification: Paquets Typst intégrés au service

**Feature Branch**: `002-typst-packages`

**Created**: 2026-10-10

**Status**: Draft

**Input**: User description: « Permettre aux templates d'utiliser des paquets Typst
(ex. `#import "@preview/cetz:0.3.4"`) sans aucun accès réseau. » Après étude
(`research/`, `synthese.md`), décision de l'utilisateur : les paquets utiles sont **intégrés à
l'application dès sa construction**, selon une **liste fixée et validée** (priorité P1 de
`research/carte-paquets.md`, sans `cmarker`) ; pas de dossier de paquets partagé, pas de
paquet de helpers maison, pas de paquets P3 pour le moment. Dans les templates, un paquet
s'importe par son seul nom, **sans version** (décision du 2026-10-10).

## User Scenarios & Testing *(mandatory)*

### Acteurs

- **Auteur de template** : écrit les templates ; utilise les paquets intégrés.
- **Mainteneur d'inkpdf** : décide quels paquets et quelles versions sont intégrés.
- **Application appelante** : demande des générations ; peut consulter les paquets disponibles.

### User Story 1 - Utiliser un paquet du service dans un template (Priority: P1)

Un auteur veut un QR code, un graphique ou un montant bien formaté dans son document. Il écrit
dans son template l'import du paquet **par son seul nom**, par exemple `#import "@preview/zero"`.
Il n'écrit jamais de version : le service utilise celle qu'il a installée. Sans rien déposer
d'autre, les générations produisent le document attendu, sans aucun accès réseau.

**Why this priority**: c'est la raison d'être de la fonctionnalité.

**Independent Test**: déposer un template qui importe un paquet du service par son nom, demander
une génération réseau coupé, vérifier que le PDF contient le rendu du paquet.

**Acceptance Scenarios**:

1. **Given** un paquet mis à disposition par le service, **When** un template l'importe par son
   nom (`@preview/zero`) et qu'une génération valide est demandée, **Then** le PDF est produit
   avec la version installée de ce paquet.
2. **Given** un paquet du service qui dépend d'autres paquets, **When** il est importé, **Then**
   ses dépendances sont résolues sans action de l'auteur.
3. **Given** un template utilisant des paquets du service, **When** deux générations identiques
   sont demandées, **Then** les deux PDF sont identiques octet pour octet.
4. **Given** le service démarré sans aucun accès réseau, **When** des générations utilisant des
   paquets sont demandées, **Then** elles aboutissent toutes.

---

### User Story 2 - Être prévenu d'un import incorrect (Priority: P2)

Un auteur importe un paquet que le service ne met pas à disposition, ou écrit une version dans
l'import. Il doit comprendre l'erreur sans lire le code du service.

**Why this priority**: une seule façon d'importer, et une erreur claire dans tous les autres cas.

**Independent Test**: déposer un template qui importe un paquet inconnu, puis un autre qui écrit
une version, et vérifier les messages.

**Acceptance Scenarios**:

1. **Given** un template qui importe un paquet que le service ne met pas à disposition, **When**
   le template est chargé, **Then** il est signalé comme invalide avec un message qui nomme le
   paquet et renvoie à la liste des paquets disponibles.
2. **Given** un template qui écrit une version dans un import (`@preview/zero:0.7.1`), **When** le
   template est chargé, **Then** il est signalé comme invalide avec un message indiquant d'écrire
   `@preview/zero`.
3. **Given** une erreur à l'intérieur du code d'un paquet, **When** elle est signalée, **Then** le
   diagnostic indique le paquet, le fichier et la ligne concernés.

---

### User Story 3 - Découvrir les paquets disponibles (Priority: P3)

Un auteur ou une application appelante consulte la liste des paquets mis à disposition (nom,
version installée, description, licence) via l'API et via la documentation.

**Why this priority**: utile pour écrire des templates, mais la génération fonctionne sans.

**Independent Test**: appeler la route de liste des paquets et vérifier qu'elle renvoie exactement
les paquets mis à disposition.

**Acceptance Scenarios**:

1. **Given** le service démarré, **When** l'appelant demande la liste des paquets, **Then** il
   obtient chaque paquet mis à disposition avec son nom, la ligne d'import à écrire, sa version
   installée, sa description et sa licence.
2. **Given** la documentation des auteurs, **When** un auteur la consulte, **Then** il y trouve
   chaque paquet avec ce qu'il fait et un exemple d'import.

---

### User Story 4 - Faire évoluer la liste en toute sécurité (Priority: P2)

Le mainteneur ajoute un paquet ou change la version installée d'un paquet. Il modifie la liste
fixée dans le dépôt ; la construction de l'application vérifie l'intégrité de chaque paquet et
qu'il fonctionne avec le moteur, sinon elle échoue. Changer la version d'un paquet la change pour
tous les templates à la prochaine version du service.

**Why this priority**: sans règle d'évolution, la liste se dégrade ou casse les templates.

**Independent Test**: changer la version d'un paquet dans la liste, construire, vérifier que
les templates utilisent la nouvelle ; altérer une empreinte, vérifier que la construction
échoue.

**Acceptance Scenarios**:

1. **Given** la liste fixée, **When** le mainteneur remplace la version d'un paquet mis à
   disposition, **Then** après construction les templates qui l'importent utilisent la nouvelle
   version, sans modification.
2. **Given** une archive de paquet dont le contenu ne correspond pas à l'empreinte notée,
   **When** l'application est construite, **Then** la construction échoue en nommant le paquet.
3. **Given** un paquet de la liste qui ne fonctionne pas avec la version du moteur, **When** les
   vérifications de construction s'exécutent, **Then** elles échouent en nommant le paquet.

---

### Edge Cases

- Un import avec une version (`@preview/zero:0.7.1`), une version partielle ou un autre namespace
  que `@preview` (ex. `@local/x`) dans un template : refusé, le template est invalide.
- Un import construit par calcul (ex. `"@preview/" + nom`) : non pris en charge ; la génération
  échoue. Les imports doivent être écrits tels quels.
- Les paquets ajoutés uniquement comme dépendances d'autres paquets ne sont pas importables par
  les templates.
- Un paquet dépend d'une version d'un autre paquet différente de celle mise à disposition (ex.
  `lilaq` utilise `zero` 0.6.1, les templates ont `zero` 0.7.1) : les deux sont intégrées ; la
  version interne reste invisible pour les templates.
- Un paquet tente de lire un fichier hors de son propre dossier (template, autre paquet,
  système) : refusé. Un template peut en revanche transmettre explicitement une image ou des
  données à un paquet.
- Un template embarque lui-même un dossier `packages/` : il n'est pas utilisé pour résoudre les
  imports.
- Un paquet contient des polices : elles ne sont pas chargées (seules les polices du service et
  celles du dossier `fonts/` du template le sont).
- Un paquet effectue un calcul très long : la génération reste soumise à la durée maximale ; la
  limite connue (certains calculs internes ne s'interrompent pas immédiatement) est documentée.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Le service DOIT intégrer, lors de sa construction, un ensemble fixe de paquets
  Typst.
- **FR-002**: Cet ensemble DOIT comprendre les 20 paquets validés (voir Assumptions), **mis à
  disposition des templates dans une seule version chacun**, et toutes leurs dépendances.
- **FR-003**: Dans un template, un paquet DOIT s'importer **par son seul nom**
  (`@preview/nom`) ; le service utilise la version installée.
- **FR-004**: Un import qui précise une version, utilise un autre namespace, ou nomme un paquet
  non mis à disposition DOIT rendre le template invalide, avec un message qui nomme l'import
  fautif (fichier et ligne) et renvoie à la liste des paquets disponibles.
- **FR-005**: Le service NE DOIT jamais télécharger de paquet ni accéder au réseau pendant son
  fonctionnement.
- **FR-006**: La vérification et la résolution des imports d'un template DOIVENT avoir lieu une
  fois, au chargement du template, sans coût ajouté à chaque génération.
- **FR-007**: Les imports internes des paquets (versionnés par leurs auteurs) DOIVENT être résolus
  exactement parmi les paquets intégrés.
- **FR-008**: La liste des paquets intégrés DOIT être définie dans un fichier versionné du dépôt
  indiquant pour chaque paquet son nom, sa version, une empreinte d'intégrité, sa licence et s'il
  est mis à disposition des templates ou seulement dépendance.
- **FR-009**: La construction de l'application DOIT vérifier l'empreinte de chaque paquet et
  échouer si l'une ne correspond pas.
- **FR-010**: Les vérifications automatiques du projet DOIVENT prouver que chaque paquet intégré
  s'importe et produit un document avec la version du moteur utilisée ; un paquet qui échoue ne
  peut pas figurer dans la liste.
- **FR-011**: Changer la version d'un paquet mis à disposition DOIT se faire en remplaçant sa
  version dans la liste ; la nouvelle version s'applique à tous les templates.
- **FR-012**: Un diagnostic d'erreur survenant dans le code d'un paquet DOIT indiquer le paquet,
  le fichier et la ligne.
- **FR-013**: Un paquet NE DOIT pouvoir lire que ses propres fichiers ; un template peut lui
  transmettre explicitement une image, des données ou un chemin vers ses propres fichiers.
- **FR-014**: Le service DOIT exposer une route listant les paquets mis à disposition (nom, ligne
  d'import, version installée, description, licence), décrite dans le document OpenAPI.
- **FR-015**: Les PDF produits avec des paquets DOIVENT rester déterministes.
- **FR-016**: Les générations utilisant des paquets DOIVENT rester soumises aux bornes existantes
  (taille de requête, durée, concurrence).
- **FR-017**: Les licences des paquets intégrés DOIVENT être conservées dans l'application et
  listées dans la documentation.
- **FR-018**: La documentation des auteurs DOIT présenter les paquets disponibles (ce que fait
  chacun, exemple d'import) et les règles (import par le nom seul, écrit tel quel, accès aux
  fichiers).
- **FR-019**: Le dépôt DOIT fournir au moins un template d'exemple utilisant des paquets du
  service (au minimum un QR code et un montant formaté), servant de référence et de test.

### Key Entities *(include if feature involves data)*

- **Paquet intégré** : paquet Typst figé dans l'application ; identifié par son nom et sa
  version ; porte une description, une licence, une empreinte d'intégrité et un rôle (mis à
  disposition des templates, ou dépendance seulement).
- **Liste des paquets** : fichier versionné du dépôt qui définit exactement les paquets intégrés ;
  seule source de vérité.
- **Import de template** : `@preview/nom` écrit dans un fichier du template ; résolu vers la
  version installée du paquet mis à disposition, ou refusé.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100 % des paquets de la liste s'importent et produisent un document lors des
  vérifications automatiques.
- **SC-002**: 100 % des générations utilisant des paquets aboutissent sans aucune tentative
  d'accès réseau.
- **SC-003**: 100 % des imports incorrects (paquet inconnu, version écrite) produisent un message
  nommant l'import en cause.
- **SC-004**: Un auteur ajoute un QR code ou un montant formaté à un template en moins de
  5 minutes en suivant la documentation.
- **SC-005**: Le temps de génération du template de démonstration existant (sans paquet) ne se
  dégrade pas de plus de 5 %.
- **SC-006**: Le template d'exemple utilisant des paquets se génère en moins d'une seconde dans
  95 % des cas.
- **SC-007**: Une archive de paquet altérée est détectée à 100 % lors de la construction.

## Assumptions

- **Liste validée (2026-10-10)** — 21 paquets, tous dans leur dernière version publiée à cette
  date (`research/carte-paquets.md`) : `tiaoma`, `zebra`, `qrypst`, `sepay`,
  `zero`, `oxifmt`, `frogst`, `ibanator`, `datify`, `linguify`, `tabut`, `tablem`, `cetz`,
  `cetz-plot`, `lilaq`, `primaviz`, `showybox`, `framefit`, `modern-mailmerge`, `payqr-swiss`.
  Avec leurs 8 dépendances (`datify-core`, `elembic`, `komet` ×2, `rustycure`, `suiji`, `tiptoe`,
  `zero` 0.6.1) : 28 paquets, environ 6 Mo. `codetastic`, validé à l'origine, a été retiré à
  l'implémentation : incompatible avec Typst 0.15 (tiaoma couvre les mêmes codes-barres). Si un paquet échoue aux vérifications avec le moteur
  actuel, il est retiré de la liste et signalé (pas de correctif local du paquet).
- `cmarker` est exclu (il peut exécuter du code contenu dans le Markdown). Les paquets P2 pourront
  être ajoutés plus tard par simple évolution de la liste ; les P3 ne sont pas intégrés.
- Seul le namespace `@preview` (Typst Universe) est servi.
- L'import par le nom seul est une règle propre à inkpdf : un tel template ne se compile pas
  tel quel avec l'outil Typst standard (qui exige une version). Les paquets intégrés gardent
  leurs imports versionnés d'origine.
- Les paquets sont téléchargés uniquement au moment de la construction de l'application ;
  l'accès réseau pendant la construction est acceptable.
- La taille de l'application n'est pas un critère ; la rapidité de génération l'est.
- Conformité à la constitution : les paquets sont des ressources embarquées par le binaire
  (principe II), rien n'est téléchargé au fonctionnement (principes II et IV), et la liste ne
  change qu'avec une nouvelle version de l'application, les templates restant modifiables à
  chaud (principe V). Aucun amendement n'est nécessaire.
- Hors périmètre : dossier de paquets partagé dans le volume, paquets embarqués dans un template,
  paquet de helpers maison, paquets P3, séparation aperçu / document final, PDF/A, Factur-X.
