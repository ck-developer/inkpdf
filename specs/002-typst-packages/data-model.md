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
| `role` | `selected` \| `dependency` | `selected` = l'un des paquets choisis ; `dependency` = ajouté pour fermer l'ensemble |

Unicité : le couple (`name`, `version`) est unique. Plusieurs versions d'un même nom sont
autorisées.

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
- `versions_of(name) -> Vec<PackageVersion>` ;
- `all() -> &[BundledPackage]`.

## PackageInfo (représentation API, `GET /packages`)

Projection publique de `BundledPackage` : `namespace`, `name`, `version`, `import` (valeur
`@preview/name:version`), `description`, `license`, `role`. Voir
[contracts/api.md](./contracts/api.md).

## PackageImport (dérivé au chargement d'un template, `src/template/imports.rs`)

| Champ | Type | Sens |
|---|---|---|
| `file` | chemin relatif au template | fichier `.typ` contenant l'import |
| `line` | entier, à partir de 1 | ligne de l'import |
| `raw` | chaîne | littéral tel qu'écrit, par exemple `@preview/zero:0.7.1` |
| `outcome` | `Bundled` \| `Invalid(spec_error)` \| `NotBundled { available: Vec<version> }` | résultat de la résolution |

**Effet sur le template.** Si au moins un import n'est pas `Bundled`, le template passe à
`TemplateStatus::Invalid { reason }`, où `reason` liste les imports fautifs (format dans
[contracts/template-imports.md](./contracts/template-imports.md)). Sinon, le statut ne change
pas.

## Transitions d'état d'un template (existantes, complétées)

```
fichiers déposés ──► chargement ──► valid
                         │
                         ├─(schéma, manifeste, taille… : V1)──► invalid
                         └─(NOUVEAU : import de paquet non intégré)──► invalid
invalid ──(correction des fichiers, rechargement à chaud)──► valid
```

L'ensemble des paquets ne change jamais à chaud : il ne change qu'avec le binaire.
