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
role = "selected"        # "selected" | "dependency"
```

- Les entrées sont triées par `name`, puis par `version`.
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

## Vérifications par les tests (`cargo test`)

- Chaque paquet s'importe.
- Chaque paquet `selected` s'utilise.
- L'ensemble est fermé : chaque `@preview/…` importé par un paquet est présent.
- `docs/packages.md` est synchrone avec le lock.

## Règles d'évolution

1. **Ajouter un paquet** : lancer `scripts/add-package.sh <name> <version>`, puis ajouter ses
   dépendances (`--dependency`) jusqu'à ce que le test de fermeture passe, puis une fixture
   d'usage et une ligne dans `docs/packages.md`.
2. **Mettre à jour un paquet** : **ajouter** la nouvelle version à côté de l'ancienne. Les
   templates qui importent l'ancienne continuent de fonctionner.
3. **Retirer une version** : c'est une rupture pour les templates qui l'importent, qui
   deviennent `invalid`. À faire seulement par décision explicite, notée dans la PR et le
   changelog.
4. **Un paquet qui échoue aux tests avec la version de Typst du service** est retiré, et non
   corrigé localement.
