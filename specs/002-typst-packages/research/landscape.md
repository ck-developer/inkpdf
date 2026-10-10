# Paysage des moteurs et services de génération de documents et de PDF

> Recherche pour la feature `002-typst-packages` d'inkpdf. Relevé du 2026-10-10, sur sources
> primaires (docs officielles, dépôts GitHub, sources des crates Typst 0.15.1). « non trouvé » =
> absent des pages officielles consultées ; ce n'est pas une preuve d'absence. Les sections
> « Emprunter », « Éviter » et « Positionnement » sont une interprétation, appuyée sur les faits
> cités plus haut.

## 0. Tableau comparatif

| Outil | Type | Modèle de template | Réutilisation · versions | Bac à sable · réseau | Validation entrée | PDF/A · UA · Factur-X | API · OpenAPI |
|---|---|---|---|---|---|---|---|
| **inkpdf (V1)** | service Rust auto-hébergé | Typst + `sys.inputs` `{data, design}` | `#include` dans le dossier ; pas de paquets (objet de 002) | pas de réseau, pas de fichier hors du dossier, durée bornée | **JSON Schema 2020-12, avant compilation** | Typst sait A-1b…A-4f, UA-1, `pdf.attach` ; inkpdf compile avec `PdfOptions::default()`, aucun standard | REST, **OpenAPI 3.1 généré** |
| Gotenberg 8 | service Docker (Chromium, LibreOffice) | HTML/Office envoyé à chaque requête, pas de fusion | rien côté serveur | réseau sortant **permissif par défaut** (IP privées OK) | non | A-1b/2b/3b + UA (post-traitement), **route Factur-X** | multipart, OpenAPI non trouvé |
| Carbone v5 | service (on-prem + cloud) | DOCX/ODT/XLSX… + balises `{d.x}` | **`templateId` + `versionId` SHA-256**, `appendTemplate`, `styleSource` | `securityLevel` LibreOffice, limites de téléchargement | non (taille seule) | A-1…4, UA (LibreOffice), `:attachFile` | **OpenAPI V5** |
| DocRaptor | SaaS (Prince 15.1) | HTML/CSS brut | aucune ; `pipeline` fige le moteur | JS off par défaut, `no_network` | `strict=html` | A-1a…3b, UA-1 ; Factur-X non trouvé | SDK générés depuis OpenAPI |
| Prince 17 | CLI | HTML/XML + CSS | feuilles multiples, XInclude (off) | JS off, `--no-network`, `--no-local-files` | non | A-1a…3b, UA-1, **`--attach` « for Factur-X »**, `--pdf-xmp` | CLI |
| PDFMonkey | SaaS (Chrome 133) | HTML + Liquid, `payload` JSON | **snippets de compte + partials, brouillon/publié** | non documenté | non (objet requis) | non trouvé | REST, OpenAPI non trouvé |
| APITemplate.io | SaaS | HTML + Jinja2 | pas d'historique ; `rendering_engine_version` | non documenté | non | **`einvoice` Factur-X/ZUGFeRD (expérimental)** | **OpenAPI 3.0** |
| Docmosis Tornado/Cloud | service Java | DOCX/ODT + `<<champs>>` | `<<ref:/common/x.docx>>`, coordinator, environnements ; pas de versions | aucune sortie requise, URL d'images en liste blanche | **`strictParams`, `getTemplateStructure`, `getSampleData`** | A-1b/2b/3b, UA, **`pdfEInvoiceSpec`** | **OpenAPI** |
| Docxtemplater 3.71 | lib JS | DOCX + `{x}`, `{#x}` | module payant Subtemplate `{:include}` | pas d'I/O ; parser `csp` | non (`nullGetter`) | pas de PDF | lib |
| pdfme 6.2 | lib JS | JSON (`basePdf` + `schemas`) | plugins (pdf/ui/propPanel), `@pdfme/jsx` | expressions en liste blanche | **zod + CLI `validate --json`** | non trouvé ; `manipulator` (fusion) | lib + CLI |
| @react-pdf/renderer 4.9 | lib JS | composants React + props | composants (npm) | images par URL, emojis en ligne | non | **PDF/A-1/2/3 b** (4.9.0) | lib |
| React Email 6.11 | lib JS (e-mail) | composants React + `PreviewProps` | composants, dossier `_components`, npm | non trouvé | TypeScript | sans objet | CLI `dev`/`export`, `render()` |
| jsreport 4.14 | serveur Node | Handlebars/JsRender + helpers JS | **assets, components, child templates, version control (commit/diff/revert)** | sandbox JS, docker-workers | JSON Schema non trouvé | A-1B bêta, UA, `pdfSign`, pièces jointes | REST + OData, OpenAPI non trouvé |
| WeasyPrint 70 | lib/CLI Python | HTML + CSS | sans objet | `URLFetcher` à configurer ; `file://` ouvert par défaut | non | **A/UA-1/UA-2/X étendus, CLI Factur-X**, validité « not guaranteed » | lib |
| Paged.js 0.4.3 | polyfill navigateur | HTML + CSS Paged Media | handlers JS | celle du navigateur | non | via Chromium | lib + CLI (Puppeteer) |
| Puppeteer 25 | navigateur headless | impression de page | sans objet | sandbox Chromium, interception réseau | non | `tagged` true par défaut | lib |
| Playwright 1.64 | navigateur headless | impression de page | sans objet | `page.route` | non | `tagged` false par défaut | lib |
| LaTeX (TeX Live 2026) | CLI | macros TeX | **paquets CTAN versionnés, installation globale non épinglée, rollback daté du noyau** | `\write18` restreint, lecture libre | non | `\DocumentMetadata{pdfstandard}`, paquet `zugferd` | services : latex-online, Overleaf CLSI, texlive.net |
| Paquets Typst / Universe | écosystème | Typst | **triplet `@ns/nom:version` exact, versions immuables, `typst init` copie le scaffold** | langage pur ; téléchargement `@preview` à la demande | non | — | — |
| Oicana | lib Typst multi-langage | Typst + entrées déclarées `[tool.oicana]` | template packé en zip | lib in-process | **JSON Schema avant Typst** | non trouvé | exemple Axum + OpenAPI |
| papermake | registre + serveur Typst | Typst + `sys.inputs.data` | **registre adressé par contenu, tags mutables vs digests** | non documenté | schéma optionnel, validation non confirmée | non trouvé | REST |
| navikt/pdfgenrs | service Typst | Typst + JSON | `resources/`, `fonts/` globaux | racine Typst bornée | non | **PDF/A-2a + UA-1** | REST, pas de rechargement à chaud |
| simplebash document-server | service Typst | Typst + `sys.inputs` | **`templates/lib/` paquets partagés** | non documenté | **JSON Schema par template** | non trouvé | OpenAPI + Swagger |

Sources du tableau : voir les sections détaillées ci-dessous (chaque ligne y est sourcée).

## 1. Services HTTP

### 1.1 Gotenberg (8.37.0)

