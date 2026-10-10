# Feature Specification: Paquets Typst embarqués dans les templates

**Feature Branch**: `002-typst-packages`

**Created**: 2026-10-10

**Status**: Draft

**Input**: User description: « Permettre aux templates d'utiliser des paquets Typst
(ex. `#import "@preview/cetz:0.3.4"`) sans aucun accès réseau, en les fournissant avec le
template. […] Un paquet ne peut lire que ses propres fichiers […] sauf via des imports de paquets
eux-mêmes embarqués, ou des images. […] Le moment où l'erreur est détectée reste à décider. […]
Hors périmètre : dossier de paquets partagé entre plusieurs templates, téléchargement
automatique, tout accès réseau au rendu. Contraintes inchangées : PDF déterministe, bac à sable
sans réseau, service sans état, contrat OpenAPI verrouillé. Autre chose : il faut aussi réfléchir
au paquet maison des helpers. »

## User Scenarios & Testing *(mandatory)*

### Acteurs

- **Auteur de template** : conçoit un template et le dépose dans le volume ; c'est lui qui
  choisit et fournit les paquets.
- **Application appelante** : demande des générations ; elle ne voit les paquets qu'à travers
  le détail d'un template et la qualité des documents produits.
- **Opérateur** : déploie le service et lit ses logs.

### User Story 1 - Utiliser un paquet de l'écosystème Typst dans un template (Priority: P1)

Un auteur veut tracer un graphique (ou un QR code, un tableau avancé…) dans son document. Il
récupère le paquet correspondant depuis Typst Universe, le place dans le dossier de paquets de
son template en respectant l'arborescence standard (namespace / nom / version), puis l'importe
depuis son fichier Typst exactement comme il le ferait en local. Les générations produisent le
document attendu, sans que le service n'accède jamais au réseau.

**Why this priority**: c'est le besoin qui motive la fonctionnalité ; sans lui, rien d'autre n'a
de valeur.

**Independent Test**: déposer un template qui embarque un paquet et l'importe, demander une
génération et vérifier que le PDF contient le rendu produit par le paquet, réseau coupé.

**Acceptance Scenarios**:

1. **Given** un template qui embarque le paquet `@preview/exemple:1.2.0` et l'importe avec
   cette version exacte, **When** l'appelant demande une génération valide, **Then** le PDF est
   produit et contient le rendu du paquet.
2. **Given** le même template, **When** deux générations identiques sont demandées, **Then** les
   deux PDF sont identiques octet pour octet.
3. **Given** un paquet embarqué qui importe lui-même un autre paquet embarqué dans le même
   template, **When** une génération est demandée, **Then** la dépendance est résolue et le PDF
   est produit.
4. **Given** un template qui importe `@preview/exemple:1.2.0` alors que seule la version `1.1.0`
   est embarquée, **When** une génération est demandée, **Then** aucune autre version n'est
   substituée et l'échec est signalé avec un diagnostic nommant `@preview/exemple:1.2.0`.

---

### User Story 2 - Comprendre immédiatement un problème de paquet (Priority: P2)

Un auteur oublie un paquet, se trompe de version ou dépose un paquet dont le manifeste est
illisible. Il doit comprendre l'erreur sans lire le code du service : le message nomme le paquet
et la version attendus et, pour un manifeste invalide, le fichier en cause.

**Why this priority**: les paquets ajoutent une nouvelle famille d'erreurs ; sans diagnostic
clair, la fonctionnalité coûte plus qu'elle n'apporte.

**Independent Test**: déposer successivement un template avec un paquet manquant, une version
absente et un manifeste invalide, et vérifier pour chacun le message obtenu.

**Acceptance Scenarios**:

1. **Given** un template qui importe un paquet non embarqué, **When** l'erreur est détectée
   (voir FR-009), **Then** le diagnostic nomme le paquet et la version demandés et précise
   qu'aucun téléchargement n'est possible.
2. **Given** un paquet embarqué dont le manifeste est absent, illisible ou incohérent avec son
   emplacement (nom ou version différents de ceux du dossier), **When** le template est chargé,
   **Then** le template est marqué invalide et le diagnostic nomme le fichier en cause.
3. **Given** un template marqué invalide à cause d'un paquet, **When** l'auteur corrige le
   paquet dans le volume, **Then** le template redevient disponible sans redémarrage.

---

### User Story 3 - Savoir quels paquets un template utilise (Priority: P3)

Une application appelante ou un opérateur consulte le détail d'un template et voit la liste des
paquets qu'il embarque (namespace, nom, version), par exemple pour vérifier la provenance des
paquets ou repérer une version obsolète.

**Why this priority**: utile pour l'audit et le support, mais la génération fonctionne sans.

