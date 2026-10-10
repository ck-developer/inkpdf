# Contract — `cargo xtask packages`

Run from the repository root. The command needs network access only for `add` and `update`, to
reach `packages.typst.org`.

| Command | Effect |
|---|---|
| `cargo xtask packages add <name> [<version>]` | Adds `<name>` as `selected`. Without a version, it uses the latest version compatible with the engine. If the name already has a `selected` entry, the command fails with a hint to use `update`. The package's literal `@preview` imports are added recursively as `dependency` entries. |
| `cargo xtask packages update <name> [<version>]` | Replaces the `selected` version, by default with the latest compatible one. It adds the new dependencies and removes entries and archives that no package needs anymore. |
| `cargo xtask packages remove <name>` | Removes the `selected` entry and any dependency that becomes orphaned. It prints a warning that templates importing it will become invalid. |
| `cargo xtask packages verify` | Checks digests, the one-to-one match between archives and entries, at most one `selected` per name, the import closure and the generated docs. Exit code 1 lists every problem. |
| `cargo xtask packages list` | Prints a table of name, version, role, license and WASM. |
| `cargo xtask docs generate` / `check` | Regenerates the generated reference pages, or checks that they are up to date. |

## Guarantees

- `add`, `update` and `remove` are atomic: either every file (`packages/lock.toml`,
  `packages/vendor/*`, `docs/src/reference/packages.md`) is updated and `verify` passes, or
  nothing changes.
- Error messages name the package and the version: download failure, unknown package, no
  compatible version, digest mismatch.
- `scripts/add-package.sh` is removed.
