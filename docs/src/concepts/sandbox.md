# Sandbox and determinism

A render runs untrusted input (the request body) through author-provided code (the template).
inkpdf constrains both: the caller can only send JSON, and the template can only see its own
files. Within those limits, the same input always gives the same PDF.

## The caller sends data, never code

- The API accepts JSON only. It takes no Typst code, no markup and no file path from the caller.
- The body is validated against the template's schema before Typst runs.
- `data` and `layout` reach Typst as values in `sys.inputs`, never by inserting text into a
  Typst source. A string such as `#import "/etc/passwd"` is displayed as is.
- Template identifiers are checked against `^[a-z0-9][a-z0-9_-]{0,63}$`, so a URL cannot
  traverse paths.

## What a template can access

During a render, Typst only sees what the service gives it:

| Resource | Available |
|----------|-----------|
| files of the template folder | yes, from the in-memory copy loaded with the template |
| bundled packages imported by name | yes, each package seeing only its own files |
| embedded fonts and the template's `fonts/` | yes |
| other files on disk (`../`, absolute paths, links pointing outside the folder) | no |
| other templates | no |
| network | no |
| environment variables | no |
| system fonts | no |

The render never reads the disk: all the template's files were read into memory when it was
loaded (see [Hot reload](hot-reload.md)). A path that leaves the template folder simply does not
exist from Typst's point of view, and the render fails with `500 render-failed`. Symbolic links
pointing outside the folder are excluded when the template is loaded.

Compilation errors are returned in the `diagnostics[]` array of the `500 render-failed` response,
each with its `message`, the `file` (relative to the template folder, or prefixed with the
package for an error inside a package), the `line`, the `column` and Typst's `hints`.

## Bounded renders

Every render is bounded so that a pathological template or input cannot block the service:

- request bodies are limited by `INKPDF_MAX_BODY_BYTES` (`413` beyond);
- at most `INKPDF_MAX_CONCURRENT_RENDERS` renders run at once; a request waits at most
  `INKPDF_QUEUE_TIMEOUT_SECS` for a slot (`503 overloaded` beyond);
- a render that exceeds `INKPDF_RENDER_TIMEOUT_SECS` gets `504 render-timeout`.

Typst offers no cancellation: an overrunning compilation is stopped at its next access to a file
or a font, and a purely computational loop (or a long WebAssembly plugin call) keeps its render
slot until it ends. The caller still gets its `504` on time. Details are in
[Limits and performance](../operations/limits.md).

## Determinism

Two renders of the same template version with the same body produce **byte-identical PDFs**:

- the PDF has no creation timestamp;
- its document identifier is derived from the template identifier and the template's
  fingerprint, not from random values;
- a date is written into the PDF properties only if the request's `metadata.date` or the
  template's `#set document(date: ...)` provides one;
- fonts come only from the binary and the template, never from the host.

This makes PDFs reproducible, cacheable and easy to compare in tests.

The one escape hatch is `datetime.today()`: it is available, but it makes the document depend on
the day it is rendered. Pass dates in `data` instead:

```json
{ "data": { "issuedOn": "2026-10-01" } }
```

```typst
#let issued = sys.inputs.data.issuedOn  // "2026-10-01", formatted as the template wants
```

## Not a security boundary for the network

The sandbox protects the host from templates and requests. It is not an access control: inkpdf
has no authentication and anyone who can reach it can render any template. Run it on a private
network; see [Security](../operations/security.md).
