# Adding or updating a package

inkpdf bundles a fixed set of Typst packages from [Typst Universe](https://typst.app/universe)
into its binary. Templates import them by name only (`#import "@preview/zero"`); nothing is
downloaded at runtime. This page is for maintainers who change that set. For template authors,
see [Bundled packages](../concepts/packages.md) and the
[list of bundled packages](../reference/packages.md).

## How the set is defined

- `packages/lock.toml` is the single source of truth. Each `[[package]]` entry has a
  `namespace` (always `preview`), `name`, `version`, `sha256` digest, `license` and `role`.
  Entries are sorted by name, then version.
- `packages/vendor/<name>-<version>.tar.gz` holds each archive, exactly as downloaded from
  `https://packages.typst.org/preview/<name>-<version>.tar.gz`. Every archive has exactly one
  entry, and every entry one archive.
- The `role` is either:
  - `selected`: offered to templates. There is at most one `selected` entry per name; it is
    the version a template gets with `#import "@preview/<name>"`.
  - `dependency`: present only because another bundled package imports it. Templates cannot
    import it directly. Several versions of the same name may exist as dependencies.
- The set must be **closed**: every literal `@preview` import found in a bundled package must
  resolve to a bundled entry.

At build time, `crates/inkpdf/build.rs` checks every digest and manifest and embeds the files.
A missing, tampered or unlisted archive, or two `selected` versions of one name, makes
`cargo build` fail, naming the package.

Do not edit these files by hand: use `cargo xtask packages`.

## `cargo xtask packages`

Run the commands from the repository root. Only `add` and `update` need network access, to
reach `packages.typst.org`.

| Command | Effect |
|---------|--------|
| `cargo xtask packages add <name> [<version>]` | Adds `<name>` as `selected`. Without a version, uses the latest version compatible with the embedded Typst engine. The package's literal `@preview` imports are added recursively as `dependency` entries. Fails, with a hint to use `update`, if the name already has a `selected` entry. |
| `cargo xtask packages update <name> [<version>]` | Replaces the `selected` version, by default with the latest compatible one. Adds the new dependencies and removes the entries and archives no package needs anymore. |
| `cargo xtask packages remove <name>` | Removes the `selected` entry and any dependency left orphaned. Prints a warning: templates that import the package become invalid. |
| `cargo xtask packages verify` | Checks the digests, the one-to-one match between archives and entries, at most one `selected` per name, the import closure and the generated docs. Exits with code `1` and lists every problem. |
| `cargo xtask packages list` | Prints a table of name, version, role, license and whether the package contains a WASM plugin. |

`add` and `update` accept `--dry-run`: they print the resolved version and the dependencies
that would be added or removed, and change nothing.

`add`, `update` and `remove` are atomic: either every file (`packages/lock.toml`,
`packages/vendor/*` and the generated `docs/src/reference/packages.md`) is updated and `verify`
passes, or nothing changes. Error messages name the package and version: download failure,
unknown package, no compatible version, digest mismatch.

## Adding a package

1. Preview the change:

   ```sh
   cargo xtask packages add rowmantic --dry-run
   ```

2. Apply it:

   ```sh
   cargo xtask packages add rowmantic
   ```

3. Write a smoke template, `crates/inkpdf/tests/fixtures/package-smoke/<name>.typ`, that
   imports the package by name and uses its main feature. The `bundled_packages` test renders
   one for every `selected` package and fails if one is missing.
4. Run the tests:

   ```sh
   cargo test -p inkpdf --test bundled_packages
   cargo test --workspace
   ```

5. Check the license: it must allow redistribution inside the binary. Note in the pull request
   whether the package ships a WASM plugin (`cargo xtask packages list`): plugin calls cannot
   be interrupted by the render timeout (see
   [Limits and performance](../operations/limits.md#known-limitation-cancellation)).

## Updating a package

```sh
cargo xtask packages update zero            # latest compatible version
cargo xtask packages update zero 0.7.1      # a given version
```

Every template moves to the new version on the next deployment. Check the package's changelog
for breaking changes, run the tests (including `examples.rs`, which renders every example), and
mention the update in the pull request.

## Removing a package

```sh
cargo xtask packages remove <name>
```

Removal is a breaking change for templates that import the package: they become invalid on the
next deployment. Remove a package only by explicit decision, and use a `feat!:` or
`BREAKING CHANGE:` commit (see [Releasing](releasing.md)).

A package that does not work with the service's Typst version is removed, never patched
locally.

## Generated documentation

The [Bundled packages](../reference/packages.md) reference page is generated from the lock
file. The `packages` commands regenerate it; you can also run the documentation commands
directly:

```sh
cargo xtask docs generate   # regenerates every generated reference page
cargo xtask docs check      # fails if a generated page is stale
```

CI runs `cargo xtask packages verify` and `cargo xtask docs check` on every pull request.
