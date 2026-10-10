# Contract — Images, releases, documentation publication

| Event | Result |
|---|---|
| PR | CI runs lint, test, perf, docs check and packages verify. Nothing is published. |
| Push to `main`, all checks green | `ghcr.io/ck-developer/inkpdf:dev` for linux/amd64 and linux/arm64. The docs site is deployed to GitHub Pages. The release PR is created or updated. |
| Release PR merged | Tag `vX.Y.Z`, a GitHub release with changelog notes, and images `X.Y.Z`, `X.Y` and `latest` for both architectures |

- No `sha-*` tags.
- `dev` always points to the latest green commit of `main`.
- The version reported by `/health` and the OpenAPI `info.version` of a released image equals
  `X.Y.Z`.
