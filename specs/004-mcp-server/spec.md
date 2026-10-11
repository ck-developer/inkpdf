# Feature Specification: MCP server

**Feature Branch**: `004-mcp-server`

**Created**: 2026-10-11

**Status**: Draft

**Input**: User description: "Optional MCP (Model Context Protocol) server for inkpdf. Disabled by
default; enabled through an INKPDF_MCP_* setting; served on the same port as the HTTP API at /mcp
using the Streamable HTTP transport. It exposes three tools that reuse the existing engine (no
duplicated logic): list_templates (like GET /templates), get_template (returns the template's
JSON schema of expected data, like GET /templates/{id}/schema, so the assistant can ask the user
for missing fields), and render_template (renders the PDF like POST /templates/{id}/render and
returns the PDF directly in the tool result). Target usage: a user tells an AI assistant
"generate an employer certificate"; the assistant lists templates, reads the schema, asks the
user for the required fields, then renders and returns the PDF. Same limits as the HTTP API
(body size, concurrency, timeouts, sandbox, no network). No authentication at first, same as the
HTTP API (private network). Out of scope: creating or editing templates through MCP, storing
documents (S3 storage and a generate route with document IDs are a later feature),
authentication."

## Clarifications

### Session 2026-10-11

- Q: Who can reach the MCP server: a private network only, or also the Internet with an access
  key? → A: Private network only, no authentication, for now; access from the Internet with an
  access key comes in a later feature.

## User Scenarios & Testing *(mandatory)*

### Actors