- **Template** : aucun moteur de template ni fusion de données. On envoie `index.html` (obligatoire) et ses assets en multipart ; `header.html` et `footer.html` acceptent seulement les classes `pageNumber`, `totalPages`, `date`, `title`, `url` ([HTML→PDF](https://gotenberg.dev/docs/convert-with-chromium/convert-html-to-pdf)). Moteurs : Chromium, LibreOffice, Markdown ([intro](https://gotenberg.dev/docs/getting-started/introduction)).
- **Réutilisation · versions** : service sans état, sans template stocké ; les assets sont renvoyés à chaque requête, dans un dossier plat ([HTML→PDF](https://gotenberg.dev/docs/convert-with-chromium/convert-html-to-pdf)). La réutilisation se fait entièrement côté client. Version 8.37.0 ([releases](https://github.com/gotenberg/gotenberg/releases)).
- **Design** : options de page Chromium uniquement.
- **Sécurité** : filtrage sortant permissif par défaut. Des drapeaux `*_DENY_PRIVATE_IPS` existent (défaut `false`), ainsi qu'un proxy anti-DNS-rebinding ; `file://` est rejeté ; depuis 8.34.0, le contenu lié des documents LibreOffice est supprimé ([outbound filtering](https://gotenberg.dev/docs/outbound-url-filtering)). `--chromium-disable-javascript` vaut `false` par défaut ; concurrence Chromium max 6 ; `API_TIMEOUT` 30 s ([configuration](https://gotenberg.dev/docs/configuration)).
- **Validation** : aucun schéma ; seulement `failOnResourceLoadingFailed`.
- **Empreinte** : variantes d'image `8-chromium` (~30 % plus petite) et `8-libreoffice` (~40 % plus petite) ; Kubernetes ≥ 512Mi, Cloud Run ≥ 1Gi ([installation](https://gotenberg.dev/docs/getting-started/installation)).
- **API** : multipart, `/health`, `/version` ; OpenAPI non trouvé ([routes](https://gotenberg.dev/docs/getting-started/routes)).
- **Avancé** :
  - PDF/A-1b/2b/3b et PDF/UA via LibreOffice ;
  - opérations : merge, split, embed, flatten, watermark, encrypt, metadata ;
  - **`/forms/pdfengines/factur-x`** ;
  - captures PNG ;
  - signatures : non trouvé.
  - Source : [routes](https://gotenberg.dev/docs/getting-started/routes).

### 1.2 Carbone (v5, changelog 5.15.4 du 6 oct. 2026)

- **Template** : document bureautique (DOCX, ODT, XLSX, PPTX) ou HTML/MD, avec des balises `{d.x}`, `{c.x}`, `{t()}`, `{o.}`, des formatters, des boucles et des conditions ([getting started](https://carbone.io/documentation/design/overview/getting-started.html)). Images, couleurs et signatures sont réservées à l'édition Enterprise ([features](https://carbone.io/documentation/design/overview/template-feature.html)).
- **Réutilisation · versions** : le modèle de versions le plus complet du panel.
  - `templateId` stable + `versionId` SHA-256 par version. Le rendu par `templateId` prend la version au `deployedAt` le plus récent ; on peut aussi viser un `versionId` ([HTTP API](https://carbone.io/documentation/developer/http-api/introduction.html)).
  - Métadonnées `category`, `tags`, `sample` (données de test) ([manage templates](https://carbone.io/documentation/developer/http-api/manage-templates.html)).
  - `:appendTemplate(id)` ajoute le PDF d'un autre template (bêta, on-prem) ([file operations](https://carbone.io/documentation/design/advanced-features/file-operations.html)).
  - `{o.styleSource=…}` applique le style d'un autre template ([options](https://carbone.io/documentation/design/overview/in-template-options.html)).
  - `{o.preReleaseFeatureIn=…}` fige le comportement du moteur ([version lifecycle](https://carbone.io/documentation/design/overview/version-lifecycle.html)).
- **Design** : `:color(scope, type)` pour des couleurs dynamiques (Enterprise) ([colors](https://carbone.io/documentation/design/advanced-features/colors.html)) et `styleSource`.
- **Sécurité** : en on-prem, l'authentification est désactivée par défaut. `securityLevel` bloque les scripts et macros LibreOffice. Limites : téléchargements 6 s / 20 fichiers / 10 Mo, données 60 Mo, `maxGenerationTime` 60 s ([configuration](https://carbone.io/documentation/developer/on-premise-installation/configuration.html)).
- **Validation** : taille seulement (413) ([generate](https://carbone.io/documentation/developer/http-api/generate-reports.html)).
- **Perf** : profil `slim` 1 vCPU / 1 Go, `full` 2 vCPU / 3 Go ; débit DOCX→PDF 2 800/min sur 1 CPU (benchmark éditeur, M4 Max) ([requirements](https://carbone.io/documentation/developer/on-premise-installation/requirements.html)).
- **API** : **spec OpenAPI V5**, Postman ([HTTP API](https://carbone.io/documentation/developer/http-api/introduction.html)).
- **Avancé** :
  - PDF/A-1…4 et PDF/UA, via LibreOffice uniquement ; sortie PNG ; `batchOutput` ([generate](https://carbone.io/documentation/developer/http-api/generate-reports.html)) ;
  - `:attachFile` avec AFRelationship, pour Factur-X ([file operations](https://carbone.io/documentation/design/advanced-features/file-operations.html)) ;
  - `:sign` place des **champs** destinés à Docusign ou Yousign, sans signature cryptographique ([signatures](https://carbone.io/documentation/design/advanced-features/signatures.html)).

### 1.3 DocRaptor (pipeline 10.1 = Prince 15.1)

- **Template** : HTML/CSS via `document_content` ou `document_url` ; pas de fusion ([API](https://docraptor.com/documentation/api)).
- **Réutilisation · versions** : rien côté serveur. Le paramètre `pipeline` fige la version de Prince et du JS ([paramètres](https://docraptor.com/documentation/api/parameters)).
- **Sécurité** : JS désactivé par défaut ; `no_network`, `http_timeout` 1–60 s ([paramètres](https://docraptor.com/documentation/api/parameters)). Données chiffrées, rétention ≤ 7 jours ([sécurité](https://docraptor.com/security-and-privacy)).
- **Validation** : `strict=html` seulement.
- **Perf** : délai max 60 s en synchrone, 600 s en asynchrone.
- **API** : SDK générés par openapi-generator ([docraptor-python](https://github.com/DocRaptor/docraptor-python)) ; asynchrone avec `callback_url`.
- **Avancé** : PDF/A-1a…3b, PDF/UA-1, A+UA combinés, PDF/X, chiffrement ; Factur-X et pièces jointes non trouvés ([paramètres](https://docraptor.com/documentation/api/parameters)).

### 1.4 Prince 17 (sept. 2026)

- **Template** : convertisseur HTML/XML/Markdown + CSS, pas de fusion de données ([input](https://www.princexml.com/doc/prince-input/)). Outil CLI, pilotable par le Prince Control Protocol ([CLI](https://www.princexml.com/doc/command-line/)).
- **Réutilisation** : feuilles de style multiples ; XInclude désactivé par défaut, avec un avertissement pour les entrées non fiables ([input](https://www.princexml.com/doc/prince-input/)).
- **Sécurité** : « Prince is not running JavaScript by default » ([JS](https://www.princexml.com/doc/javascript/)) ; `--no-network`, `--no-local-files` ([CLI](https://www.princexml.com/doc/command-line/)).
- **Avancé** :
  - PDF/A-1a…3b, PDF/UA-1, PDF/X ;
  - **`--attach` et `--attach-data`, présentés comme « mostly needed for Factur-X/ZUGFeRD »** ([profils](https://www.princexml.com/doc/pdf-profiles/)) ;
  - `--pdf-xmp`, sortie raster PNG/JPEG ([CLI](https://www.princexml.com/doc/command-line/)) ;
  - champs de signature `required`, sans signature cryptographique ([release 17](https://www.princexml.com/releases/17/)).

### 1.5 PDFMonkey (moteur v5 = Chrome 133)

- **Template** : HTML + SCSS + **Liquid**, ou éditeur visuel. Le `payload` JSON doit être un objet ; `meta` est limité à 200 Ko ([docs](https://pdfmonkey.io/docs/), [documents](https://pdfmonkey.io/docs/api/documents/)).
- **Réutilisation · versions** :
  - **Snippets au niveau du compte** (`load_snippets`, puis `{% include 'nom', var: … %}`) et **partials** propres au template ([reusing code](https://pdfmonkey.io/docs/code-templates/reusing-code/)).
  - **Brouillon/publié** champ par champ (`body_draft`, `sample_data`, `pdf_engine_id`) ; `preview_url` sur le brouillon ([templates API](https://pdfmonkey.io/docs/api/templates/)). Pas d'historique trouvé.
  - Moteur figé par template ([engines](https://pdfmonkey.io/docs/code-templates/our-engines/)).
  - CLI `template watch` qui synchronise des fichiers locaux ([CLI](https://pdfmonkey.io/docs/code-templates/cli/)).
- **Design** : format, marges, en-têtes et pieds de page dans les réglages du template ([layout](https://pdfmonkey.io/docs/code-templates/page-layout/)).
- **Sécurité** : payload chiffré, hébergement UE ; sandbox et réseau non documentés ([sécurité](https://pdfmonkey.io/docs/account-and-security/security-measures/)).
- **Validation** : pas de schéma (422 si un champ requis manque) ([documents](https://pdfmonkey.io/docs/api/documents/)).
- **Perf** : délai max de 30 s à 5 min selon le plan ([engines](https://pdfmonkey.io/docs/code-templates/our-engines/)).
- **Avancé** : sortie image, formulaires, mot de passe ; PDF/A et Factur-X non trouvés.

### 1.6 APITemplate.io (API v2)

- **Template** : **Jinja2** avec des filtres QR, code-barres et devises ([template language](https://apitemplate.io/docs/template-language/introduction/)). Éditeur avec onglets Template, CSS, Sample JSON et Settings ([éditeur](https://apitemplate.io/docs/pdf-generation/html-template-editor/)).
- **Réutilisation · versions** :
  - `include`, `macro` et `extends` non documentés ;
  - aucun endpoint de versions ; `update-template` est expérimental ;
  - `rendering_engine_version` « 97 » / « 153 » fige le moteur.
  - Source : [OpenAPI](https://apitemplateio.s3.ap-southeast-1.amazonaws.com/redoc_apiv2/apitemplateiov2_api.yaml).
- **Sécurité** : logs conservés 2 semaines ; URL de PDF sans expiration par défaut ([FAQ sécurité](https://apitemplate.io/docs/faq/secure-pdf-generation/)).
- **API** : **OpenAPI 3.0 public**, asynchrone + webhook ([REST](https://apitemplate.io/docs/integrations/rest-api/)).
- **Avancé** :
  - `merge-pdfs` ;
  - sorties PNG/JPEG ;
  - **`einvoice=true` (expérimental)** : PDF/A-3 Factur-X ou ZUGFeRD à partir de `einvoice_xml`, profils MINIMUM à XRECHNUNG, `einvoice_validate` en option ([OpenAPI](https://apitemplateio.s3.ap-southeast-1.amazonaws.com/redoc_apiv2/apitemplateiov2_api.yaml)).

### 1.7 Docmosis (Tornado 2.11.4, Cloud DWS4)

- **Template** : DOCX/ODT avec `<<champ>>`, sections répétées et conditionnelles, expressions, codes-barres ([Template Guide](https://resources.docmosis.com/Documentation/Tornado/2.11.1/Tornado-Template-Guide-2.11.1.pdf)).
- **Réutilisation · versions** :
  - `<<ref:/common/header.docx>>` (chemins relatifs ou absolus) et `<<refLookup:champ>>` (indirection), imbriquables à volonté. C'est jugé « noticeably slower ».
  - `<<coordinator:>>` combine plusieurs templates.
  - Source : [Template Guide §3.16](https://resources.docmosis.com/Documentation/Tornado/2.11.1/Tornado-Template-Guide-2.11.1.pdf).
  - Pas de versions : l'upload écrase le template ; `keepPrevOnFail` garde l'ancien si le nouveau a des erreurs ; environnements dev/test/prod ([Cloud guide](https://resources.docmosis.com/Documentation/Cloud/DWS4/Cloud-Web-Services-Guide-DWS4.pdf)).
  - Tornado lit `DOCMOSIS_TEMPLATESDIR` ([tornado-docker](https://github.com/Docmosis/tornado-docker)).
- **Design** : styles Word ; `stylesInText` dans les données.
- **Sécurité** : aucune connexion sortante requise ; images par URL désactivées par défaut, avec une liste blanche par préfixe ([Security Guide](https://resources.docmosis.com/Documentation/Tornado/2.11.1/Tornado-Deployment-Security-Guide-2.11.1.pdf), [WS Guide](https://resources.docmosis.com/Documentation/Tornado/2.11.1/Tornado-Web-Services-Guide-2.11.1.pdf)).
- **Validation** (la plus proche d'un schéma, côté services) : **`strictParams`** rejette les paramètres inconnus ; **`/getTemplateStructure`** décrit les champs ; **`/getSampleData`** génère un JSON d'exemple ; le mode production échoue sur toute erreur ([WS Guide](https://resources.docmosis.com/Documentation/Tornado/2.11.1/Tornado-Web-Services-Guide-2.11.1.pdf)).
- **API** : OpenAPI téléchargeable depuis la console.
- **Avancé** :
  - PDF/A-1b/2b/3b, PDF/UA, PDF balisé ;
  - **`pdfEInvoiceSpec`** (ZUGFeRD v1/v2, Order-X, Factur-X EN16931) ;
  - fusion de plusieurs templates, formulaires ;
  - signatures : non trouvé.
  - Source : [WS Guide](https://resources.docmosis.com/Documentation/Tornado/2.11.1/Tornado-Web-Services-Guide-2.11.1.pdf).

## 2. Moteurs et bibliothèques de templates

### 2.1 Docxtemplater (3.71.0)

- **Template** : DOCX avec `{x}`, `{#x}…{/x}` (boucle ou condition), `{^x}` (inverse), `{@x}` (XML brut) ([tag types](https://docxtemplater.com/docs/tag-types/)). Expressions via angular-expressions ([angular-parse](https://docxtemplater.com/docs/angular-parse/)).
- **Réutilisation** : module **payant** Subtemplate `{:include x}`. Les en-têtes et pieds de page ne sont pas inclus ; en cas de conflit, le style du maître s'applique, sauf avec `keepStyles` ([subtemplate](https://docxtemplater.com/modules/subtemplate/)). Paquets et versions : non trouvés.
- **Sécurité** : par défaut, le parser d'expressions utilise `new Function`. L'option `csp: true` le désactive ([FAQ](https://docxtemplater.com/faq/)) ; elle est par défaut depuis 3.68.7. La doc recommande aussi `disabledSyntaxes: ["CallExpression"]` ([angular-parse](https://docxtemplater.com/docs/angular-parse/)). Aucun appel réseau ([FAQ](https://docxtemplater.com/faq/)).
- **Validation** : `nullGetter` seulement.
- **Perf** : 250 rendus d'un document de 50 pages en 44 s ; boucle de 1000 éléments < 60 ms ([FAQ](https://docxtemplater.com/faq/)).
- **Avancé** : pas de PDF. La FAQ renvoie vers `libreoffice --headless`, avec un rendu « not 100% perfect » ([FAQ](https://docxtemplater.com/faq/)).

### 2.2 pdfme (6.2.4)

- **Template** : JSON `{basePdf, schemas[]}`, des champs positionnés et un tableau d'`inputs` ([getting started](https://pdfme.com/docs/getting-started)). Expressions `{…}` dans les champs en lecture seule ([expression](https://pdfme.com/docs/expression)). `@pdfme/jsx` produit des templates pdfme en JSX, sans React ([jsx](https://github.com/pdfme/pdfme/tree/main/packages/jsx)).
- **Réutilisation** : **plugins** en trois parties (`pdf`, `ui`, `propPanel`), partagés comme modules npm ([custom schemas](https://pdfme.com/docs/custom-schemas)). Depuis la v6, seul `text` est enregistré par défaut ([migration v6](https://pdfme.com/docs/migration-v6)). En-têtes et pieds de page par `staticSchema` ([headers](https://pdfme.com/docs/headers-and-footers)). Galerie de templates par PR ([contribution](https://pdfme.com/docs/template-contribution-guide)).
- **Sécurité** : expressions limitées aux fonctions fléchées ; `eval` et `prototype` interdits ; objets globaux en liste blanche ([expression](https://pdfme.com/docs/expression)).
- **Validation** : **zod** (`checkTemplate`, `checkInputs`) ([helper.ts](https://github.com/pdfme/pdfme/blob/main/packages/common/src/helper.ts)). La CLI `pdfme validate --strict --json` renvoie `errors`, `warnings` et `inputHints` ([CLI](https://pdfme.com/docs/cli)).
- **Avancé** : `@pdfme/manipulator` (merge, split, rotate) ([manipulator](https://pdfme.com/docs/manipulator)) ; signature dessinée, sans signature cryptographique ; PDF/A non trouvé.

### 2.3 @react-pdf/renderer (4.9.0)

- **Template** : composants React (`Document`, `Page`, `View`, `Text`, `Image`…) alimentés par les props ([components](https://react-pdf.org/components)).
- **Réutilisation** : composants React distribués par npm. `fixed` répète un élément sur chaque page ; `wrap` et `break` contrôlent la pagination ([page wrapping](https://react-pdf.org/docs/v4/advanced/page-wrapping)).
- **Design** : `StyleSheet.create`, Flexbox, media queries ; pas de système de thème ([styling](https://react-pdf.org/styling)).
- **Sécurité** : les emojis exigent internet au rendu ([fonts](https://react-pdf.org/fonts)) ; `Image.src` accepte une URL avec méthode et en-têtes ([image](https://react-pdf.org/docs/v4/components/image)). Contrôle réseau : non trouvé.
- **API** : `renderToBuffer`, `renderToStream` ([node](https://react-pdf.org/node)).
- **Avancé** : prop `conformance` ([Document](https://react-pdf.org/docs/v4/components/document)). La 4.9.0 produit du PDF/A-1/2/3 b ([releases](https://github.com/diegomura/react-pdf/releases)). PDF/UA et Factur-X non trouvés.

### 2.4 React Email (6.11.1) : référence du modèle de composants

- **Template** : composants React d'e-mail (`Html`, `Section`, `Row`, `Column`, `Button`, `Text`, `Tailwind`…) ([llms.txt](https://react.email/docs/llms.txt)). Les données passent par les props. **`Email.PreviewProps`** fournit les props d'aperçu ([CLI](https://react.email/docs/cli.md)).
- **Réutilisation · versions** :
  - 6.0.0 (16 avril 2026) regroupe tous les composants dans le paquet unique `react-email` ([changelog](https://react.email/docs/changelog.md)) ;
  - le serveur d'aperçu ignore les dossiers `_components`, ce qui en fait l'emplacement conventionnel des composants partagés ; `static/` pour les assets ([CLI](https://react.email/docs/cli.md)) ;
  - versions par npm/semver.
- **Thème** :
  - composant `Tailwind` avec une prop `config` (`theme.extend`, presets) ; styles inlinés ([tailwind](https://react.email/docs/components/tailwind.md)) ;
  - thèmes de l'Editor : `basic` et `minimal`. Un thème est une **table composant → `CSSProperties`**, personnalisable avec `extendTheme('basic', …)` ou `createTheme` ([theming](https://react.email/docs/editor/features/theming)).
- **API** : `email dev` (aperçu avec rechargement), `export`, `render()` ([CLI](https://react.email/docs/cli.md), [render](https://react.email/docs/utilities/render.md)).
- **Leçon pour inkpdf** : le triptyque « composant headless + props typées + thème par table de styles avec `extend` » se transpose en Typst. Un paquet expose des fonctions (composants) qui prennent `design` en argument, et une fonction `theme(base, overrides)` fusionne les surcharges sur les défauts.

### 2.5 jsreport (4.14.2)

- **Template** : Handlebars (recommandé), JsRender, EJS ou Pug, avec des helpers JS exécutés dans un sandbox ([engines](https://jsreport.net/learn/templating-engines)). `POST /api/report` prend `{template, data, options}` ([API](https://jsreport.net/learn/api)).
- **Réutilisation · versions** : le modèle de réutilisation le plus abouti.
  - **Assets** : `{{asset "x.css"}}`. Les helpers peuvent être partagés globalement ou par sous-arbre de dossiers ; `jsreport.assets.require` charge un asset comme module ([assets](https://jsreport.net/learn/assets)).
  - **Components** : `{{component "./customer"}}`, qui reçoit automatiquement le contexte courant, « perform great even when thousands of components are rendered in a loop » ([components](https://jsreport.net/learn/components)).
  - **Child templates** : rendu complet, sortie texte uniquement ([child templates](https://jsreport.net/learn/child-templates)).
  - **Version control** : commit, diff et revert stockés dans le template store ; git est cité comme alternative ([version control](https://jsreport.net/learn/version-control)).
  - Export `.jsrexport` ([import-export](https://jsreport.net/learn/import-export)).
- **Sécurité** :
  - sandbox par défaut, avec `allowedModules` ; **`trustUserCode: true` retire le sandbox** ([configuration](https://jsreport.net/learn/configuration)) ;
  - docker-workers : un conteneur par requête, système de fichiers en lecture seule, 0,5 CPU / 420 Mo ([docker-workers](https://jsreport.net/learn/docker-workers)) ;
  - correctifs de sécurité en 4.14.1/4.14.2 ([releases](https://github.com/jsreport/jsreport/releases)).
- **Validation** : JSON Schema non trouvé ; données d'exemple liées ([inline data](https://jsreport.net/learn/inline-data)).
- **API** : REST + OData `$metadata` ; OpenAPI non trouvé ([API](https://jsreport.net/learn/api)).
- **Avancé** (pdf-utils) : merge et append, table des matières, `addAttachment`, **`pdfSign` (p12)**, PDF/A-1B en bêta, `pdfUA`, mots de passe ([pdf-utils](https://jsreport.net/learn/pdf-utils)).

### 2.6 WeasyPrint (70.0, 8 sept. 2026)

- **Template** : HTML + CSS Paged Media, sans moteur de template ([use cases](https://doc.courtbouillon.org/weasyprint/stable/common_use_cases.html)).
- **Sécurité** : la doc prévient que du contenu non fiable peut provoquer des boucles « quite easily » et lire `file://` (y compris `/dev/urandom`) ; elle recommande un sandbox et un fetcher personnalisé ([first steps](https://doc.courtbouillon.org/weasyprint/stable/first_steps.html)). `URLFetcher(allowed_protocols, timeout…)` ([API](https://doc.courtbouillon.org/weasyprint/stable/api_reference.html)). CVE corrigées en 68, 69 et 70 ([changelog](https://doc.courtbouillon.org/weasyprint/stable/changelog.html)).
- **Réutilisation · versions** : sans objet côté outil (HTML/CSS externes) ; paquets non trouvés.
- **Design** : CSS ; `output_intent` (srgb, device-cmyk…) ([API](https://doc.courtbouillon.org/weasyprint/stable/api_reference.html)).
- **Validation** : sans objet.
- **Perf** : non trouvé.
- **API** : Python (`write_pdf`) et CLI ; serveur HTTP et OpenAPI non trouvés ([API](https://doc.courtbouillon.org/weasyprint/stable/api_reference.html)).
- **Avancé** :
  - `pdf_variant` : A-1b…4f, **UA-1 et UA-2**, X-1a…X-5g, mais les documents sont « not guaranteed to be valid » ;
  - `Attachment(relationship)`, `custom_metadata`, hook `finisher` ([API](https://doc.courtbouillon.org/weasyprint/stable/api_reference.html)) ;
  - **Factur-X** : PDF/A-3b + XML avec `relationship=Data` + `--xmp-metadata` ([use cases](https://doc.courtbouillon.org/weasyprint/stable/common_use_cases.html)) ; CLI Factur-X depuis la 68.0 ([changelog](https://doc.courtbouillon.org/weasyprint/stable/changelog.html)).

### 2.7 Paged.js (0.4.3)

- Polyfill Paged Media qui tourne dans le navigateur, avec des handlers et hooks (`registerHandlers`) ; `pagedjs-cli` passe par Puppeteer ([dépôt](https://github.com/pagedjs/pagedjs)).
- Margin boxes, `running()`, `string-set` ([generated content](https://pagedjs.org/en/documentation/7-generated-content-in-margin-boxes/)).
- Une version « 2.0 » a été annoncée en mai 2025, avec des ruptures prévues ([annonce](https://pagedjs.org/posts/en/a-new-chapter-in-paged.js-life/)) ; npm reste en 0.4.3 ([npm](https://registry.npmjs.org/pagedjs/latest)).
- **Template** : HTML + CSS ; données injectées en amont (pas de moteur).
- **Réutilisation · versions** : handlers JS ; pas de paquets de templates.
- **Design** : CSS Paged Media (named pages, références croisées) ([documentation](https://pagedjs.org/en/documentation/)).
- **Sécurité, validation, perf** : non trouvés.
- **API** : bibliothèque JS + `pagedjs-cli` (Puppeteer) ; OpenAPI sans objet.
- **Avancé** : PDF/A, UA, Factur-X, signatures non trouvés ; le PDF vient de Chromium.

### 2.8 Puppeteer (25.13.0) et 2.9 Playwright (1.64.0)

- **Puppeteer** : `page.pdf` imprime la page en média `print` ([page.pdf](https://pptr.dev/api/puppeteer.page.pdf)). Les en-têtes et pieds de page n'acceptent que les classes `pageNumber`, `totalPages`… ; **`tagged` vaut `true` par défaut** (expérimental), `outline` vaut `false` ([PDFOptions](https://pptr.dev/api/puppeteer.pdfoptions)). « Running without a sandbox is strongly discouraged » ([troubleshooting](https://pptr.dev/troubleshooting)) ; interception réseau possible ([interception](https://pptr.dev/guides/network-interception)). Sous WebDriver BiDi (Firefox), seule une partie des options est supportée ([BiDi](https://pptr.dev/webdriver-bidi)).
- **Playwright** : mêmes options, mais **`tagged` et `outline` valent `false` par défaut** (v1.42) ; les `<script>` des en-têtes ne sont pas évalués ([page.pdf](https://playwright.dev/docs/api/class-page#page-pdf)) ; blocage réseau par `page.route` ([network](https://playwright.dev/docs/network)) ; PDF Chromium uniquement ([MCP pdf](https://playwright.dev/mcp/tools/pdf)).
- Pour les deux :
  - **Template** : aucun ;
  - **Réutilisation, validation** : sans objet ;
  - **Design** : CSS ;
  - **Perf** : non trouvé ;
  - **API** : bibliothèques, OpenAPI sans objet ;
  - **Avancé** : PDF balisé et plan, mais pas de PDF/A, Factur-X ni signature. La conformité doit venir d'un post-traitement.

### 2.10 LaTeX (TeX Live 2026) et services LaTeX

- **Réutilisation · versions** :
  - paquets CTAN versionnés un par un (ex. `zugferd` 0.13 du 2026-07-23 ([CTAN](https://ctan.org/pkg/zugferd)), `pdfx` 1.6.5f ([CTAN](https://ctan.org/pkg/pdfx))), mais installés **globalement** dans une distribution annuelle ([texlive](https://ctan.org/pkg/texlive)) ;
  - `tlmgr update -all` met tout à jour, avec `tlmgr restore` pour revenir en arrière ([guide TeX Live](https://texdoc.org/serve/texlive-en/0)) ;
  - le document ne peut pas épingler la version d'un paquet. Seul le noyau a un rollback daté, via `latexrelease` ([ltnews28](https://texdoc.org/serve/ltnews28/0), [latexrelease](https://ctan.org/pkg/latexrelease)).
- **Sécurité** : `\write18` restreint par défaut ; **`openin_any` n'a plus d'effet en 2026, toute lecture est permise** ; pour les entrées non fiables, la doc recommande « use a new subdirectory or chroot » ([guide TeX Live](https://texdoc.org/serve/texlive-en/0)).
- **Avancé** :
  - `\DocumentMetadata{pdfstandard=…}` couvre A-1b…A-4F, X-4…X-6p, UA-1/UA-2, mais la clé « doesn't mean that the document actually follows the standard » ([documentmetadata](https://texdoc.org/serve/documentmetadata-support/0)) ;
  - paquet `zugferd` pour ZUGFeRD et Factur-X ([CTAN](https://ctan.org/pkg/zugferd)).
- **Services** :
  - [latex-online](https://github.com/aslushnikov/latex-online) : `/compile?url|text|git` ;
  - [Overleaf CLSI](https://github.com/overleaf/overleaf/tree/main/services/clsi) : JSON `resources[]`, `SANDBOXED_COMPILES` avec un conteneur frère par projet (en montant le socket Docker), sans réseau hôte ([Server Pro](https://docs.overleaf.com/on-premises/configuration/overleaf-toolkit/server-pro-only-configuration/sandboxed-compiles)) ;
  - [texlive.net](https://texlive.net/) : multipart `filename[]`/`filecontents[]`, paramètre `engine` ([latexcgi](https://davidcarlisle.github.io/latexcgi/)).
  - Tous exécutent du **source fourni par l'appelant**.

## 3. Écosystème Typst : paquets, templates et services

### 3.1 Paquets Typst et Typst Universe (référence directe pour la spec 002)

- **Modèle** : un paquet est un dossier versionné avec un manifeste `typst.toml` ; import par triplet exact `#import "@preview/example:0.1.0": add` ([docs scripting](https://typst.app/docs/reference/scripting/)). « You must always specify the full package version » ([typst/packages](https://github.com/typst/packages)) ; un import sans version échoue au parsing avec `package specification is missing version` (source `typst-syntax-0.15.1/src/package.rs`, [dépôt](https://github.com/typst/typst/tree/main/crates/typst-syntax/src)).
- **Arborescence locale** : `{data-dir}/typst/packages/{namespace}/{name}/{version}` ; namespace arbitraire autorisé (« You can create an arbitrary `{namespace}` », ex. `@local`) ; cache `{cache-dir}/typst/packages/preview` ; le data-dir a priorité sur le cache ([typst/packages](https://github.com/typst/packages)). La structure `packages/<ns>/<nom>/<version>/` supposée par la spec 002 est donc exactement celle de Typst.
- **Manifeste** : `[package]` requis par le compilateur : `name`, `version`, `entrypoint` ; optionnels : `compiler` (version minimale), `exclude`, `authors`, `license`, `description`, `categories`… ; `[template]` (`path`, `entrypoint`, `thumbnail`) pour les paquets-templates ; `[tool.<outil>]` réservé aux outils tiers. **Aucun champ de déclaration de dépendances** ([manifest.md](https://github.com/typst/packages/blob/main/docs/manifest.md)). Conséquence : si inkpdf veut détecter au chargement les paquets manquants (FR-009 b), la liste doit vivre dans `template.json` (ou sous `[tool.inkpdf]`), pas dans un champ standard ; sinon il faut analyser les imports (FR-009 c).
- **Résolution (typst-kit 0.15.1)** : `SystemPackages::obtain` essaie data-dir → cache → **téléchargement depuis Universe si namespace `preview`** ; sinon `NotFound` ([packages.rs](https://github.com/typst/typst/blob/main/crates/typst-kit/src/packages.rs), lu en version 0.15.1). C'est exactement le comportement qu'inkpdf ne doit pas hériter : il faut une résolution propre sur l'instantané du template (FR-003/FR-004), pas `SystemPackages`.
- **Immutabilité** : « Once submitted, a package will not be changed or removed without good reason » ; correctifs = nouvelles versions ([docs/README.md](https://github.com/typst/packages/blob/main/docs/README.md)). Seul namespace publiable : `preview` (même source). Pas de semver « ^ » : versions exactes, donc deux versions d'un même paquet coexistent sans conflit.
- **Templates Universe et `typst init`** : `typst init @preview/x[:v]` résout la dernière version si absente, exige `[template]`, puis **copie** `template.path` dans un nouveau dossier (refuse d'écraser) ([init.rs](https://github.com/typst/typst/blob/main/crates/typst-cli/src/init.rs)). Le template copié importe la bibliothèque **par version exacte** : `charged-ieee` 0.1.4 a `entrypoint = "lib.typ"`, `[template] path = "template"`, et `template/main.typ` commence par `#import "@preview/charged-ieee:0.1.4": ieee` ([typst.toml](https://github.com/typst/packages/blob/main/packages/preview/charged-ieee/0.1.4/typst.toml), [main.typ](https://github.com/typst/packages/blob/main/packages/preview/charged-ieee/0.1.4/template/main.typ)). Les consignes demandent d'importer `@preview/my-package:0.1.0` « rather than `../lib.typ` » ([docs/README.md](https://github.com/typst/packages/blob/main/docs/README.md)). **Modèle clé** : le scaffold est copié une fois (et diverge), le code qui évolue vit dans une bibliothèque versionnée et immuable. C'est le patron « briques simples qui évoluent » à reprendre.
- **Thème/design** : pas de notion de thème standard ; conventions de fonction `#show: tpl.with(...)` paramétrée (même exemple charged-ieee).
- **Sécurité** : paquets soumis à revue, ne doivent pas « exploit the compiler or packaging system or exfiltrate user data » ([docs/README.md](https://github.com/typst/packages/blob/main/docs/README.md)) ; les chemins relatifs se résolvent depuis le fichier appelant, `/` désigne la racine du projet (ou celle du paquet, dans un paquet), et un projet « can only access paths within its project root » ([path](https://typst.app/docs/reference/foundations/path/)).
- **Validation d'entrée** : rien de standard ; le paquet [jsonschemeyst](https://typst.app/universe/package/jsonschemeyst/) valide du JSON contre JSON Schema dans le document (WASM), utile en preview mais pas comme garde serveur.

### 3.2 Typst lui-même : fonctions PDF avancées (0.15.1)

- PDF 1.4–2.0, **PDF/A-1b/1a/2b/2u/2a/3b/3u/3a/4/4f/4e**, **PDF/UA-1** (pas UA-2) ; UA-1 incompatible avec A-4 et PDF 2.0 ; Tagged PDF par défaut ([docs pdf](https://typst.app/docs/reference/pdf/)).
- `pdf.attach(path, data:, relationship: "source"|"data"|"alternative"|"supplement", mime-type:, description:)` ; `relationship` ignoré hors PDF/A-3 ; la doc cite ZUGFeRD/Factur-X ([pdf.attach](https://typst.app/docs/reference/pdf/attach/)). `data:` permet d'attacher des octets calculés, donc un XML transmis dans `data`.
- **Manque pour Factur-X** : métadonnées XMP personnalisées (schéma d'extension `fx:`) toujours non supportées ; [issue #5667](https://github.com/typst/typst/issues/5667) ouverte, « The API design is still not settled » (mars 2026), commentaires jusqu'en sept. 2026. Un PDF Factur-X conforme exige donc un post-traitement XMP aujourd'hui.
- Pas de signature ni de formulaires dans la doc PDF ([docs pdf](https://typst.app/docs/reference/pdf/)).
- Perf : compilations « commonly complete in milliseconds », binaire ~40 Mo, économie de 5 à 100 ms de démarrage en bibliothèque vs CLI ([blog Typst 2025](https://typst.app/blog/2025/automated-generation/)).

### 3.3 Bibliothèques et services Typst

| Projet | Forme | Entrée | Paquets / réutilisation | Validation | API | Notes |
|---|---|---|---|---|---|---|
| [typst-as-lib](https://github.com/Relacibo/typst-as-lib) | crate Rust | `sys.inputs` via `compile_with_input` | résolveurs statiques/mémoire/FS (racine bornée), `local_package_root`, paquets distants (feature `packages`, téléchargement paresseux, cache FS ou `InMemoryCache`) | non | — | API instable ; une seule version Typst |
| [typst.ts](https://github.com/Myriad-Dreamin/typst.ts) | WASM (navigateur, Node) | — | non documenté sur la page | non | — | ~1,2 k étoiles, rendu SVG/canvas, Apache-2.0 |
| [Oicana](https://oicana.com/compare/gotenberg/) | lib multi-langage + WASM | entrées déclarées `[[tool.oicana.inputs]]` (json, blob) avec valeurs `development` pour la preview ([Universe](https://typst.app/universe/package/oicana)) | template packé en zip (`oicana pack`) | **JSON Schema avant Typst** → `ValidationFailed` (400) ([doc Rust](https://oicana.com/docs/getting-started/4-5-rust/)) | exemple Axum avec OpenAPI | source-available, payant en commercial ; « single digit milliseconds » à chaud |
| [papermake](https://github.com/rkstgr/papermake) | registre + serveur Axum | `sys.inputs.data` | **registre adressé par contenu** (SHA-256, blobs S3), tags mutables `invoice:latest` vs digests immuables `invoice@sha256:…`, journal des rendus (hash entrée/sortie) ([billet](https://www.ersteiger.com/posts/papermake-registry/)) | schéma optionnel à l'upload, validation non confirmée | REST publish/render | 179 étoiles, Apache-2.0 |
| [navikt/pdfgenrs](https://github.com/navikt/pdfgenrs) | service Rust (NAV, Norvège) | JSON + `templates/<app>/` | `resources/` et `fonts/` globaux | non (limite 2 MiB, RFC 9457) | REST + metrics Prometheus | **PDF/A-2a + PDF/UA-1 simultanés** ; pas de rechargement à chaud ; MIT |
| [tweedegolf/typst-webservice](https://github.com/tweedegolf/typst-webservice) | lib + serveur Axum | `input.json` | `PdfContext` en mémoire (sources, polices, assets) partagé via `Arc` | non | GET rendu, POST batch → ZIP streamé | paquets non mentionnés |
| [simplebash document-server](https://github.com/simplebash-official/document-server) | service Axum | `sys.inputs` | **`templates/lib/` pour paquets Typst partagés** | JSON Schema par template, erreurs par champ | OpenAPI + Swagger, sync manuel `POST /api/templates/sync` | AGPL-3.0, 0 étoile |
| [Mapaor/typst-api](https://github.com/Mapaor/typst-api) | service Docker | source Typst brute (JSON ou multipart) | télécharge le registre (préchargement, « ~1.9 GB » pour tout) | non | REST + admin | exécute du code fourni par l'appelant |
| [Typsetter](https://typsetter.dev/) | SaaS (bêta) | JSON + placeholders `{{ }}` | 26+ templates à cloner | non mentionnée | OpenAPI 3.1, webhooks, async | « <500ms » moyen annoncé |

## 4. Ce qu'inkpdf devrait emprunter

Classement par rapport valeur/coût. Chaque idée cite les sources qui la motivent, et indique son lien avec la spec 002 quand il existe.

| # | Idée | Valeur | Coût | Inspiration |
|---|---|---|---|---|
| 1 | **Paquet = bibliothèque versionnée immuable, import par version exacte ; template = scaffold qui l'importe.** Les helpers maison deviennent un paquet Typst ordinaire (ex. `@inkpdf/helpers:0.1.0`), écrit dans un dépôt à part, versionné en semver et jamais modifié après publication. | très haute | faible : c'est le format de 002 | Universe : versions immuables ; `charged-ieee` (bibliothèque `lib.typ` + `template/main.typ` qui importe `@preview/charged-ieee:0.1.4`) ; consigne « rather than `../lib.typ` » (§3.1) |
| 2 | **US4 : choisir (b), pas (a).** Un paquet de helpers embarqué dans l'image lie son évolution aux releases du binaire, ce qui va contre « binaire figé, templates à chaud » (principe V) et contre « briques qui évoluent ». En (b), chaque template choisit sa version et les migrations sont progressives. | haute | nul : seulement de la doc et un dépôt d'exemple | modèle npm de React Email / react-pdf (§2.3–2.4) ; Carbone `preReleaseFeatureIn` pour l'épinglage (§1.2) |
| 3 | **Exposer les standards PDF dans `template.json`** (ex. `"pdf": {"standards": ["a-2b", "ua-1"]}`), avec un échec au chargement si la combinaison est invalide (UA-1 est incompatible avec A-4 et PDF 2.0). Aujourd'hui inkpdf appelle `typst_pdf::pdf` avec `PdfOptions::default()` (seuls `ident` et `timestamp` sont fixés, `src/render/mod.rs`) : aucun standard n'est demandé. C'est un changement du format de template, donc OpenAPI et `docs/templates.md` doivent évoluer dans la même PR (constitution). | haute | faible : `PdfStandards` existe dans typst-pdf 0.15.1 | Typst [pdf](https://typst.app/docs/reference/pdf/) ; pdfgenrs A-2a+UA-1 ; Docmosis `pdfVersion` (§1.7) |
| 4 | **Données d'exemple dans le template** (`examples/*.json`, ou mot-clé `examples` de JSON Schema), exposées par `GET /templates/{id}` et rendues en CI. Elles servent à la fois de doc, de test de non-régression et de base d'aperçu. | haute | faible | Carbone `sample`, PDFMonkey `sample_data`, React Email `PreviewProps`, Oicana valeurs `development`, Docmosis `getSampleData` |
| 5 | **FR-009 : (a) + (c) léger.** Le contrôle au rendu reste la référence. En plus, au chargement, on analyse les imports littéraux `@ns/nom:ver` avec typst-syntax (dépendance transitive de `typst`) : un paquet manquant marque le template invalide. Pas de liste déclarée (b), puisque `typst.toml` n'a **aucun champ de dépendances** et qu'une liste à part se désynchroniserait. | haute | moyen | [manifest.md](https://github.com/typst/packages/blob/main/docs/manifest.md) ; validation précoce d'Oicana (`ValidationFailed` avant Typst) ; `keepPrevOnFail` de Docmosis |
| 6 | **FR-005 : (a), c'est le mécanisme natif de Typst.** Typst 0.15 dit que « code in a package cannot construct a path that lives in the project or another package » ; pour donner un fichier du projet à un paquet, « you create the path in project code with the `path()` constructor and pass it in » ([path](https://typst.app/docs/reference/foundations/path/)). `image` accepte aussi « str or path or bytes » ([image](https://typst.app/docs/reference/visualize/image/)). Le template transmet donc l'image (en `path` ou en `bytes`) ; le paquet n'a aucun droit de lecture en plus. Point d'attention pour 002 : le `World` d'inkpdf doit honorer ces `path` (des `FileId` hors du paquet, créés par le template) sans ouvrir de lecture libre. | haute | faible : comportement natif, à tester | Oicana : `setup(read-project-file)` ([Universe](https://typst.app/universe/package/oicana)) ; `pdf.attach(data:)` accepte des octets ([attach](https://typst.app/docs/reference/pdf/attach/)) |
| 7 | **Empreinte de contenu du template** (SHA-256 de l'instantané, paquets compris) dans le détail et dans un en-tête de réponse du rendu. Avec le déterminisme, on peut dire quel template exact a produit quel PDF. inkpdf calcule déjà une `Fingerprint` (hash u64 de chemin, taille et mtime, `src/registry/fingerprint.rs`), mise dans l'`ident` du PDF mais non exposée ; ce n'est pas une empreinte de contenu. | moyenne-haute | faible | papermake : digests contre tags mutables ; Carbone `versionId` SHA-256 |
| 8 | **Vérifier le champ `compiler` des manifestes** et accepter un `typst` minimal dans `template.json`, avec un diagnostic explicite (déjà prévu dans les edge cases de 002). | moyenne | faible | `[package] compiler` ([manifest.md](https://github.com/typst/packages/blob/main/docs/manifest.md)) ; DocRaptor `pipeline`, APITemplate `rendering_engine_version` |
| 9 | **Thème = fonction de fusion sur des défauts**, dans le paquet de helpers : `theme(design)` complète `design` avec les défauts du schéma, puis les composants lisent le thème. C'est la première brique du futur « headless », sans framework. | moyenne | faible : Typst pur | React Email `extendTheme`/`createTheme` (table composant → styles) ; Carbone `styleSource` |
| 10 | **Dossier de paquets partagé (plus tard)** : volume en lecture seule `packages/<ns>/<nom>/<ver>` commun aux templates. Les copies du template seraient prioritaires et la résolution resterait exacte. C'est sûr justement parce que les versions sont immuables. | moyenne | moyen : invalidation du rechargement à chaud entre templates | data-dir prioritaire sur le cache dans Typst ([typst/packages](https://github.com/typst/packages)) ; jsreport : assets partagés par dossier ; Docmosis `ref:/common/` ; simplebash `templates/lib/` |
| 11 | **Aperçu PNG/SVG** (`/render?format=png&page=1`) via typst-render ou typst-svg, sans navigateur. | moyenne | moyen | Gotenberg screenshots, Carbone `convertTo` png, PDFMonkey `output_type` image, Prince `--raster-output` |
| 12 | **Factur-X** : le template peut déjà attacher un XML tiré de `data` via `pdf.attach(data: bytes(...), relationship: "alternative")` en PDF/A-3b. Mais il manque le XMP `fx:` ([#5667](https://github.com/typst/typst/issues/5667)). À documenter comme limite ; on attend l'API Typst plutôt que de post-traiter le PDF. | moyenne (FR) | faible à documenter ; élevé à contourner | Docmosis `pdfEInvoiceSpec`, Gotenberg `/factur-x`, APITemplate `einvoice`, WeasyPrint `--xmp-metadata` |
| 13 | **Rejet des clés inconnues dans `design`** (`additionalProperties: false` recommandé dans la doc des auteurs). | moyenne | nul | Docmosis `strictParams` ; `template.json` d'inkpdf rejette déjà les clés inconnues |

## 5. Ce qu'il faut éviter

- **Toute résolution implicite ou « latest ».**
  - `SystemPackages` de typst-kit télécharge `@preview` dès que le paquet manque ([packages.rs](https://github.com/typst/typst/blob/main/crates/typst-kit/src/packages.rs)), et `typst init` sans version prend la dernière ([init.rs](https://github.com/typst/typst/blob/main/crates/typst-cli/src/init.rs)).
  - inkpdf doit écrire son propre résolveur sur l'instantané du template et ne jamais activer la feature `packages` de typst-as-lib ([typst-as-lib](https://github.com/Relacibo/typst-as-lib)).
  - Contre-exemples : react-pdf exige internet pour les emojis ([fonts](https://react-pdf.org/fonts)) ; typst-api met en cache tout le registre (« ~1.9 GB ») ([typst-api](https://github.com/Mapaor/typst-api)).
- **Les installations globales non épinglées** : le modèle TeX Live (`tlmgr update -all`, sans épinglage par document) ([guide](https://texdoc.org/serve/texlive-en/0)) est exactement ce qu'un « dossier de paquets partagé » deviendrait sans versions exactes.
- **Les références mutables en production** : tags `latest` de papermake ([billet](https://www.ersteiger.com/posts/papermake-registry/)), écrasement à l'upload chez Docmosis ([Cloud guide](https://resources.docmosis.com/Documentation/Cloud/DWS4/Cloud-Web-Services-Guide-DWS4.pdf)). Si une notion de « canal » apparaît un jour, elle doit pointer vers une empreinte.
- **Exécuter du code fourni par l'appelant** : rendu « anonyme » de jsreport avec `content`, et `trustUserCode` ([API](https://jsreport.net/learn/api), [configuration](https://jsreport.net/learn/configuration)) ; typst-api et les services LaTeX compilent du source arbitraire. C'est contraire au principe IV, et il faut le garder ainsi.
- **Les défauts réseau permissifs** : Gotenberg autorise les IP privées par défaut ([outbound](https://gotenberg.dev/docs/outbound-url-filtering)) ; WeasyPrint lit `file://` par défaut ([first steps](https://doc.courtbouillon.org/weasyprint/stable/first_steps.html)) ; LaTeX autorise toute lecture ([guide](https://texdoc.org/serve/texlive-en/0)).
- **Les inclusions qui fusionnent styles et contexte de façon implicite** : Docxtemplater (styles du maître ou `keepStyles`) ([subtemplate](https://docxtemplater.com/modules/subtemplate/)) ; Docmosis `ref:` (« noticeably slower ») ; child templates de jsreport à sortie texte seulement. Les paquets Typst évitent ce piège parce qu'ils exposent des fonctions et ne partagent pas d'état ; pas besoin d'un mécanisme de partials en plus.
- **Croire qu'une option de conformité suffit** : WeasyPrint (« not guaranteed to be valid ») et LaTeX (« doesn't mean that the document actually follows the standard ») le disent eux-mêmes. Si l'idée n°3 est retenue, il faut valider en CI avec un outil externe (veraPDF, par exemple).
- **Un magasin de templates avec base de données et historique côté serveur** (version control de jsreport, versions de Carbone) : utile en SaaS, mais contraire au principe VII (sans état). Le volume plus git joue ce rôle, et jsreport cite lui-même git comme alternative ([version control](https://jsreport.net/learn/version-control)).
- **Les fonctions éclatées en modules payants ou « Enterprise »** (Docxtemplater PRO, Carbone EE) : elles rendent le comportement dépendant de l'édition.
- **Un framework de composants avant d'en avoir le besoin** : React Email a dû regrouper tous ses paquets en un seul en 6.0 ([changelog](https://react.email/docs/changelog.md)), et Paged.js annonce une 2.0 avec ruptures sans la publier ([annonce](https://pagedjs.org/posts/en/a-new-chapter-in-paged.js-life/)). Mieux vaut commencer par un seul paquet de helpers, petit et versionné.

## 6. Positionnement d'inkpdf

**Ce qui le différencie** (combinaison qu'aucun projet étudié ne réunit) :

1. **Moteur embarqué, sans navigateur ni sous-processus**, contre Chromium ou LibreOffice chez Gotenberg, Carbone, PDFMonkey, APITemplate et Paged.js. Le compilateur Typst fait « a few megabytes » ([Oicana](https://oicana.com/compare/gotenberg/)), soit environ 40 Mo de binaire, avec des compilations en millisecondes ([blog Typst](https://typst.app/blog/2025/automated-generation/)) ; Gotenberg plafonne à 6 conversions Chromium simultanées ([configuration](https://gotenberg.dev/docs/configuration)).
2. **Contrat JSON Schema sur deux dimensions (`data` et `design`), validé avant compilation et publié tel quel.** Côté services : aucun parmi Gotenberg, Carbone, PDFMonkey, APITemplate et jsreport ; seul Docmosis s'en approche avec `strictParams` et `getTemplateStructure`. Côté Typst : Oicana et simplebash valident aussi par JSON Schema, mais aucun ne sépare explicitement design et données.
3. **Bac à sable par construction** : le langage Typst est pur, sans réseau, avec des lectures bornées au dossier ; l'appelant n'envoie jamais de code. À comparer au sandbox JS de jsreport, que l'on peut désactiver, et au durcissement à faire soi-même chez WeasyPrint, LaTeX et Gotenberg.
4. **Templates en volume, rechargés à chaud, avec une API OpenAPI 3.1 générée depuis le code** : pdfgenrs exige un redémarrage ([pdfgenrs](https://github.com/navikt/pdfgenrs)) et simplebash un `POST /sync` ([document-server](https://github.com/simplebash-official/document-server)).
5. **PDF déterministes**, sur lesquels l'idée n°7 (empreinte) peut s'appuyer.
6. **Open source permissif** (licence à trancher), contre Oicana source-available et payant ([Oicana](https://oicana.com/compare/gotenberg/)) et simplebash en AGPL.

**Concurrents les plus proches** :
- **Oicana** : même philosophie (Typst, entrées déclarées, JSON Schema), mais c'est une bibliothèque sous licence commerciale ;
- **simplebash document-server** : service Axum avec schémas et dossier `lib/` partagé, mais 0 étoile et AGPL ;
- **pdfgenrs** : maintenu par une administration (NAV), avec PDF/A+UA, mais sans schéma ni rechargement à chaud ;
- **papermake** : registre de templates, une approche différente et avec état.

**Manques notables** (par ordre de demande probable) :
1. **Paquets et réutilisation** : l'objet de 002. Sans eux, chaque template duplique ses helpers, alors que tous les concurrents mûrs ont un mécanisme (snippets, components, `ref:`, styleSource).
2. **Standards PDF non exposés** (PDF/A, PDF/UA) alors que Typst les fournit : c'est le gain le moins cher (idée n°3).
3. **Données d'exemple et aperçu** (idées n°4 et n°11) : PDFMonkey, Carbone, React Email et Docmosis les ont tous.
4. **Factur-X complet** : bloqué par le XMP personnalisé de Typst ([#5667](https://github.com/typst/typst/issues/5667)) ; Docmosis, Gotenberg et APITemplate le font déjà.
5. **Opérations PDF** (fusion, signature cryptographique, chiffrement, filigrane) : hors du périmètre V1 (principe VII) ; à confier à un service voisin (Gotenberg sait fusionner) plutôt qu'à inkpdf.
6. **Rendu par lots et asynchrone** (webhooks chez APITemplate et Typsetter, ZIP chez typst-webservice) : hors périmètre explicite.
