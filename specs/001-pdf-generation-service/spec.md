# Feature Specification: Service de génération de PDF à partir de templates (V1)

**Feature Branch**: `001-pdf-generation-service`

**Created**: 2026-10-09

**Status**: Implemented — 2026-10-09, PR #1 (57/57 tâches)

**Input**: User description: commande lancée sans argument ; description reprise du périmètre V1
fourni lors de la ratification de la constitution : « Microservice de génération de PDF basé sur
des templates Typst lus à chaud depuis un volume monté. Lister les templates disponibles,
récupérer le détail et le schéma d'un template, générer un PDF à partir d'un template et d'un
corps JSON validé contre le schéma. Le JSON porte deux dimensions : données métier et paramètres
de design. API auto-descriptive (OpenAPI), schémas en JSON Schema. »

## User Scenarios & Testing *(mandatory)*

### Acteurs

- **Application appelante** : service interne du VPC qui a besoin de produire un document PDF.
  Le service ignore tout de la nature du document : seul le template lui donne un sens.
- **Auteur de template** : personne qui conçoit un template et le dépose dans le volume.
- **Opérateur** : personne qui déploie et supervise le service.

### User Story 1 - Générer un PDF à partir d'un template et de données (Priority: P1)

Une application appelante envoie l'identifiant d'un template et un document JSON contenant les
données métier et, optionnellement, des paramètres de design
(ex. couleur principale). Le service vérifie que le JSON respecte le schéma du template puis
renvoie directement le PDF produit.

**Why this priority**: c'est la raison d'être du service ; sans elle, rien d'autre n'a de valeur.

**Independent Test**: déposer un template de démonstration dans le volume, envoyer un JSON
valide et vérifier qu'un PDF lisible, contenant les valeurs envoyées, est renvoyé.

**Acceptance Scenarios**:

1. **Given** un template valide présent dans le volume, **When** l'appelant demande
   une génération avec un JSON conforme au schéma, **Then** le service renvoie un document PDF
   dont le contenu reflète les données envoyées.
2. **Given** le même template, **When** l'appelant omet les paramètres de design, **Then** le PDF
   est généré avec les valeurs de design par défaut déclarées par le template.
3. **Given** le même template, **When** l'appelant envoie un JSON auquel il manque un champ
   métier obligatoire, **Then** la génération est refusée avant toute production de document,
   avec une erreur indiquant précisément le champ fautif.
4. **Given** aucun template portant l'identifiant demandé, **When** l'appelant demande une
   génération, **Then** le service répond que le template est introuvable.

---

### User Story 2 - Personnaliser l'apparence par appelant sans dupliquer le template (Priority: P2)

Une application appelante doit produire des variantes visuelles d'un même document (par exemple
pour plusieurs de ses propres clients). Elle envoie les mêmes types de données métier mais des
paramètres de design différents (couleur, position d'un élément, affichage ou non de certains
blocs), tels que le template les déclare. Un seul template suffit.

**Why this priority**: c'est le différenciateur du service (double dimension du JSON) ; il
s'appuie sur la génération (P1).

**Independent Test**: générer deux PDF avec le même template et les mêmes données métier mais
des paramètres de design différents, et constater que seule l'apparence diffère.

**Acceptance Scenarios**:

1. **Given** un template déclarant un paramètre de design « couleur principale », **When**
   l'appelant génère deux documents avec deux couleurs différentes, **Then** les deux PDF ont
   le même contenu métier et des couleurs différentes.
2. **Given** un template déclarant un paramètre booléen d'affichage d'un bloc, **When**
   l'appelant le positionne à faux, **Then** le bloc n'apparaît pas dans le PDF.
3. **Given** un template, **When** l'appelant envoie un paramètre de design hors des valeurs
   acceptées (ex. position du logo inconnue), **Then** la génération est refusée avec une erreur
   désignant ce paramètre.

---

### User Story 3 - Découvrir les templates et leur contrat (Priority: P3)

Un développeur d'une application appelante consulte la liste des templates disponibles, puis le
détail d'un template : sa description et le schéma qui décrit les données métier attendues et
les paramètres de design acceptés. Il consulte aussi la description complète de l'interface du
service, lisible par machine, pour générer un client ou tester des appels.

**Why this priority**: indispensable pour l'intégration autonome, mais une équipe peut générer
des PDF (P1) en connaissant déjà le template.

**Independent Test**: avec deux templates dans le volume, la liste renvoie les deux, et le
détail de chacun renvoie un schéma exploitable pour construire un JSON valide.

**Acceptance Scenarios**:

1. **Given** deux templates valides dans le volume, **When** l'appelant demande la liste,
   **Then** il obtient les deux identifiants avec leur nom et leur description.
2. **Given** un template valide, **When** l'appelant demande son détail, **Then** il obtient son
   schéma complet, distinguant les données métier des paramètres de design.
3. **Given** le service démarré, **When** l'appelant demande la description de l'interface,
   **Then** il obtient une description standard couvrant toutes les opérations exposées.

---

### User Story 4 - Publier un template sans redéployer le service (Priority: P4)

