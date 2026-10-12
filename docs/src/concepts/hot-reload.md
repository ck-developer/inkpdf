# Loading templates

Templates live in a volume, not in the image. The service loads them when it starts and keeps
them unchanged while it runs: to publish a new template, or a new version of one, copy it into
the volume and restart the service. For development, hot reload picks up changes without a
restart.

## Loading at startup

At startup, the service scans `INKPDF_TEMPLATES_DIR` (`/templates` in the image) and loads every
folder whose name is a valid template identifier, with all its versions. `GET /ready` answers
successfully once this initial scan is complete; `GET /health` only reports that the process is
alive.

A missing or empty volume does not prevent startup: the service runs with no templates and logs
a warning.

All the files of each version are read into memory. From then on, by default, the service does
not look at the volume again: the template list, the renders and the OpenAPI document all use
what was loaded at startup, and a caller that asks for a version always gets exactly that
version. The `server.listening` log line says whether hot reload is on (`hotReload: false` by
default).

## Publishing a template or a version

1. Copy the template folder, or the new version folder, into the volume. To change a version
   that callers use, add a new version folder instead of editing it (see
   [Templates → Versions](templates.md#versions)).
2. Restart the service.
3. Check `GET /templates`: the template or version is listed, with its status.

With a single instance, the restart makes the service unavailable for a moment: requests get a
connection error until the server listens again. With several instances behind a load balancer,
restart them one at a time, each once the previous one answers `GET /ready`: this rolling
restart publishes without downtime. See [Deployment](../operations/deployment.md#publishing-templates).

An invalid template or version is listed in `GET /templates` with `"status": "invalid"` and a
`reason`, and logged. It does not affect the other templates or versions. Fix the files, then
restart again.

## Hot reload, for development

Set `INKPDF_HOT_RELOAD=true`, and the service watches the volume and picks up added, changed and
removed templates within a few seconds, without a restart. The repository's `compose.yaml`
turns it on for local development, so that you can edit a template and render it again at once.

Keep it off in production: with hot reload, a template can change while callers use it, so two
identical requests for the same version could give different documents.

### Detecting changes

Two mechanisms run side by side:

- **File system events**: the service watches the volume recursively. Events are grouped over
  500 ms before the volume is checked.
- **Periodic rescan**: every `INKPDF_RESCAN_INTERVAL_SECS` seconds (2 by default), the volume is
  checked anyway. This covers volumes that emit no events (network file systems, some container
  setups, Kubernetes ConfigMaps). If watching is unavailable, the periodic rescan is used alone.

Each check computes a **fingerprint** of every template folder, version by version: the relative
path, size and modification time of each file. No content is read for this; the fingerprint
only tells whether something changed, and gives the total size before anything is loaded.

### Loading a change safely

A version is reloaded only when its fingerprint has changed **and** stayed the same over two
observations about one second apart. A folder that is still being copied keeps changing, so it
is not loaded half-written. If files change while they are being read, the load is discarded and
retried later.

Meanwhile, the **previous state keeps being served**. Once a change is stable, all the files of
the version are read into memory and the registry switches to it atomically: a render in
progress finishes with the files it started with, and the next render uses the new ones.

A folder that disappears from the volume is removed from the registry on the next check.

In practice:

```bash
cp -R my-templates/greeting /path/to/volume/
sleep 3
curl -s localhost:3000/templates/greeting
```

When a valid template is changed into an invalid one, it becomes invalid: the previous files
are not kept once the new ones have been read. Fix the files in place and the template becomes
valid on the next check.

### Effect on the API

With hot reload, everything the API reports follows the registry:

- `GET /templates` and `GET /templates/{templateId}` show the current state (with its
  `loadedAt` time);
- `GET /openapi.json` and `GET /docs` are rebuilt on every request, so a new or changed template
  shows up there once it has been reloaded, and an invalid one disappears from them.

## Determinism and caching

Each loaded version is identified by its fingerprint, which is also part of the identifier
written into the PDF. Two renders of the same body give identical bytes as long as the template
does not change. See [Sandbox and determinism](sandbox.md).