- **End user**: talks to an AI assistant in plain language ("make me an employer certificate
  for Jane Doe") and expects a PDF back. Knows nothing about templates, schemas or the API.
- **AI assistant**: an MCP client (for example Claude) connected to inkpdf. It discovers the
  templates, reads what data they need, asks the end user for what is missing, and requests
  the document.
- **Operator**: deploys inkpdf, decides whether the MCP server is turned on, and connects the
  assistant to it.

### User Story 1 - Generate a document by asking an assistant (Priority: P1)

The end user asks the assistant for a document in their own words. The assistant finds the
matching template among those inkpdf offers, reads the data it needs, asks the end user only
for the fields that are required and not yet known, then produces the PDF and hands it back.

**Why this priority**: this is the whole point of the feature: producing a real document from a
conversation, with no API call written by hand.

**Independent Test**: with the MCP server turned on and the demonstration templates loaded,
connect an MCP client, ask it for one of those documents, answer its questions, and check that a
valid PDF comes back whose content matches the answers given.

**Acceptance Scenarios**:

1. **Given** the MCP server is on and an "employer certificate" template is loaded, **When** the
   assistant asks inkpdf for the list of templates, **Then** it receives every template with its
   identifier, name and description, enough to pick the right one from the end user's request.
2. **Given** the assistant picked a template, **When** it asks inkpdf for that template's
   details, **Then** it receives the full description of the expected data (business data and
   layout parameters), including which fields are required, their types, allowed values,
   descriptions and defaults.
3. **Given** the assistant collected the data, **When** it asks inkpdf to render the template
   with that data, **Then** it receives the PDF itself, with a suggested file name, in the same
   answer, and the PDF is identical to the one the HTTP API returns for the same input.
4. **Given** the same input, **When** the document is rendered once through the assistant and
   once through the HTTP API, **Then** both documents are the same.

---

### User Story 2 - Recover from incomplete or wrong data (Priority: P1)

When the data the assistant sends does not match the template (a required field is missing, a
value has the wrong type or is not among the allowed values), inkpdf tells the assistant
precisely what is wrong and where, so it can ask the end user again instead of failing.

**Why this priority**: assistants regularly guess or forget fields; without readable errors the
conversation dead-ends and the feature is unusable in practice.

**Independent Test**: ask for a render with a required field missing and another with a wrong
value; check that each answer is an error naming every faulty field and the reason, and that no
PDF is produced.

**Acceptance Scenarios**:

1. **Given** a template requiring a `employee.name` field, **When** the assistant asks for a
   render without it, **Then** inkpdf answers with an error that names `employee.name` as
   missing, and no rendering is attempted.
2. **Given** several faulty fields, **When** the assistant asks for a render, **Then** the error
   lists all of them at once, not only the first.
3. **Given** an unknown template identifier, **When** the assistant asks for its details or a
   render, **Then** inkpdf answers that the template does not exist and invites it to list the
   available templates.
4. **Given** a template that failed to load (invalid schema, missing file), **When** the
   assistant asks for a render, **Then** inkpdf answers that the template is unavailable, with
   the reason.

---

### User Story 3 - Turn the MCP server on and connect an assistant (Priority: P2)

The operator turns the MCP server on with one setting, without changing the image or the
templates, and connects an assistant to it by following the documentation.

**Why this priority**: required to use the feature at all, but simple and done once.

**Independent Test**: start inkpdf without the setting and check the MCP endpoint does not
answer; start it with the setting and connect an MCP client by following the documentation
only.

**Acceptance Scenarios**:

1. **Given** default settings, **When** inkpdf starts, **Then** the MCP server is off and its
   address answers "not found", exactly like any unknown path.
2. **Given** the setting turned on, **When** inkpdf starts, **Then** the MCP server is reachable
   on the same address and port as the HTTP API, and the startup log says it is on.
3. **Given** the MCP server is on, **When** the operator follows the documentation, **Then** they
   can connect at least Claude (desktop app or Claude Code) and see the three tools.
4. **Given** the MCP server is on, **When** a template is added to, changed in or removed from
   the volume, **Then** the next listing through the assistant reflects it, without a restart.

---

### Edge Cases

- A render is asked while all render slots are busy: the assistant gets the same "busy, try
  again" outcome as the HTTP API after the same waiting time, not an indefinite wait.
- A render exceeds the render time limit: the assistant gets a "took too long" error; the
  service keeps running.
- The data sent is larger than the request size limit: refused with an error saying so, before
  any validation or rendering.
- The produced PDF is large: it is still returned whole; the documentation states that some
  assistants limit the size of what they accept and that very large documents are better
  produced through the HTTP API.
- The assistant sends anything other than the expected data (Typst code, a file path, extra
  unknown keys at the top level): handled exactly like the HTTP API: data is only ever data,
  unknown top-level keys are refused.
- The template's layout parameters are omitted: their defaults apply, as with the HTTP API.
- A browser page tries to reach the MCP server from another site: refused (see FR-012).
- The MCP server is on but no template is loaded: the listing is empty and says so; nothing
  fails.

## Requirements *(mandatory)*

### Functional Requirements

**Activation**

- **FR-001**: The MCP server MUST be off by default and turned on by a single configuration
  setting, documented with the other settings.
- **FR-002**: When on, the MCP server MUST be served by the same process, on the same address
  and port as the HTTP API, at a fixed path (`/mcp`), using the standard MCP transport for
  remote servers over HTTP. When off, that path MUST answer exactly like an unknown path.
- **FR-003**: Turning the MCP server on or off MUST NOT change the behavior of the HTTP API in
  any way.

**Tools**

- **FR-004**: The MCP server MUST offer exactly three tools: list the templates, get one
  template's details, render one template. Each tool MUST carry a name, a plain-language
  description written for an assistant, and a description of its inputs.
- **FR-005**: The listing tool MUST return, for every template the HTTP API lists, its
  identifier, name, description, version and status (available or not, with the reason), and
  MUST reflect the current content of the template volume, as the HTTP API does.
- **FR-006**: The details tool MUST take a template identifier and return the template's
  summary and its full data description as published by the HTTP API (business data and layout
  parameters, with required fields, types, allowed values, descriptions and defaults), unaltered.
- **FR-007**: The render tool MUST take a template identifier, the business data, and the
  optional layout parameters and document metadata accepted by the HTTP API, and MUST return
  the PDF itself as a file (PDF media type, suggested file name) within the tool's answer,
  together with a short text summary (template, file name, size).
- **FR-008**: The render tool MUST produce the same document as the HTTP API for the same
  template and input: same validation, same defaults, same metadata rules, same engine.

**Errors**

- **FR-009**: Every failure the HTTP API can report for these operations (unknown template,
  unavailable template, invalid input with the faulty paths, input too large, busy, timeout,
  rendering failure) MUST be reported to the assistant as a tool error, in plain language, with
  the same information the HTTP API gives, including every faulty field path for invalid input.
- **FR-010**: Errors MUST distinguish what the assistant can fix (wrong or missing data, unknown
  template) from what it cannot (busy, timeout, internal failure), so it knows whether to ask the
  end user again or to retry later.

**Limits and security**

- **FR-011**: Renders started through the MCP server MUST share the HTTP API's limits and
  guarantees: request size, render concurrency and queue waiting time, render time, sandbox
  without network or file access outside the template, data injected only as data.
- **FR-012**: The MCP server MUST refuse requests coming from a web page of another site
  (cross-origin browser requests), as the MCP standard requires for HTTP servers, so that a
  malicious page cannot drive it through a user's browser.
- **FR-013**: The MCP server MUST have no authentication, like the HTTP API: it is meant to be
  reached from a private network only (an assistant on the same network or through a VPN). The
  documentation MUST state this and warn against exposing it on the Internet. Access from the
  Internet with an access key (for example a Claude connector configured on claude.ai) is a
  later, separate feature.

**Observability and documentation**

- **FR-014**: Each MCP tool call MUST be logged like an HTTP request: tool, template identifier
  when relevant, duration and outcome, and MUST be distinguishable from HTTP API calls in the
  logs.
- **FR-015**: The documentation site MUST gain an MCP page: what it is for, how to turn it on,
  how to connect Claude (desktop app and Claude Code) with a ready-to-copy configuration, the
  three tools, the limits, and a worked example of a conversation producing a document. The
  Docker Compose example and the settings reference MUST include the new setting.
- **FR-016**: The HTTP API's OpenAPI document MUST stay unchanged: the MCP server is described
  in the documentation, not as a REST operation.

### Key Entities

- **MCP tool**: one of the three operations offered to assistants (list templates, get template,
  render template); has a name, a description for the assistant, an input description and a
  result.
- **Template**: unchanged; the same templates, read from the same volume, as the HTTP API.
- **Rendered document**: the PDF returned by the render tool, with its suggested file name; it
  is not stored by inkpdf.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Starting from a request in plain language for one of the demonstration documents,
  an assistant connected to inkpdf produces the correct PDF in a single conversation, with no
  API call written by hand.
- **SC-002**: For 100% of the demonstration templates and their sample inputs, the PDF returned
  through the MCP server is identical to the one returned by the HTTP API.
- **SC-003**: For every invalid sample input, the error returned through the MCP server names
  100% of the faulty fields that the HTTP API names.
- **SC-004**: Rendering through the MCP server adds no noticeable delay compared with the HTTP
  API: generation time stays within 10% of the HTTP API's for the same template and input.
- **SC-005**: An operator turns the MCP server on and connects Claude in under 10 minutes using
  only the documentation.
- **SC-006**: With the MCP server off (the default), the service behaves exactly as before: all
  existing tests pass unchanged and the MCP path answers "not found".

## Assumptions

- The tools are fixed and generic (three tools); templates are not turned into one tool each.
  The assistant learns each template's fields from the details tool. This keeps the tool list
  stable while templates change in the volume.
- The PDF is returned directly in the tool's answer; inkpdf stores nothing. Storing documents
  with an identifier (S3) is a later, separate feature, which will add a tool to the MCP server.
- The tool names are `list_templates`, `get_template` and `render_template`.
- The suggested file name follows the HTTP API's default file name for the template.
- Only the remote HTTP transport is offered; a local, command-line transport is not, since inkpdf
  runs as a service in a container.
- The MCP server offers tools only: no prompts, no resources, no notifications that the template
  list changed (the assistant lists again when needed).
- Out of scope: creating or editing templates through MCP, document storage, batch generation,
  user management, authentication and access from the Internet (planned as a later feature).
- **Constitution**: principle VI states the public interface is a REST API. Adding an MCP
  interface that only exposes the same operations requires a MINOR amendment of principle VI
  (an optional MCP interface over the same engine and limits), made with this feature.
  Principles IV (JSON only, sandbox) and VII (no state, no database) are unchanged and hold for
  the MCP interface.
