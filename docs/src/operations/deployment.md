# Deployment

inkpdf ships as a single container image, `ghcr.io/ck-developer/inkpdf`, built for
`linux/amd64` and `linux/arm64`. Templates are not part of the image: they live in a volume
mounted at `/templates` and are picked up live, without a restart.

## Image tags

| Tag | Content |
|-----|---------|
| `X.Y.Z` (for example `0.1.0`) | An exact release. Never moves. Recommended for production. |
| `X.Y` (for example `0.1`) | The latest patch release of that minor version. |
| `latest` | The latest release. |
| `dev` | The latest commit of `main` whose checks all passed. For testing unreleased changes only. |

There are no per-commit tags. Releases are described in [Releasing](../contributing/releasing.md).
The version of a running service is reported by `GET /health` and in `info.version` of
`GET /openapi.json`.

## Running with Docker

```sh
docker run -d --name inkpdf \
  -p 3000:3000 \
  -v /srv/inkpdf/templates:/templates:ro \
  ghcr.io/ck-developer/inkpdf:0.1.0
```

Mount the volume **read-only** (`:ro`): the service never writes to it. Each sub-folder of the
volume is a template; adding, changing or removing one takes effect within a few seconds (see
[Hot reload](../concepts/hot-reload.md)).

Every setting is an environment variable; see [Configuration](../reference/configuration.md).

## Docker Compose

A minimal service:

```yaml
services:
  inkpdf:
    image: ghcr.io/ck-developer/inkpdf:0.1.0
    ports:
      - "3000:3000"
    volumes:
      - ./templates:/templates:ro
    restart: unless-stopped
```

### Complete example

Every setting, with its default value. Uncomment and change only what you need; see
[Configuration](../reference/configuration.md) for the details of each variable.

```yaml
services:
  inkpdf:
    # `X.Y.Z` (recommended in production), `X.Y`, `latest`, or `dev` (latest main).
    image: ghcr.io/ck-developer/inkpdf:0.1.0
    ports:
      - "3000:3000"                 # host:container; the container listens on 3000
    volumes:
      # One sub-folder per template; read-only is enough. Changes are picked up live.
      - ./templates:/templates:ro
    environment:
      # Where the templates are, inside the container (the volume above).
      INKPDF_TEMPLATES_DIR: /templates
      # Address and port the server listens on, inside the container.
      INKPDF_LISTEN: 0.0.0.0:3000
      # Maximum request body size, in bytes (413 beyond). Default: 5 MiB.
      INKPDF_MAX_BODY_BYTES: "5242880"
      # Maximum duration of a render, in seconds (504 beyond).
      INKPDF_RENDER_TIMEOUT_SECS: "30"
      # Renders running at the same time. Default: number of CPUs.
      # INKPDF_MAX_CONCURRENT_RENDERS: "4"
      # Maximum wait for a free render slot, in seconds (503 beyond).
      INKPDF_QUEUE_TIMEOUT_SECS: "10"
      # Periodic rescan of the templates folder, in seconds (backs up file events).
      INKPDF_RESCAN_INTERVAL_SECS: "2"
      # Maximum total size of a template folder, in bytes. Default: 50 MiB.
      INKPDF_MAX_TEMPLATE_BYTES: "52428800"
      # Log format: `json` (one object per line) or `pretty` (human-readable).
      INKPDF_LOG_FORMAT: json
      # PDF author when neither the request nor the template sets one.
      INKPDF_DEFAULT_AUTHOR: inkpdf
      # Log filter (tracing EnvFilter syntax), e.g. `warn` or `info,inkpdf=debug`.
      RUST_LOG: info
    # The image already declares this health check; shown here to tune it.
    healthcheck:
      test: ["CMD", "/usr/local/bin/inkpdf", "healthcheck"]
      interval: 10s
      timeout: 3s
      start_period: 5s
      retries: 3
    # The image runs as a non-root user and needs no write access.
    read_only: true
    restart: unless-stopped
```

