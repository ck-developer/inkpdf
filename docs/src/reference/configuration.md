# Configuration

inkpdf is configured only through environment variables. Every variable is optional: an unset
or empty variable keeps its default. The values are read once, at startup; changing them needs
a restart.

## Variables

| Variable | Default | Meaning |
|----------|---------|---------|
| `INKPDF_TEMPLATES_DIR` | `/templates` | Folder that holds the templates, one sub-folder per template. In the Docker image it is the `/templates` volume. |
| `INKPDF_LISTEN` | `0.0.0.0:3000` | Address and port the HTTP server listens on (`host:port`). |
| `INKPDF_MAX_BODY_BYTES` | `5242880` (5 MiB) | Maximum size of a request body, in bytes. A larger body is rejected with `413 payload-too-large`. |
| `INKPDF_RENDER_TIMEOUT_SECS` | `30` | Maximum duration of a render, in seconds. Beyond it, the caller receives `504 render-timeout`. |
| `INKPDF_MAX_CONCURRENT_RENDERS` | number of CPUs | Number of renders that may run at the same time (render slots). |
| `INKPDF_QUEUE_TIMEOUT_SECS` | `10` | Maximum time, in seconds, a render request waits for a free slot. Beyond it, the caller receives `503 overloaded`. |
| `INKPDF_RESCAN_INTERVAL_SECS` | `2` | Interval, in seconds, of the periodic rescan of the templates folder. It backs up file system events, which some volumes (network file systems, Kubernetes ConfigMaps) do not deliver. |
| `INKPDF_MAX_TEMPLATE_BYTES` | `52428800` (50 MiB) | Maximum total size of a template folder, in bytes. A template is loaded in memory; a larger one is marked invalid. |
| `INKPDF_LOG_FORMAT` | `json` | Log format on stdout: `json` (one JSON object per line) or `pretty` (human-readable, for local use). |
| `INKPDF_DEFAULT_AUTHOR` | `inkpdf` | PDF author used when neither the request (`metadata.author`) nor the template (`set document(author: …)`) provides one. Surrounding whitespace is trimmed. |
| `RUST_LOG` | `info` | Log filter, in the [`tracing-subscriber` `EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) syntax, for example `warn` or `info,inkpdf=debug`. |

## Validation

- Numeric values must be positive integers: `0` is refused for every size, count and duration.
- `INKPDF_LISTEN` must be a socket address such as `127.0.0.1:8080` or `[::]:3000`.
- `INKPDF_LOG_FORMAT` accepts only `json` and `pretty`.

A malformed value prevents startup. The process prints an explicit message naming the variable
and exits with status `2`, for example:

```text
configuration error: invalid value for INKPDF_MAX_BODY_BYTES: `lots`: invalid digit found in string
```

## Examples

Running locally against the example templates, with readable logs:

```sh
INKPDF_TEMPLATES_DIR=examples/templates \
INKPDF_LOG_FORMAT=pretty \
cargo run --release -p inkpdf
```

A container with a shorter timeout and two render slots:

```sh
docker run --rm -p 3000:3000 \
  -v "$PWD/templates:/templates:ro" \
  -e INKPDF_RENDER_TIMEOUT_SECS=10 \
  -e INKPDF_MAX_CONCURRENT_RENDERS=2 \
  ghcr.io/ck-developer/inkpdf:latest
```

See [Limits and performance](../operations/limits.md) for how the limits interact, and
[Deployment](../operations/deployment.md) for the container setup.
