# Security

inkpdf is designed for a narrow attack surface: callers send only JSON, which is validated
against a schema and handed to a sandboxed Typst engine as data. This page lists what the
service protects against and what it leaves to the deployment.

## Deploy on a private network

inkpdf has **no authentication, no authorization and no users**. Anyone who can reach the port
can list the templates, read their schemas and render documents. Run it on a private network
(a VPC, a Kubernetes cluster network, a Docker network) and expose it only to the services that
need it. If it must be reachable from a wider network, put it behind a gateway or reverse proxy
that handles authentication, TLS and rate limiting.

The service itself speaks plain HTTP; terminate TLS in front of it when needed.

## What callers can send

- Only JSON. The API accepts no Typst code, no markup that would be interpreted, and no file
  paths.
- The body is validated against the template's JSON Schema (and `metadata` against a fixed
  schema) **before** any compilation. An invalid body is rejected with
  `422 validation-failed`, listing every faulty path; nothing is compiled. Unless the
  schema says otherwise, properties not declared at the root of the body are refused.
- The values reach Typst as data, through `sys.inputs`, never by building Typst source from
  strings. A string in the request is a Typst string value, never parsed as markup, unless
  the template itself evaluates it: never call `eval` on request data.
- Template ids must match `^[a-z0-9][a-z0-9_-]{0,63}$`: they cannot contain `/` or `.`, so
  they cannot traverse paths.
- The `filename` query parameter of a download is cleaned: path separators, quotes, control
  characters and other unsafe characters are removed, as is a trailing `.pdf`, before it is
  placed in the `Content-Disposition` header (with an ASCII fallback and a UTF-8 `filename*`).
- Request sizes and render durations are bounded; see
  [Limits and performance](limits.md).

## The sandbox

Templates are trusted code written by your team, but the engine still runs them in a sandbox
(see [Sandbox and determinism](../concepts/sandbox.md)):

- **No file system access during rendering.** A template is read once, when it is loaded, into
  an in-memory snapshot. Rendering only sees that snapshot, the fonts embedded in the binary
  and the template's own fonts. System fonts are never loaded.
- **No files outside the template folder.** Typst paths cannot leave the template root.
  Symbolic links whose target is outside the template folder are excluded at load time, and
  logged as `template.link_excluded`. Hidden files are ignored.
- **No network.** Packages are never downloaded: only the packages bundled into the binary
  (verified by digest at build time) can be imported, and each package only sees its own
  files.
- **No environment variables or processes.** The engine is a library inside the binary; it
  starts no subprocess and no browser, and environment variables are not exposed to templates.

## The image

The container image is based on `gcr.io/distroless/cc-debian12:nonroot`: it runs as an
unprivileged user and contains no shell or package manager. Mount the templates volume
read-only. See [Deployment](deployment.md).

## Logs

Request bodies are never logged. Render logs carry only the template id, the duration and the
outcome.

## Reporting a vulnerability

Please do **not** open a public issue for a security problem. Report it privately through
GitHub's private vulnerability reporting: on the repository, open the **Security** tab and
choose **Report a vulnerability**. The full policy is in
[`SECURITY.md`](https://github.com/ck-developer/inkpdf/blob/main/SECURITY.md).