The repository's own `compose.yaml` builds the image from source and mounts
`examples/templates`; it is meant for local testing (`docker compose up --build`, then
`http://localhost:3000/docs`).

## Health and readiness

| Route | Meaning | Status |
|-------|---------|--------|
| `GET /health` | The process responds (liveness). | Always `200` with `{"status": "ok", "templates": <valid count>, "version": "<X.Y.Z>"}`. |
| `GET /ready` | The initial scan of the volume is finished (readiness). | `503` with `"status": "starting"` during the initial scan, then `200`. |

The server starts listening only after the initial scan, so `/ready` mostly matters to
orchestrators that probe early. An invalid template never makes the service unready: it is
reported in `GET /templates` and in the logs.

The image defines a Docker `HEALTHCHECK`. The image contains neither a shell nor `curl`, so the
check runs the binary itself: `inkpdf healthcheck` queries `/health` on the configured port and
exits with `0` or `1`.

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: inkpdf
spec:
  replicas: 2
  selector:
    matchLabels: { app: inkpdf }
  template:
    metadata:
      labels: { app: inkpdf }
    spec:
      containers:
        - name: inkpdf
          image: ghcr.io/ck-developer/inkpdf:0.1.0
          ports:
            - containerPort: 3000
          env:
            - name: INKPDF_MAX_CONCURRENT_RENDERS
              value: "2"
          resources:
            requests: { cpu: "1", memory: 256Mi }
            limits: { cpu: "2", memory: 1Gi }
          volumeMounts:
            - name: templates
              mountPath: /templates
              readOnly: true
          livenessProbe:
            httpGet: { path: /health, port: 3000 }
            periodSeconds: 10
          readinessProbe:
            httpGet: { path: /ready, port: 3000 }
            periodSeconds: 5
      volumes:
        - name: templates
          persistentVolumeClaim:
            claimName: inkpdf-templates
            readOnly: true
```

Notes:

- The number of CPUs seen by the process may be the node's, not the container's limit. Set
  `INKPDF_MAX_CONCURRENT_RENDERS` to match the CPU limit.
- Size the memory for your templates: each template is held in memory (bounded by
  `INKPDF_MAX_TEMPLATE_BYTES`), plus one compilation per render slot.
- ConfigMaps and network volumes may not deliver file system events. The periodic rescan
  (`INKPDF_RESCAN_INTERVAL_SECS`) picks up their changes anyway.
- On `SIGTERM`, the server stops accepting connections and finishes the requests in flight.

## Logs

Logs go to stdout, one JSON object per line by default (`INKPDF_LOG_FORMAT=json`); use
`pretty` for local reading. The level is set with `RUST_LOG` (default `info`).

Each event carries an `event` field, for example:

| Event | When |
|-------|------|
| `server.listening`, `server.shutdown` | Start and stop of the HTTP server. |
| `registry.ready` | End of the initial scan, with template counts and duration. |
| `template.loaded`, `template.invalid`, `template.removed` | Hot reload of a template. |
| `template.link_excluded` | A symbolic link pointing outside a template folder was ignored. |
| `render` | One per render request, with `templateId`, `durationMs` and `outcome` (`ok`, `validation_failed`, `render_failed`, `timeout`, `overloaded`). |
| `render.overrun` | A compilation finished after its deadline (see [Limits](limits.md)). |

Request bodies are never logged.

## The image

- Multi-stage build: the binary is compiled in a Rust image, then copied into
  `gcr.io/distroless/cc-debian12:nonroot`.
- It runs as the unprivileged `nonroot` user, and contains no shell, package manager or
  `curl`.
- It exposes port `3000` and declares the `/templates` volume
  (`INKPDF_TEMPLATES_DIR=/templates`).
- Fonts and bundled Typst packages are embedded in the binary: the container needs no network
  access at runtime.
