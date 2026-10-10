# Limits and performance

Every render is bounded, so that a pathological template or request cannot block the service.
All limits are set through [environment variables](../reference/configuration.md).

## Limits

| Limit | Variable | Default | When exceeded |
|-------|----------|---------|---------------|
| Request body size | `INKPDF_MAX_BODY_BYTES` | 5 MiB | `413 payload-too-large` |
| Render duration | `INKPDF_RENDER_TIMEOUT_SECS` | 30 s | `504 render-timeout` |
| Concurrent renders | `INKPDF_MAX_CONCURRENT_RENDERS` | number of CPUs | the request waits in the queue |
| Wait for a render slot | `INKPDF_QUEUE_TIMEOUT_SECS` | 10 s | `503 overloaded` |
| Template folder size | `INKPDF_MAX_TEMPLATE_BYTES` | 50 MiB | the template is marked invalid (`409 template-invalid` on render) |

Errors use the RFC 9457 format described in [Error codes](../reference/errors.md).

### Body size

The body limit is enforced while the body is read, before any JSON parsing or schema
validation. Raise it only if your documents genuinely need large inputs (for example, many
table rows); images and fonts belong in the template folder, not in the request.

### Concurrency and queue

Each render takes one of `INKPDF_MAX_CONCURRENT_RENDERS` slots for the whole compilation. A
request that finds every slot busy waits in the queue. If no slot frees up within
`INKPDF_QUEUE_TIMEOUT_SECS`, it is rejected with `503 overloaded`: clients may retry later,
ideally with backoff. Requests rejected before rendering (validation errors, unknown template)
never take a slot.

Compilation is CPU-bound: more slots than CPUs adds latency without adding throughput. To serve
more traffic, add replicas.

### Render timeout

The timeout covers the compilation and the PDF export, not the wait in the queue. When it
expires, the caller receives `504 render-timeout` at once, and the service asks the compilation
to stop.

## Known limitation: cancellation

Typst offers no way to cancel a compilation. inkpdf stops it at the template's next access to
a file or a font: from then on, every such access fails and the compilation ends early. A
purely computational loop that touches no file keeps its render slot until it ends on its own.
In every case:

- the caller still receives `504` within the timeout;
- the slot is released only when the compilation really ends, so other renders remain bounded
  by `INKPDF_MAX_CONCURRENT_RENDERS`;
- a compilation that ends after its deadline is reported by a `render.overrun` log event, with
  the template id and the real duration.

The same applies to the **WASM plugins** used by some bundled packages: a plugin call cannot be
interrupted before it returns. The packages that ship a plugin are flagged in
[Bundled packages](../reference/packages.md).

In a debug build (`cargo test`), abandoning a cancelled compilation shows up as a
`comemo: found differing return values` message on the render thread. It comes from `comemo`'s
purity assertion, which only exists in debug builds; the slot is released normally. In a
release build, the compilation simply ends with an error.

## Performance

Typical render times, release build, after the first render (which loads fonts and warms the
caches):

| Template | Time |
|----------|------|
| Simple document (title and a 20-row table) | ≈ 2 ms |
| Document with charts and QR codes (bundled packages) | ≈ 100 ms |

A compilation cache is shared between renders: the bundled packages are evaluated once and
reused, and only what depends on the request data is recomputed.

Debug builds are much slower; always measure with `--release`. CI guards
against regressions with `cargo test --release -p inkpdf --test perf -- --ignored`, which fails
if the p95 exceeds 200 ms for the simple template or 1 s for the template with packages (see
[Development and tests](../contributing/development.md)).
