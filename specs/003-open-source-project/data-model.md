# Data Model — Open-source project foundation (003)

Nothing is persisted. The entities below are built in memory or derived from repository files.

## Live OpenAPI document (service, in memory)

| Element | Source | Notes |
|---|---|---|
| Static base | `utoipa` document (generic routes, shared components) | Built once. Equal to `openapi/openapi.json` when no template is loaded. |
| Template operation | Each **valid** `TemplateEntry` | `POST /templates/{id}/render`, `operationId` `render_<id>`, summary = name, description = description + version |
| `<Prefix>RenderRequest` | Template schema root | `required` copied from the root; properties `data`, `layout` (if declared) and `metadata` (`$ref DocumentMetadata`) |
| `<Prefix>Data`, `<Prefix>Layout` | `properties.data`, `properties.layout` | Self-contained copies; internal `$ref`s rewritten |
| `<Prefix>_<def>` | Each `$defs` entry | Hoisted into components |
| Cache entry | `(Weak<TemplateMap>, Arc<Value>)` | Rebuilt when the registry snapshot identity changes |

**Invariants**
- Every `$ref` in the document resolves.
- Prefixes are unique.
- Invalid templates have no typed operation.
- The generic part is identical to the locked contract.

## Package lock entry (unchanged from 002)

The fields are `name`, `version`, `sha256`, `license` and `role` (`selected` | `dependency`).
The `xtask` command maintains the following invariants:
- at most one `selected` entry per name;
- every archive matches exactly one entry;
- the set is closed under the packages' literal imports.

## Universe index entry (read by `xtask`)

`https://packages.typst.org/preview/index.json` provides `name`, `version`, `compiler` (an
optional minimum Typst version), `license` and `description`. A version is a candidate if its
`compiler` is absent or not higher than the engine version.

## Release

| State | Trigger | Result |
|---|---|---|
| Unreleased commits on `main` | push | release PR opened or updated by release-please (version bump plus `CHANGELOG.md`) |
| Release PR merged | merge | tag `vX.Y.Z`, GitHub release with notes, images `X.Y.Z`, `X.Y` and `latest` |
| Any push to `main` with green checks | push | image `dev` republished |