Un auteur de template dépose un nouveau dossier de template (ou modifie un template existant)
dans le volume monté. Le template devient disponible, à jour, sans redémarrage ni reconstruction
du service.

**Why this priority**: c'est ce qui découple le cycle de vie des documents de celui du service ;
sans elle le service reste utilisable (en redémarrant), mais l'exploitation est bien plus lourde.

**Independent Test**: service démarré, copier un nouveau dossier de template dans le volume,
puis vérifier qu'il apparaît dans la liste et qu'il est utilisable pour générer un PDF.

**Acceptance Scenarios**:

1. **Given** le service démarré, **When** un nouveau template valide est déposé dans le volume,
   **Then** il apparaît dans la liste et est utilisable sans redémarrage.
2. **Given** un template existant, **When** son fichier de mise en page est modifié dans le
   volume, **Then** les générations suivantes utilisent la nouvelle version.
3. **Given** un template supprimé du volume, **When** l'appelant le demande, **Then** le service
   répond qu'il est introuvable.
4. **Given** un template défectueux (schéma illisible ou fichier de mise en page absent) déposé
   dans le volume, **When** l'appelant liste les templates, **Then** les autres templates restent
   disponibles et le template défectueux est signalé comme invalide avec la raison.

---

### Edge Cases

- Le volume de templates est vide ou absent au démarrage : le service démarre, la liste est
  vide, et l'anomalie est signalée dans les journaux.
- Le JSON envoyé n'est pas un JSON syntaxiquement valide : erreur de l'appelant, sans génération.
- Le corps de la requête dépasse la taille maximale autorisée : rejet explicite.
- Le JSON est valide vis-à-vis du schéma mais la mise en page échoue (erreur dans le template) :
  erreur interne distincte d'une erreur de validation, avec un message exploitable par l'auteur
  du template.
- La génération dépasse la durée maximale autorisée : elle est interrompue et une erreur est
  renvoyée ; le service reste disponible pour les autres requêtes.
- Un identifiant de template contient des caractères de navigation dans l'arborescence
  (ex. `../`) : rejet, aucun fichier hors du volume n'est lu.
- Un template tente de lire un fichier hors de son dossier ou d'accéder au réseau : l'accès est
  refusé et la génération échoue avec une erreur.
- Une donnée métier contient du texte ressemblant à du code de mise en page : il est rendu
  littéralement comme texte, jamais interprété.
- Un template est en cours de copie dans le volume au moment d'une requête : le service ne
  produit pas de document incohérent (il renvoie une erreur ou utilise un état complet).
- Plusieurs générations simultanées sur le même template : elles aboutissent toutes de façon
  indépendante.

## Requirements *(mandatory)*

### Functional Requirements

**Templates**

- **FR-001**: Un template DOIT être un dossier autonome du volume, contenant un fichier de mise
  en page paramétrable, un schéma d'entrée et, optionnellement, des ressources (polices, images).
- **FR-002**: L'identifiant d'un template DOIT être le nom de son dossier dans le volume.
- **FR-003**: Le template DOIT pouvoir déclarer des métadonnées descriptives (nom lisible,
  description, version) exposées lors de la découverte.
- **FR-004**: Le schéma d'un template DOIT décrire séparément deux sections de l'entrée : les
  données métier et les paramètres de design.
- **FR-005**: Les paramètres de design DOIVENT pouvoir déclarer des valeurs par défaut,
  appliquées lorsque l'appelant ne les fournit pas.
- **FR-006**: Un template NE DOIT pouvoir utiliser que les ressources présentes dans son propre
  dossier ou fournies par défaut par le service ; aucune ressource distante n'est résolue.

**Découverte**

- **FR-007**: Le service DOIT détecter au fil de l'eau les templates ajoutés, modifiés ou
  supprimés dans le volume, sans redémarrage ni reconstruction.
- **FR-008**: Le service DOIT permettre de lister les templates disponibles avec, pour chacun,
  identifiant, nom, description et statut (valide / invalide).
- **FR-009**: Le service DOIT permettre d'obtenir le détail d'un template par identifiant,
  incluant ses métadonnées et son schéma complet au format JSON Schema.
- **FR-010**: Un template invalide DOIT être signalé comme tel avec la raison, sans empêcher le
  démarrage du service ni la disponibilité des autres templates ; il ne peut pas être utilisé
  pour générer.

**Génération**

- **FR-011**: Le service DOIT permettre de générer un PDF à partir d'un identifiant de template
  et d'un corps JSON, et renvoyer le PDF directement dans la réponse.
- **FR-012**: Le corps JSON DOIT être validé contre le schéma du template avant toute
  génération ; en cas d'échec, aucune génération n'est tentée.
- **FR-013**: Une erreur de validation DOIT lister chaque violation avec le chemin de la donnée
  fautive et la raison.
- **FR-014**: Les données reçues DOIVENT être traitées uniquement comme des valeurs ; le service
  NE DOIT accepter aucun code de mise en page ni chemin de fichier venant de l'appelant.
