# Contrat — `packages/lock.toml` et `packages/vendor/`

Ce contrat s'adresse aux **mainteneurs** d'inkpdf. Le fichier `packages/lock.toml` est la
seule source de vérité de l'ensemble des paquets intégrés.

## Format

```toml
# Paquets Typst intégrés à inkpdf. Modifier via scripts/add-package.sh.

[[package]]
namespace = "preview"
name = "zero"
version = "0.7.1"
sha256 = "<64 caractères hexadécimaux>"
license = "MIT"
role = "selected"        # "selected" (mis à disposition des templates) | "dependency"
```

- Les entrées sont triées par `name`, puis par `version`.
- **Une seule entrée `selected` par nom** : c'est la version que les templates obtiennent avec
  `#import "@preview/<name>"`. D'autres versions du même nom peuvent exister, avec le rôle
  `dependency`, si un paquet les importe.
- Chaque entrée correspond exactement à une archive `packages/vendor/<name>-<version>.tar.gz`,
  téléchargée telle quelle depuis `https://packages.typst.org/preview/<name>-<version>.tar.gz`.

## Vérifications à la construction (`build.rs`)

La compilation échoue, en nommant le paquet, dans les cas suivants :

| Cas | Message (indicatif) |
|---|---|
| Archive absente | `packages/vendor/zero-0.7.1.tar.gz missing` |
| Empreinte différente | `zero 0.7.1: sha256 mismatch (expected …, got …)` |
| Archive illisible | `zero 0.7.1: unreadable archive` |
| `typst.toml` absent ou nom/version différents | `zero 0.7.1: manifest mismatch` |
| Entrypoint absent | `zero 0.7.1: entrypoint lib.typ not found` |
| Chemin absolu ou `..` dans l'archive | `zero 0.7.1: unsafe path …` |
| Archive sans entrée dans le lock | `packages/vendor/foo-1.0.0.tar.gz not listed in lock.toml` |
| Deux entrées `selected` pour un même nom | `zero: several selected versions (0.6.1, 0.7.1)` |

## Vérifications par les tests (`cargo test`)

- Chaque paquet s'importe.
- Chaque paquet `selected` s'utilise.
- L'ensemble est fermé : chaque `@preview/…` importé par un paquet est présent.
- `docs/packages.md` est synchrone avec le lock.

## Règles d'évolution

1. **Ajouter un paquet** : lancer `scripts/add-package.sh <name> <version>`, qui crée une entrée
   `selected`. Ajouter ensuite ses dépendances (`--dependency`) jusqu'à ce que le test de
   fermeture passe, puis une fixture d'usage et une ligne dans `docs/packages.md`.
2. **Changer la version d'un paquet** : **remplacer** son entrée `selected` (nouvelle archive,
   nouvelle empreinte). Tous les templates utilisent la nouvelle version à la prochaine version
   du service. Vérifier les templates d'exemple ; noter le changement dans la PR et le changelog.
   L'ancienne archive n'est conservée que si un autre paquet l'importe (`dependency`).
3. **Retirer un paquet** : c'est une rupture pour les templates qui l'importent, qui deviennent
   `invalid`. À faire seulement par décision explicite.
4. **Un paquet qui échoue aux tests avec la version de Typst du service** est retiré, et non
   corrigé localement.
