# Deployment

inkpdf ships as a single container image, `ghcr.io/ck-developer/inkpdf`, built for
`linux/amd64` and `linux/arm64`. Templates are not part of the image: they live in a volume
mounted at `/templates` and are loaded when the service starts.

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
volume is a template. The service loads them at startup; to publish a new template or version,
restart it (see [Publishing templates](#publishing-templates)).

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
      # One sub-folder per template; read-only is enough. Loaded at startup.
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
      # Hot reload of the templates folder: `true` for development only. Off, templates are
      # loaded at startup and a change needs a restart.
      INKPDF_HOT_RELOAD: "false"
      # Periodic rescan of the templates folder, in seconds, only with hot reload.
      INKPDF_RESCAN_INTERVAL_SECS: "2"
      # Maximum total size of a template version folder, in bytes. Default: 50 MiB.
      INKPDF_MAX_TEMPLATE_BYTES: "52428800"
      # Log format: `json` (one object per line) or `pretty` (human-readable).
      INKPDF_LOG_FORMAT: json
      # PDF author when neither the request nor the template sets one.
      INKPDF_DEFAULT_AUTHOR: inkpdf
      # MCP server for AI assistants at /mcp: `true` to turn it on. Private network only.
      INKPDF_MCP_ENABLED: "false"
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

The repository's own `compose.yaml` builds the image from source, mounts `examples/templates`
and turns hot reload on; it is meant for local development (`docker compose up --build`, then
`http://localhost:3000/docs`).

## Publishing templates

Templates are loaded once, at startup, so a running service never changes under its callers'
feet. To publish a new template, or a new version of a template:

1. copy the template folder, or the new version folder (for example `invoice/2.0.0/`), into the
   volume;
2. restart the service: `docker restart inkpdf`, `docker compose restart inkpdf`, or
   `kubectl rollout restart deployment/inkpdf`;
3. check `GET /templates`: it lists the new template or version, with its status.

Restarting a single instance makes the service unavailable for a moment. With several
instances behind a load balancer, a **rolling restart** avoids any downtime: restart them one
at a time, each once the previous one answers `GET /ready`. Kubernetes does this for
`kubectl rollout restart`, given a readiness probe on `/ready` and more than one replica (see
below).

To change a template that callers use, add a new version folder rather than editing the
existing one: callers that ask for the old version keep getting it, and the others get the new
one as the default. See [Templates → Versions](../concepts/templates.md#versions).

`INKPDF_MAX_TEMPLATE_BYTES` applies to each version folder, and every version stays loaded:
the memory the templates take grows with the number of versions in the volume. Remove the
versions nobody uses any more.

Hot reload (`INKPDF_HOT_RELOAD=true`) picks up changes without a restart, but is meant for
development only; see [Loading templates](../concepts/hot-reload.md).

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
- Size the memory for your templates: each template version is held in memory (each bounded
  by `INKPDF_MAX_TEMPLATE_BYTES`), plus one compilation per render slot.
- Templates are read at startup: after updating the volume or the ConfigMap, run
  `kubectl rollout restart deployment/inkpdf`. With two replicas or more and the readiness
  probe above, the restart causes no downtime.
- On `SIGTERM`, the server stops accepting connections and finishes the requests in flight, MCP
  tool calls included.

## Logs

Logs go to stdout, one JSON object per line by default (`INKPDF_LOG_FORMAT=json`); use
`pretty` for local reading. The level is set with `RUST_LOG` (default `info`). The MCP library
(`rmcp`) only logs errors, whatever `RUST_LOG` says: its other events can quote the input.

Each event carries an `event` field, for example:

| Event | When |
|-------|------|
| `server.listening`, `server.shutdown` | Start and stop of the HTTP server. `server.listening` says whether the MCP server is on (`mcp`) and whether hot reload is on (`hotReload`). |
| `mcp.listening` | The MCP server is on, with its `path` (`/mcp`). |
| `registry.ready` | End of the initial scan, with template counts and duration. |
| `template.loaded`, `template.invalid`, `template.removed` | Loading of a template version: at startup, and on a change with hot reload. |
| `template.version_ignored` | A sub-folder of a versioned template is not named `MAJOR.MINOR.PATCH`, and was skipped (`templateId`, `folder`). |
| `template.example_invalid` | A template's `example.json` failed its check and is not offered (`templateId`, `reason`); the template stays valid. |
| `template.link_excluded` | A symbolic link pointing outside a template folder was ignored. |
| `render` | One per render request, with `templateId`, `templateVersion` when the template has a version, `durationMs`, `outcome` (`ok`, `validation_failed`, `render_failed`, `timeout`, `overloaded`) and `interface` (`http` or `mcp`). |
| `mcp.tool` | One per MCP tool call, with `tool`, `templateId` and `templateVersion` when there is one, `durationMs` and `outcome` (`ok`, the error code, or `invalid-arguments` for a call refused for its arguments). See [Using inkpdf from an AI assistant](../guides/mcp.md). |
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
