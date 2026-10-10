<!--
Sync Impact Report
==================
Version change: 1.0.1 → 1.0.2
Bump rationale: PATCH — la dimension « paramètres de design » est renommée « paramètres de mise
en page » (clé `layout` dans le corps, les schémas et `sys.inputs`) ; même concept, aucune règle
ajoutée, retirée ou modifiée (spec 002, US7).

Modifications 1.0.2 :
- II. Le template, unité auto-suffisante : « paramètres de design » → « paramètres de mise en
  page (`layout`) »
- III. Entrée JSON à double dimension : même renommage (quatre occurrences)

Templates dépendants : aucun changement requis.

Historique 1.0.1 :
Version change: 1.0.0 → 1.0.1
Bump rationale: PATCH — exemples métier (« facture », « clients finaux ») remplacés par des
formulations neutres ; aucune règle ajoutée, retirée ou modifiée.

Modifications 1.0.1 :
- III. Entrée JSON à double dimension : exemples neutralisés, « clients finaux » →
  « variantes visuelles »
- Workflow de développement et qualité : « template d'exemple (ex. facture) » →
  « template de démonstration neutre »

Historique 1.0.0 (ratification initiale, depuis le template) :

Principes (template → nouveau) :
- [PRINCIPLE_1_NAME] → I. Moteur Typst embarqué, sans navigateur
- [PRINCIPLE_2_NAME] → II. Le template, unité auto-suffisante
- [PRINCIPLE_3_NAME] → III. Entrée JSON à double dimension, validée par schéma
- [PRINCIPLE_4_NAME] → IV. Sécurité par construction
- [PRINCIPLE_5_NAME] → V. Templates à chaud, binaire figé
- Ajouté → VI. API REST auto-descriptive
- Ajouté → VII. Simplicité et périmètre maîtrisé

Sections ajoutées :
- Identité et contraintes techniques (remplace [SECTION_2_NAME])
- Workflow de développement et qualité (remplace [SECTION_3_NAME])
- Governance (renseignée)

Sections supprimées : aucune

Templates dépendants (lus au runtime, non modifiés par cette commande) :
- .specify/templates/plan-template.md : la section « Constitution Check » s'appuie sur les
  principes I à VII
- .specify/templates/spec-template.md : aucun changement requis
- .specify/templates/tasks-template.md : aucun changement requis

TODO différés :
- TODO(LICENSE) : choix entre MIT et Apache-2.0 (ou double licence MIT OR Apache-2.0) à trancher
  avant la première publication de la crate et de l'image.
-->

# inkpdf Constitution

## Core Principles

### I. Moteur Typst embarqué, sans navigateur

- La génération de PDF DOIT passer exclusivement par le moteur Typst, embarqué comme bibliothèque
  Rust dans le binaire `inkpdf`.
- Le service NE DOIT lancer aucun binaire externe ni sous-processus (pas de CLI `typst`, pas de
  navigateur headless, pas de Chromium, pas de rendu HTML/CSS).
- Toute dépendance qui introduirait un navigateur ou un processus fils est refusée.

**Rationale** : inkpdf existe comme alternative légère et rapide aux services basés sur un
navigateur (type Gotenberg). Un sous-processus ou un navigateur annulerait ce bénéfice en
latence, en mémoire et en taille d'image.

### II. Le template, unité auto-suffisante

- Le template est l'unité du service. Un template est un dossier contenant :
  1. un fichier Typst paramétrable décrivant le document ;
  2. un schéma (JSON Schema) déclarant les données métier et les paramètres de mise en page (`layout`) acceptés ;
  3. des ressources optionnelles (polices, images).
- Un template DOIT être auto-suffisant : toutes ses ressources sont résolues localement, à
  l'intérieur de son dossier (ou des ressources embarquées par le binaire, comme les polices par
  défaut).
