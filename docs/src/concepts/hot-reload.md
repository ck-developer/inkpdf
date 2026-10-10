# Hot reload

Templates live in a volume, not in the image. Adding, changing or removing a template never
requires rebuilding the image or restarting the service: copy a folder into the volume and it is
available within a few seconds.

## Discovery at startup

At startup, the service scans `INKPDF_TEMPLATES_DIR` (`/templates` in the image) and loads every
folder whose name is a valid template identifier. `GET /ready` answers successfully once this
initial scan is complete; `GET /health` only reports that the process is alive.

A missing or empty volume does not prevent startup: the service runs with no templates and logs
a warning.

## Detecting changes

Two mechanisms run side by side:

- **File system events**: the service watches the volume recursively. Events are grouped over
  500 ms before the volume is checked.
- **Periodic rescan**: every `INKPDF_RESCAN_INTERVAL_SECS` seconds (2 by default), the volume is
  checked anyway. This covers volumes that emit no events (network file systems, some container
  setups, Kubernetes ConfigMaps). If watching is unavailable, the periodic rescan is used alone.

Each check computes a **fingerprint** of every template folder: the relative path, size and
modification time of each file. No content is read for this; the fingerprint only tells whether
something changed, and gives the total size before anything is loaded.

## Loading a change safely

A template is reloaded only when its fingerprint has changed **and** stayed the same over two
observations about one second apart. A folder that is still being copied keeps changing, so it
is not loaded half-written. If files change while they are being read, the load is discarded and
retried later.

Meanwhile, the **previous version keeps being served**. Once a new version is stable, all its
files are read into memory and the registry switches to it atomically: a render in progress
finishes with the version it started with, and the next render uses the new one.

A folder that disappears from the volume is removed from the registry on the next check.

In practice:

```bash
cp -R my-templates/greeting /path/to/volume/
sleep 3
curl -s localhost:3000/templates/greeting
```

To replace a template, copy over the existing folder: the new version is served once the copy
has settled.

## Invalid templates

A template that fails to load (missing `main.typ`, malformed schema, wrong package import,
oversized folder…) is listed in `GET /templates` with `"status": "invalid"` and a `reason`, and is
logged. It does not affect the other templates. Fix the files in place and the template becomes
valid on the next check.

When a valid template is changed into an invalid one, it becomes invalid: the previous version is
not kept once the new one has been read.

## Effect on the API

Everything the API reports follows the registry:

- `GET /templates` and `GET /templates/{templateId}` show the current version (with its
  `loadedAt` time);
- `GET /openapi.json` and `GET /docs` are rebuilt on every request, so a new or changed template
  shows up there once it has been reloaded, and an invalid one disappears from them.

## Determinism and caching

Each loaded version is identified by its fingerprint, which is also part of the identifier
written into the PDF. Two renders of the same body give identical bytes as long as the template
does not change. See [Sandbox and determinism](sandbox.md).