**Independent Test**: consulter le détail d'un template qui embarque deux paquets et vérifier
qu'ils sont listés ; consulter un template sans paquet et vérifier que la liste est vide.

**Acceptance Scenarios**:

1. **Given** un template qui embarque deux paquets, **When** l'appelant consulte son détail,
   **Then** les deux paquets y figurent avec leur namespace, leur nom et leur version.
2. **Given** un template sans paquet, **When** l'appelant consulte son détail, **Then** la liste
   des paquets est présente et vide.

---

### User Story 4 - Paquet maison de helpers (Priority: P3)

Les auteurs de templates réécrivent souvent les mêmes fonctions utilitaires d'un template à
l'autre. Un paquet de helpers maison leur évite cette duplication.
[NEEDS CLARIFICATION: qu'est-ce que le « paquet maison des helpers » ? (a) un paquet fourni par
le service lui-même, embarqué dans l'image et importable par tous les templates, avec un
contenu défini par inkpdf ; (b) un paquet écrit par l'équipe, embarqué template par template
comme n'importe quel autre paquet (rien de spécifique à construire) ; (c) hors de cette
spécification, à traiter dans une feature dédiée]

**Why this priority**: confort d'écriture ; dépend de la réponse à la clarification.

**Independent Test**: à définir selon la clarification.

---

### Edge Cases

- Un template importe un paquet sans préciser de version : l'import est refusé avec un
  diagnostic, aucune version « la plus récente » n'est choisie.
- Deux paquets embarqués dépendent de deux versions différentes d'un même paquet : les deux
  versions peuvent coexister dans le template et chacune est résolue exactement.
- Un paquet tente de lire un fichier hors de son propre dossier (chemin absolu, `../`, fichier
  du template, fichier d'un autre paquet) : l'accès est refusé.
- Un paquet tente d'accéder au réseau ou à un paquet d'un autre template : refusé.
- Le dossier de paquets contient un lien symbolique sortant du dossier du template : il est
  ignoré ou fait échouer le chargement, comme pour les autres fichiers du template.
- Les paquets font dépasser la taille maximale d'un template : le template est marqué invalide,
  comme pour tout dépassement de taille.
- Un paquet est ajouté, modifié ou supprimé pendant qu'une génération est en cours : la
  génération en cours utilise l'état précédent, les suivantes le nouvel état.
- Le dossier de paquets contient des fichiers hors de l'arborescence attendue (README à la
  racine, dossier sans version) : ils n'empêchent pas le chargement et ne sont pas exposés comme
  paquets.
- Un paquet déclare dans son manifeste une version minimale du moteur Typst supérieure à celle
  du service : le diagnostic l'indique explicitement.
- Un paquet contient lui-même des polices : elles ne sont pas chargées (seules les polices du
  dossier de polices du template et celles du service le sont), ce qui est documenté.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Un template DOIT pouvoir embarquer des paquets Typst dans un dossier dédié de son
  propre dossier, rangés selon l'arborescence standard `namespace/nom/version`, chaque version
  contenant son manifeste.
- **FR-002**: Un import de la forme `@namespace/nom:version` DOIT être résolu uniquement parmi
  les paquets embarqués par le template, avec une correspondance exacte du namespace, du nom et
  de la version. Tout namespace est accepté tant que le paquet est embarqué.
- **FR-003**: Le service NE DOIT jamais télécharger de paquet, consulter de registre, ni
  utiliser de cache de paquets partagé entre templates ou présent sur la machine.
- **FR-004**: Les fichiers d'un paquet DOIVENT être lus depuis l'instantané du template déjà
  chargé, au même titre que les autres fichiers du template ; aucune lecture directe du disque
  n'a lieu pendant une génération.
- **FR-005**: Un paquet NE DOIT pouvoir lire que ses propres fichiers. L'accès aux fichiers du
  template, à ceux d'un autre paquet, ou à tout fichier hors du dossier du template est refusé.
  [NEEDS CLARIFICATION: exception « ou des images » — un paquet doit-il pouvoir afficher une
  image du template ? (a) oui, seulement si le template lui transmet l'image elle-même (contenu
  passé en argument, aucune lecture par le paquet) ; (b) oui, le paquet peut lire en lecture
  seule les images du dossier du template ; (c) non, aucune exception]
- **FR-006**: Les dépendances d'un paquet vers d'autres paquets DOIVENT être résolues selon
  FR-002, parmi les paquets embarqués du même template.
- **FR-007**: Le chargement d'un template DOIT valider chaque paquet embarqué : manifeste
  présent et lisible, nom et version du manifeste identiques à ceux de son emplacement, point
  d'entrée existant. Un paquet invalide rend le template invalide, avec un diagnostic nommant le
  paquet et le fichier en cause.
- **FR-008**: Un import vers un paquet ou une version non embarqués DOIT produire un diagnostic
  qui nomme le namespace, le nom et la version demandés, et indique que les paquets doivent être
  fournis avec le template.