- **FR-015**: La génération NE DOIT avoir aucun accès réseau ni aucun accès aux fichiers hors du
  dossier du template et des ressources fournies par défaut par le service.
- **FR-016**: Le service DOIT imposer une taille maximale de requête et une durée maximale de
  génération, configurables par l'opérateur.
- **FR-017**: Deux générations avec les mêmes entrées et le même template DOIVENT produire des
  documents au contenu visuellement identique.

**Interface et erreurs**

- **FR-018**: Le service DOIT publier une description standard et lisible par machine
  (OpenAPI) de toutes ses opérations, cohérente avec son comportement réel.
- **FR-019**: Les erreurs DOIVENT suivre un format unique et distinguer : template introuvable,
  template invalide, entrée invalide (syntaxe ou schéma), requête trop volumineuse, échec de
  génération, dépassement de durée.
- **FR-020**: Le service NE DOIT exiger aucune authentification (usage interne en réseau privé).
- **FR-024**: Le service DOIT être agnostique du contenu : il ne connaît aucun type de document
  ni aucun champ métier. Toute la sémantique (structure de `data` et `design`, mise en page)
  est portée par les templates ; le service se limite à découvrir, valider et rendre.

**Exploitation**

- **FR-021**: Le service DOIT exposer un point de contrôle de santé utilisable par un
  orchestrateur.
- **FR-022**: Chaque génération DOIT être journalisée avec l'identifiant du template, la durée
  et l'issue (succès, erreur de validation, erreur de génération), sans journaliser le contenu
  des données métier.
- **FR-023**: L'emplacement du volume, le port d'écoute et les limites (FR-016) DOIVENT être
  configurables au déploiement, avec des valeurs par défaut documentées.

### Key Entities

- **Template** : unité publiée dans le volume. Attributs : identifiant (nom du dossier),
  métadonnées (nom, description, version), fichier de mise en page, schéma, ressources, statut
  (valide / invalide + raison).
- **Schéma de template** : contrat d'entrée en JSON Schema, composé de deux sections — données
  métier et paramètres de design (avec valeurs par défaut éventuelles).
- **Requête de génération** : identifiant de template + document JSON contenant une section
  données métier et une section paramètres de design (optionnelle).
- **Document généré** : PDF renvoyé à l'appelant ; non conservé par le service.
- **Erreur** : type d'erreur, message, et pour les erreurs de validation la liste des violations
  (chemin, raison).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Un document d'une page (ex. un titre et un tableau de 20 lignes) est renvoyé à l'appelant en
  moins de 200 ms dans 95 % des cas, sur un template déjà découvert.
- **SC-002**: Le service démarre et est prêt à répondre en moins de 2 secondes avec 50 templates
  dans le volume.
- **SC-003**: Un template déposé dans le volume est utilisable en moins de 5 secondes, sans
  aucune intervention sur le service.
- **SC-004**: 100 % des entrées non conformes au schéma sont rejetées avant génération, avec au
  moins le chemin de chaque champ fautif.
- **SC-005**: Un développeur découvrant le service peut, à partir de la seule découverte
  (liste, détail, description d'interface), produire son premier PDF valide en moins de
  15 minutes.
- **SC-006**: Un même template sert au moins 2 rendus visuellement distincts (couleur, blocs
  affichés) sans aucune copie du template.
- **SC-007**: Le service traite 20 générations simultanées sans erreur ni indisponibilité.
- **SC-008**: Aucune tentative d'accès réseau ou de lecture hors du dossier du template, issue
  d'un template ou d'une entrée, n'aboutit (vérifié par une batterie de cas malveillants).
- **SC-009**: L'image livrée pèse moins de 100 Mo, soit au moins 5 fois moins qu'une solution
  équivalente basée sur un navigateur headless.

## Assumptions

- Les exemples de la spec (titre, liste d'éléments, couleur) sont purement illustratifs ; le
  dépôt fournit un template de démonstration neutre servant aux tests et à la documentation.
- Le service est déployé dans un réseau privé non exposé ; l'absence d'authentification est
  acceptée (constitution, principe VII).
- Le corps JSON d'une génération comporte deux sections de premier niveau distinctes, l'une
  pour les données métier et l'autre pour les paramètres de design ; les noms exacts seront
  fixés lors de la conception.
- Les métadonnées d'un template sont déclarées dans un fichier de son dossier ; à défaut, le
  nom affiché est l'identifiant.
- Un template invalide reste visible dans la liste avec le statut « invalide » (plutôt
  qu'exclu), afin d'aider les auteurs à diagnostiquer.
- Les polices fournies par défaut par le service couvrent les alphabets latins ; d'autres
  polices sont apportées par les templates.
- Le volume est en lecture seule pour le service ; il n'y a ni upload ni modification de
  templates via l'interface.
- Les PDF générés ne sont ni stockés ni mis en cache : le service est sans état.
- Valeurs par défaut des limites : 5 Mo par requête, 30 secondes par génération (ajustables).
- Hors périmètre V1 : génération par lots, autres moteurs de rendu, formats de sortie autres
  que PDF, envoi asynchrone ou par webhook, conservation des documents.
