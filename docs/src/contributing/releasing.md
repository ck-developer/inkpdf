# Releasing

Releases are automated with [release-please](https://github.com/googleapis/release-please).
Nobody tags or publishes by hand: the version, the changelog, the tag and the images all follow
from the commit history.

## Conventional commits

Commit messages (and pull request titles, which become the squash commit) follow
[Conventional Commits](https://www.conventionalcommits.org):

```text
<type>(<optional scope>): <summary>
```

| Type | Use | Version bump |
|------|-----|--------------|
| `feat` | A user-visible feature | minor |
| `fix` | A bug fix | patch |
| `perf` | A performance improvement | patch |
| `docs`, `refactor`, `test`, `build`, `ci`, `chore` | Everything else | none |

A breaking change is marked with `!` after the type (`feat!: …`) or a `BREAKING CHANGE:` footer.
Before 1.0, a breaking change bumps the **minor** version (`0.2.0` → `0.3.0`); from 1.0 on, it
bumps the major version.

Examples:

```text
feat(api): live, template-aware OpenAPI document and /docs page
fix(render): release the slot when a compilation panics
feat!: remove the codetastic package
```

Commit messages, like everything else in the repository, are written in English.

## The release flow

| Event | Result |
|-------|--------|
| Pull request | CI runs lint, tests, the performance test, `docs check` and `packages verify`. Nothing is published. |
| Push to `main`, all checks green | The `ghcr.io/ck-developer/inkpdf:dev` image is published for `linux/amd64` and `linux/arm64`. The documentation site is deployed to GitHub Pages. release-please creates or updates the release pull request. |
| Release pull request merged | The tag `vX.Y.Z` and a GitHub release with the changelog notes are created, and the images `X.Y.Z`, `X.Y` and `latest` are published for both architectures. |

### The release pull request

release-please keeps one open pull request, titled after the next version. It:

- computes the next version from the conventional commits since the last release;
- bumps `version` in `crates/inkpdf/Cargo.toml` and in `Cargo.lock` (cargo-workspace plugin);
- adds the new section to `crates/inkpdf/CHANGELOG.md`, grouped by type.

It is updated on every push to `main`. To release, review it and merge it.

### Republishing the images of a release

If the image publication of a release failed, run the **Release** workflow manually
(*Actions → Release → Run workflow*) with the existing tag, e.g. `v0.1.0`: it rebuilds and
pushes `X.Y.Z`, `X.Y` and `latest` from that tag.

### Tags and images

- Tags are `vX.Y.Z`, without a component prefix.
- `dev` always points to the latest green commit of `main`; there are no per-commit tags.
- `X.Y.Z` never moves; `X.Y` and `latest` move to the newest release.

See [Deployment](../operations/deployment.md#image-tags) for which tag to use.

The version reported by `GET /health` and in `info.version` of the OpenAPI document comes from
`CARGO_PKG_VERSION`, so a released image always reports `X.Y.Z`. The OpenAPI contract test
ignores `info.version`, so a release bump does not break it.

## `CHANGELOG.md`

The changelog lives in `crates/inkpdf/CHANGELOG.md` (release-please only writes inside the
package folder); the root `CHANGELOG.md` points to it. It is maintained by release-please: do
not edit released sections by hand. History from before automated releases (0.1.0) is summarised by hand in its
own section.
