# Contract — Template versions and loading

## Volume layout

```text
templates/
├── facture/              # versioned layout
│   ├── 1.0.0/            # main.typ, schema.json, template.json?, example.json?, assets…
│   ├── 1.1.0/
│   └── 2.0.0/
└── attestation/          # flat layout (as before): one version
    ├── main.typ
    └── schema.json
```

| Situation | Result |
|---|---|
| Flat layout | One version: `template.json`'s `version`, or none |
| Versioned layout | One version per `MAJOR.MINOR.PATCH` sub-folder, each loaded and checked on its own |
| Template files at the root and version sub-folders | Template invalid: `put the template files either at the root or in version folders, not both` |
| Sub-folder that is not a version (`draft/`) | Skipped; log `{"event":"template.version_ignored","templateId":"facture","folder":"draft"}` |
| `template.json` version ≠ folder name | That version invalid: `template.json: version "1.2.0" does not match its folder "1.3.0"` |

## Selecting a version

`version` (optional), as a query parameter (HTTP) or an argument (MCP):

| Value | Version used |
|---|---|
| absent | highest valid version |
| `1` | highest valid `1.x.y` |
| `1.0.0` | exactly `1.0.0` (an invalid one gives `template-invalid`) |
| `latest`, `v2`, `1.2`, `` | `invalid-parameter` (400) on `version` |
| no match | `template-version-not-found` (404) |

`template-version-not-found` problem:

```json
{ "type": "https://github.com/ck-developer/inkpdf/errors/template-version-not-found",
  "code": "template-version-not-found", "title": "Template version not found", "status": 404,
  "detail": "template facture has no version matching \"3\"; available: 2.0.0, 1.1.0, 1.0.0",
  "templateId": "facture", "availableVersions": ["2.0.0", "1.1.0", "1.0.0"] }
```

## HTTP

- `GET /templates/{templateId}?version=…`, `GET /templates/{templateId}/schema?version=…`,
  `POST /templates/{templateId}/render?version=…`.
- Render responses carry `Inkpdf-Template-Version: 2.0.0` when the template has a version.
- `GET /templates`: each summary has `version` (default version) and `versions`:

  ```json
  { "id": "facture", "name": "Facture", "version": "1.1.0", "status": "valid",
    "versions": [ { "version": "2.0.0", "status": "invalid", "reason": "schema.json: …" },
                  { "version": "1.1.0", "status": "valid" },
                  { "version": "1.0.0", "status": "valid" } ] }
  ```

- `GET /templates/{id}`: the detail of the selected version, including its `version`.

## MCP

- `get_template` and `render_template`: new optional `version` string argument.
- Render text: `Rendered facture.pdf from template facture 2.0.0 (48.2 KiB).`
- `template-version-not-found` tool error hint: `Call get_template without a version to see the
  available versions.`

## Logs

- `render` and `mcp.tool` events: new `templateVersion` field.

## Loading

| `INKPDF_HOT_RELOAD` | Behaviour |
|---|---|
| `false` (default) | Templates loaded once at startup; no watcher, no rescan; changes need a restart |
| `true` | Watcher and periodic rescan every `INKPDF_RESCAN_INTERVAL_SECS`, as before 005 |

- `server.listening` log: new `hotReload` field.
- `compose.yaml` (development): `INKPDF_HOT_RELOAD: "${INKPDF_HOT_RELOAD:-true}"`.
