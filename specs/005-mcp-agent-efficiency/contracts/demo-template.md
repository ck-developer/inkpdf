# Contract — The `demo` template

Folder: `examples/templates/demo/` (`template.json`, `schema.json`, `main.typ`, `example.json`).
Name "Demo document", version `1.0.0`, description "Neutral demonstration of every template
possibility: variable roles, kinds of variables, layout settings, metadata, a bundled package and
an example."

## `data`

| Property | Kind | Constraints | `x-role` | Description (gist) |
|---|---|---|---|---|
| `title` | string | 1–120 chars | ask | Document title. |
| `reference` | string | pattern `^[A-Z0-9-]{3,30}$` | fill | Document reference; build `DEMO-YYYY-NNN` if the user gives none. |
| `date` | string | `format: date` | fill | Document date, `YYYY-MM-DD`; today's date unless the user says otherwise. |
| `recipient` | object | closed | ask | Who the document is for. |
| `recipient.name` | string | 1–120 chars | ask | Full name. |
| `recipient.address` | string | ≤ 300 chars, line breaks allowed | ask | Postal address. |
| `recipient.email` | string | `format: email` | default | Optional e-mail address. |
| `lines` | array | 1–50 items | ask | Lines of the document. |
| `lines[].label` | string | 1–200 chars | ask | What the line is about. |
| `lines[].quantity` | integer | 1–9999 | ask | Number of units. |
| `lines[].amount` | string | pattern `^[0-9]{1,9}(\.[0-9]{1,2})?$` | ask | Unit amount in euros, dot as decimal separator. |
| `urgent` | boolean | default `false` | default | Shows an "Urgent" badge. |
| `status` | enum | `draft`, `final`; default `draft` | default | Shown in the header; `draft` adds a watermark. |
| `note` | string | ≤ 1000 chars | default | Optional free text at the end. |

## `layout`

| Property | Kind | Default | `x-role` |
|---|---|---|---|
| `accentColor` | string `^#[0-9a-fA-F]{6}$` | `#1f4e79` | default |
| `font` | enum `Libertinus Serif`, `New Computer Modern` | `Libertinus Serif` | default |
| `density` | enum `normal`, `compact` | `normal` | default |
| `showTotal` | boolean | `true` | default |

Every object has `additionalProperties: false`.

## Rendering

- Header: title, reference, date (long format, English), status, urgent badge.
- Recipient block; table of lines (label, quantity, unit amount, line amount) with a repeated
  header; total formatted with `@preview/zero` when `showTotal` is true; note.
- Page number "page x / n" in the footer.

## `example.json`

A complete body with three lines, `metadata.title` and `metadata.author`, using every `data`
property except `note`, and no `layout` (defaults apply). It renders unchanged (SC-003).