- La résolution d'un template NE DOIT jamais nécessiter d'accès réseau (pas de paquets Typst
  téléchargés, pas d'URL distante).
- Un template NE PEUT PAS référencer de fichiers hors de son propre dossier.

**Rationale** : un dossier auto-suffisant est déployable par simple copie, reproductible, et
borne précisément ce que le moteur peut lire.

### III. Entrée JSON à double dimension, validée par schéma

- Le JSON d'entrée d'une génération porte deux dimensions distinctes et explicitement séparées :
  - les **données métier** : le contenu du document, dont la structure est définie par le
    template ;
  - les **paramètres de mise en page** (`layout`) : les réglages d'apparence acceptés par le
    template
    (ex. couleur, alignement, affichage ou non d'un bloc).
- Le schéma du template DOIT décrire les deux dimensions ; les paramètres de mise en page DEVRAIENT
  déclarer des valeurs par défaut afin qu'un appel sans réglage produise un document valide.
- Le JSON DOIT être validé contre le schéma du template avant toute compilation Typst. Une
  entrée invalide est rejetée avec une erreur structurée (client error) indiquant les chemins
  fautifs ; aucune compilation n'est tentée.
- Un même template DOIT pouvoir produire plusieurs variantes visuelles par simple variation des
  paramètres de mise en page, sans duplication du template.

**Rationale** : séparer contenu et apparence rend un template réutilisable ; valider en amont
transforme les erreurs de rendu opaques en erreurs de contrat lisibles par l'appelant.

### IV. Sécurité par construction (NON-NEGOTIABLE)

- L'appelant n'envoie que du JSON. L'API NE DOIT accepter aucun code Typst, aucun fragment de
  markup interprété, ni aucun chemin de fichier fourni par l'appelant.
- Les données JSON DOIVENT être injectées dans Typst comme données (ex. via `sys.inputs` ou une
  source virtuelle), jamais par concaténation ou interpolation de source Typst.
- Le moteur Typst s'exécute en bac à sable : aucun accès réseau, aucun accès au système de
  fichiers hors du dossier du template et des ressources embarquées, aucune variable
  d'environnement exposée.
- Les identifiants de template DOIVENT être validés (pas de traversée de chemin, pas de lien
  symbolique sortant du volume).
- Toute génération DOIT être bornée (taille de requête, durée de compilation) afin qu'un
  template ou une entrée pathologique ne puisse pas bloquer le service.

**Rationale** : l'absence d'authentification (cf. VII) n'est acceptable que si la surface
d'attaque est réduite au strict contrat JSON + schéma.

### V. Templates à chaud, binaire figé

- Le binaire Rust (moteur inclus) est figé dans l'image Docker `ghcr.io/ck-developer/inkpdf`.
  Ajouter ou modifier un template NE DOIT jamais nécessiter de recompiler ni de reconstruire
  l'image.
- Les templates sont lus depuis un volume monté et découverts dynamiquement au runtime : un
  template déposé dans le volume DOIT devenir disponible sans redémarrage.
- Un template invalide (schéma illisible, fichier Typst manquant) NE DOIT pas empêcher le
  service de démarrer ni masquer les autres templates ; il est signalé (logs, et exclu ou marqué
  comme invalide dans la découverte).
- Toute mise en cache éventuelle DOIT rester cohérente avec le contenu du volume (invalidation
  sur modification).

**Rationale** : séparer le cycle de vie du moteur de celui des templates permet aux équipes
de livrer des documents sans toucher au service.

### VI. API REST auto-descriptive

- L'interface publique est une API REST dont la ressource principale est le template, exposée
  en lecture seule. La V1 expose au minimum :
  - la liste des templates disponibles ;
  - le détail et le schéma d'un template par identifiant ;
  - la génération d'un PDF à partir d'un template et d'un corps JSON, le PDF étant renvoyé
    directement dans la réponse (`application/pdf`).
- L'API DOIT être décrite par un document OpenAPI servi par le service lui-même et maintenu
  synchrone avec le code (généré depuis le code ou vérifié par les tests).
- Les schémas de templates DOIVENT être exprimés en JSON Schema et exposés tels quels par l'API.
- Les erreurs DOIVENT suivre un format structuré unique et distinguer erreur de l'appelant
  (template inconnu, JSON invalide) et erreur interne (échec de compilation).

**Rationale** : un appelant doit pouvoir découvrir, comprendre et appeler un template sans
lire son code Typst ni la documentation du dépôt.

### VII. Simplicité et périmètre maîtrisé

- inkpdf est conçu pour un usage interne en VPC privé, non exposé publiquement. Il NE DOIT PAS
  embarquer de gestion d'utilisateurs, d'authentification ni de permissions.
- Le périmètre V1 est limité à : moteur Typst uniquement, gestion des templates via volume,
  génération unitaire (un template + un JSON), API REST de découverte + OpenAPI.
- Toute fonctionnalité hors de ce périmètre (autres moteurs, génération par lots, upload de
  templates via l'API, stockage des PDF, etc.) DOIT faire l'objet d'un amendement ou d'une
  spécification explicite justifiant sa complexité.
- Le service est sans état : aucune base de données ; le volume de templates est la seule
  source de vérité.

**Rationale** : un microservice étroit reste léger, auditable et facile à opérer ; YAGNI
s'applique tant qu'un besoin réel n'est pas démontré.

## Identité et contraintes techniques

- **Nom** : inkpdf — **Dépôt** : github.com/ck-developer/inkpdf — **Crate** : `inkpdf` —
  **Image** : `ghcr.io/ck-developer/inkpdf`.
- **Licence** : open source permissive. TODO(LICENSE) : MIT ou Apache-2.0 (ou double licence
  `MIT OR Apache-2.0`, convention de l'écosystème Rust) à trancher avant la première release.
- **Langage** : Rust stable. Le service est livré sous forme d'un binaire unique.
- **Distribution** : image Docker minimale contenant le binaire et ses ressources embarquées ;
  les templates sont fournis par un volume monté en lecture seule, dont le chemin est
  configurable.
- **Configuration** : par variables d'environnement (chemin du volume, port d'écoute, limites
  de taille et de durée), avec des valeurs par défaut documentées.
- **Performance** : la génération DOIT rester rapide et légère ; toute régression notable de
  latence ou d'empreinte mémoire sur les templates d'exemple DOIT être justifiée.
- **Observabilité** : logs structurés sur stdout incluant au minimum l'identifiant du template,
  la durée de génération et l'issue (succès, erreur de validation, erreur de compilation) ; un
  endpoint de santé DOIT être disponible pour l'orchestrateur.

## Workflow de développement et qualité

- Chaque fonctionnalité suit le cycle Spec Kit : spécification → plan → tâches →
  implémentation. Le « Constitution Check » du plan DOIT vérifier les principes I à VII.
- Le code DOIT passer `cargo fmt --check`, `cargo clippy` (sans avertissement) et
  `cargo test` avant fusion.
- Les tests DOIVENT couvrir au minimum :
  - la validation JSON contre schéma (entrées valides et invalides, sur les deux dimensions) ;
  - le contrat de l'API REST (codes, formats d'erreur, conformité au document OpenAPI) ;
  - la génération de bout en bout d'au moins un template d'exemple ;
  - les garanties de bac à sable (tentative d'accès réseau, de lecture hors dossier, de
    traversée de chemin dans l'identifiant).
- Le dépôt DEVRAIT fournir au moins un template de démonstration neutre, sans domaine métier,
  illustrant les deux dimensions du JSON d'entrée ; il sert de référence de documentation et
  de test.
- Toute modification de l'API publique ou du format d'un template DOIT être reflétée dans le
  document OpenAPI et la documentation dans le même changement.

## Governance

- Cette constitution prévaut sur toute autre pratique du projet. En cas de conflit, la
  constitution l'emporte jusqu'à son amendement.
- **Amendement** : toute modification passe par une pull request qui met à jour ce fichier,
  son Sync Impact Report et la version ; elle explique la motivation et, si nécessaire, le plan
  de migration des templates ou des appelants existants.
- **Versionnement** (SemVer) :
  - MAJOR : suppression ou redéfinition incompatible d'un principe ;
  - MINOR : ajout d'un principe ou d'une section, ou extension matérielle d'une règle ;
  - PATCH : clarification, reformulation ou correction sans effet sur les règles.
- **Conformité** : chaque revue de PR vérifie le respect des principes ; toute dérogation DOIT
  être justifiée explicitement (section « Complexity Tracking » du plan) et approuvée en revue.
- Les décisions différées (TODO) DOIVENT être résolues par un amendement PATCH ou MINOR dès
  qu'elles sont tranchées.

**Version**: 1.0.2 | **Ratified**: 2026-10-09 | **Last Amended**: 2026-10-10