- **FR-009**: Les imports de paquets non embarqués DOIVENT être détectés
  [NEEDS CLARIFICATION: à quel moment ? (a) uniquement au rendu : la génération échoue avec le
  diagnostic de FR-008, le template reste disponible ; (b) au chargement, d'après une liste de
  dépendances que le template déclare dans son manifeste : le template est marqué invalide si un
  paquet déclaré manque ; (c) au chargement, par analyse des imports présents dans les fichiers
  Typst, en plus du contrôle au rendu]
- **FR-010**: La taille des paquets DOIT être comptée dans la taille du template et soumise à la
  même limite.
- **FR-011**: L'ajout, la modification ou la suppression d'un paquet DOIT être pris en compte par
  le rechargement à chaud, dans les mêmes délais que les autres fichiers du template.
- **FR-012**: Le détail d'un template DOIT exposer la liste des paquets embarqués (namespace,
  nom, version), éventuellement vide. Le document OpenAPI DOIT décrire ce champ.
- **FR-013**: Les PDF produits avec des paquets DOIVENT rester déterministes : mêmes template,
  paquets et entrée → même document.
- **FR-014**: La génération avec paquets DOIT rester soumise aux bornes existantes (durée de
  génération, concurrence) ; un paquet coûteux ne peut pas bloquer le service.
- **FR-015**: La documentation des auteurs DOIT expliquer comment récupérer un paquet depuis
  Typst Universe, où le placer, comment l'importer, et lister les limites (pas de réseau, version
  exacte, pas de polices embarquées par les paquets, accès aux fichiers).
- **FR-016**: Le dépôt DOIT fournir au moins un template d'exemple utilisant un paquet embarqué,
  servant de référence de documentation et de test.

### Key Entities *(include if feature involves data)*

- **Paquet embarqué** : copie d'un paquet Typst rangée dans le template ; identifiée par son
  namespace, son nom et sa version ; contient un manifeste, un point d'entrée et ses fichiers.
  Appartient à un seul template.
- **Manifeste de paquet** : fichier décrivant le paquet (nom, version, point d'entrée, version
  minimale du moteur…), dont la cohérence avec l'emplacement est vérifiée au chargement.
- **Référence de paquet** : triplet `namespace/nom:version` utilisé par un import ; résolu à
  l'identique ou refusé.
- **Template** (existant) : gagne la liste de ses paquets embarqués dans son détail.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Un auteur peut rendre utilisable un paquet de Typst Universe dans son template en
  moins de 5 minutes en suivant uniquement la documentation.
- **SC-002**: 100 % des générations utilisant des paquets aboutissent sans aucune tentative
  d'accès réseau (vérifiable en coupant le réseau du service).
- **SC-003**: 100 % des erreurs de paquet (manquant, version absente, manifeste invalide)
  produisent un message qui nomme le paquet et la version en cause.
- **SC-004**: Le temps de génération du template de démonstration existant, qui n'utilise aucun
  paquet, ne se dégrade pas de plus de 5 %.
- **SC-005**: Un template d'exemple utilisant un paquet de graphique se génère en moins d'une
  seconde dans 95 % des cas.
- **SC-006**: Une modification d'un paquet dans le volume est prise en compte en moins de
  5 secondes, sans redémarrage.
- **SC-007**: Aucune tentative de lecture d'un paquet hors de son propre dossier n'aboutit
  (couvert par des tests dédiés).

## Assumptions

- Le dossier dédié aux paquets s'appelle `packages/` à la racine du template ; son contenu suit
  l'arborescence `packages/<namespace>/<nom>/<version>/`, identique à celle d'un dossier de
  paquets local de Typst, ce qui permet de copier un paquet tel quel.
- Les paquets sont fournis par l'auteur du template, qui est responsable de leur provenance et de
  leur licence ; le service ne vérifie ni signature ni empreinte.
- La version du moteur Typst est celle du service ; un paquet incompatible avec cette version
  échoue avec un diagnostic, sans mécanisme de repli.
- La validation des paquets au chargement porte sur leur structure et leur manifeste, pas sur la
  compilation de leur code.
- Contraintes héritées de la V1 et inchangées : PDF déterministe, bac à sable sans réseau,
  service sans état, contrat OpenAPI verrouillé, aucune police système.
- L'ajout de paquets embarqués respecte la constitution : tout reste résolu à l'intérieur du
  dossier du template (principe II) et aucun accès réseau n'est introduit (principe IV).
- Hors périmètre : dossier de paquets partagé entre plusieurs templates (à évaluer plus tard),
  téléchargement automatique ou mise à jour des paquets, tout accès réseau au rendu, outil de
  gestion des paquets (installation, verrouillage des versions).
