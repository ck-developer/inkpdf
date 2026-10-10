# Data Model — Paquets Typst intégrés (002)

Aucune donnée persistée. Toutes les entités sont soit **statiques**, c'est-à-dire générées à la
construction et figées dans le binaire, soit **dérivées** au chargement d'un template.

## LockEntry (fichier `packages/lock.toml`, lu par `build.rs`)

| Champ | Type | Règles |
|---|---|---|
| `namespace` | chaîne | toujours `preview` |
| `name` | chaîne | identifiant Typst en kebab-case ; égal au `name` du `typst.toml` de l'archive |
| `version` | chaîne `MAJ.MIN.PATCH` | exacte ; égale à la `version` du `typst.toml` |
| `sha256` | 64 caractères hexadécimaux en minuscules | empreinte de `packages/vendor/<name>-<version>.tar.gz` |
| `license` | chaîne SPDX | recopiée de `typst.toml` (revue humaine) |
| `role` | `selected` \| `dependency` | `selected` = mis à disposition des templates (une seule version par nom) ; `dependency` = ajouté pour fermer l'ensemble |

Unicité : le couple (`name`, `version`) est unique, et il y a **au plus une** entrée `selected`
par `name`. Plusieurs versions d'un même nom ne coexistent que via des entrées `dependency`.

## BundledPackage (table statique générée, module `src/packages`)

| Champ | Type | Origine |
|---|---|---|
| `spec` | `PackageSpec` (`@preview/name:version`) | lock |
| `description` | chaîne | `typst.toml` → `[package].description` (vide si absent) |
| `license` | chaîne | lock |
| `role` | `Selected` \| `Dependency` | lock |
| `entrypoint` | chemin relatif | `typst.toml` → `[package].entrypoint` |
| `files` | table `chemin relatif → &'static [u8]` | contenu décompressé de l'archive |

**Invariants**, garantis par `build.rs` (R3) :
- l'entrypoint existe dans `files` ;
- aucun chemin n'est absolu ni ne contient `..` ;
- la table est triée par (`name`, `version`).

**Opérations** :
- `get(&PackageSpec) -> Option<&BundledPackage>` ;
- `selected(name) -> Option<&BundledPackage>` : la version mise à disposition des templates
  pour ce nom ;
- `all() -> &[BundledPackage]`.

## PackageInfo (représentation API, `GET /packages`)

Projection publique des paquets `selected` : `name`, `import` (`@preview/name`), `version`,
`description`, `license`. Voir [contracts/api.md](./contracts/api.md).

## TemplateImport (dérivé au chargement d'un template, `src/template/imports.rs`)

| Champ | Type | Sens |
|---|---|---|
| `file` | chemin relatif au template | fichier `.typ` contenant l'import |
| `line` | entier, à partir de 1 | ligne de l'import |
| `raw` | chaîne | littéral tel qu'écrit, par exemple `@preview/zero` |
| `outcome` | `Resolved(spec)` \| `VersionWritten` \| `OtherNamespace` \| `Unavailable` \| `Malformed` | résultat |

**Effets au chargement.**
- Si tous les imports sont `Resolved`, le texte des fichiers `.typ` de l'instantané est
  **réécrit une fois** : chaque `@preview/nom` devient `@preview/nom:<version installée>`.
- Sinon, le template passe à `TemplateStatus::Invalid { reason }`, avec une ligne par import
  fautif (format dans [contracts/template-imports.md](./contracts/template-imports.md)).

## Transitions d'état d'un template (existantes, complétées)

```
fichiers déposés ──► chargement ──► valid
                         │
                         ├─(schéma, manifeste, taille… : V1)──► invalid
                         └─(NOUVEAU : import incorrect : version écrite, paquet indisponible…)──► invalid
invalid ──(correction des fichiers, rechargement à chaud)──► valid
```

L'ensemble des paquets ne change jamais à chaud : il ne change qu'avec le binaire.
