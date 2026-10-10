# Security policy

## Supported versions

| Version | Supported |
|---|---|
| Latest release (`latest`, `X.Y.Z`) | ✅ |
| `dev` image (main branch) | ✅ best effort |
| Older releases | ❌ — please upgrade |

## Reporting a vulnerability

Please **do not open a public issue**. Report it privately through
[GitHub private vulnerability reporting](https://github.com/ck-developer/inkpdf/security/advisories/new).

Include the version (or image tag), a description of the impact and, if possible, a minimal
template and request that reproduce the problem. You should get a first answer within a few
days; we will agree on a fix and a disclosure date with you.

## Scope

In scope, for example:

- escaping the rendering sandbox: reading files outside the template folder, network access,
  reading environment variables;
- path traversal through template identifiers, file names or package imports;
- code injection through request data (`data`, `layout`, `metadata`);
- resource exhaustion that bypasses the configured limits (body size, render timeout,
  concurrency).

## Deployment model

inkpdf has **no authentication** by design and is meant to run on a private network, behind
the services that call it. Exposing it directly to the internet is out of the supported
deployment model. See [Security](https://ck-developer.github.io/inkpdf/operations/security.html)
in the documentation.
