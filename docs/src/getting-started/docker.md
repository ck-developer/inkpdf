# Run with Docker

The quickest way to try inkpdf is the published container image, with the example templates of
the repository mounted as the templates volume.

## The image

The image is published as `ghcr.io/ck-developer/inkpdf`:

| Tag | Content |
|-----|---------|
| `latest` | the latest released version |
| `X.Y.Z` (for example `1.2.0`) | a specific released version |
| `dev` | the current `main` branch, rebuilt on every merge |

Pin an `X.Y.Z` tag in production. The image contains only the binary (no shell, no package
manager); it listens on port `3000` and reads templates from `/templates`.

## Start the service

From a clone of the repository:

```bash
git clone https://github.com/ck-developer/inkpdf.git
cd inkpdf

docker run --rm -p 3000:3000 \
  -v "$PWD/examples/templates:/templates:ro" \
  ghcr.io/ck-developer/inkpdf:latest
```

The volume is mounted read-only: inkpdf never writes to it.

Check that the service is up and that the initial scan of the volume is done:

```bash
curl -s localhost:3000/health
curl -s localhost:3000/ready
```

Then list the templates it found:

```bash
curl -s localhost:3000/templates
```

Each template is reported with a `status` of `valid` or `invalid`; an invalid template carries a
`reason`.

## Render a first PDF

The `sample` template takes a title and a list of labels and values. Send the example request
from `examples/requests`:

```bash
curl -s -H 'Content-Type: application/json' \
  -d @examples/requests/sample.json \
  -o sample.pdf \
  localhost:3000/templates/sample/render
```

Open `sample.pdf`. The request only contains `data`: every `layout` parameter took its default
value from the template's schema. Change the appearance by adding a `layout` section:

```bash
curl -s -H 'Content-Type: application/json' \
  -d '{
        "data": { "title": "Hello", "items": [{ "label": "A", "value": 1 }] },
        "layout": { "primaryColor": "#8b0000", "align": "center", "showFooter": false }
      }' \
  -o hello.pdf \
  localhost:3000/templates/sample/render
```

By default the PDF is returned inline (`Content-Disposition: inline`). Add `?download=true` to
get an attachment, and `filename` to name it:

```bash
curl -s -OJ -H 'Content-Type: application/json' \
  -d @examples/requests/sample.json \
  'localhost:3000/templates/sample/render?download=true&filename=my-report'
```

## Explore the API

Open <http://localhost:3000/docs> in a browser. The page is generated from
`GET /openapi.json`, which the service builds live from the loaded templates: each valid
template has its own render operation, with the exact shape of its `data` and `layout`, and you
can send requests from the page. See [Exploring the API](../guides/exploring-the-api.md).

## Use your own templates

Mount any folder that contains template folders:

```bash
docker run --rm -p 3000:3000 \
  -v "$PWD/my-templates:/templates:ro" \
  ghcr.io/ck-developer/inkpdf:latest
```

Templates added, changed or removed in that folder are picked up within a few seconds, without
restarting the container (see [Hot reload](../concepts/hot-reload.md)).

## With Docker Compose

The repository's `compose.yaml` builds the image from source, which is handy to test local
changes:

```bash
docker compose up --build                                   # http://localhost:3000/docs
INKPDF_TEMPLATES=./my-templates docker compose up --build   # another templates folder
INKPDF_PORT=8080 docker compose up --build                  # another host port
```

It mounts `./examples/templates` by default and uses human-readable logs
(`INKPDF_LOG_FORMAT=pretty`).

## Configuration

The service is configured with `INKPDF_*` environment variables (body size, render timeout,
concurrency, log format…), passed with `-e`:

```bash
docker run --rm -p 3000:3000 \
  -e INKPDF_RENDER_TIMEOUT_SECS=10 \
  -e INKPDF_LOG_FORMAT=pretty \
  -v "$PWD/examples/templates:/templates:ro" \
  ghcr.io/ck-developer/inkpdf:latest
```

A malformed value prevents startup with an explicit message. All variables are listed in
[Configuration](../reference/configuration.md); health checks, image tags and orchestration
notes are in [Deployment](../operations/deployment.md).

Next: [write your first template](first-template.md).
