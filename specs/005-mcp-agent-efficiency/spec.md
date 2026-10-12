# Feature Specification: Templates that guide AI assistants, with versions

**Feature Branch**: `005-mcp-agent-efficiency`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "MCP efficiency for AI assistants (builds on feature 004, the MCP
server). Goal: an assistant such as Claude produces the right document in as few tool calls and
as little exchanged text as possible, without guessing and without exploring local files." Re-scoped
the same day by the user after reading the first draft: "Remove the invoice: it is a bad example.
Make one simple template with all the possibilities, and stop there; everyone will develop their
own template. The recommendation is that the schema defines the role of each variable, so that
the agent understands what the variable is, and whether it fills it in itself or asks the
question. In short: the MCP server, plus generic skills for this MCP server." Extended the same day:
"Remove hot reload, keep it for development mode only. Templates have a version that is not used:
at generation, I want to say which version to use, for example version 1 or version 2."

## Clarifications

### Session 2026-10-12

- Q: Which example templates does the repository keep? → A: One demonstration template that shows
  every possibility. `sample`, `packages-demo` and `progress-invoice` are removed, with everything
  that only exists for them.
- Q: Which roles can a variable have? → A: Three: **ask** (the assistant asks the end user),
  **fill** (the assistant fills it in itself, for example today's date), **default** (the
  assistant leaves the default value unless the end user mentions it).
- Q: Which generic skills? → A: Two: "generate a document with inkpdf" for end users, and "write an
  inkpdf template" for template authors.
- Q: What else stays in this feature? → A: A ready-to-fill example returned with each template's
  details, and impossible calendar dates refused at validation. A separate check-without-rendering
  tool and a render summary are dropped: rendering already validates the input and returns the
  errors.
- Q: What about the payment QR code limits of the invoice template? → A: Moot: the invoice
  template is removed.
- Q: Template versions: how are they laid out, and which is used? → A: One sub-folder per version,
  named after its version number (`facture/1.0.0/`, `facture/2.0.0/`); a template laid out as
  today is a single version. Without a version, the most recent valid version is used; a caller
  can ask for an exact version (`1.2.0`) or the most recent of a major version (`1`). Part of this
  feature.
- Q: Hot reload? → A: Development only, off by default. In production, templates are loaded at
  startup and a new template or version is published by restarting the service (constitution
  2.0.0, principle V).
- Q: How are faults in the tool arguments themselves reported (unknown key, missing `templateId`
  or `data`)? → A: As tool errors the assistant can see and fix, not as protocol errors that some
  clients hide from the model (decided after the analysis of this feature; audit defect 34).

## User Scenarios & Testing *(mandatory)*

### Actors

- **End user**: asks an AI assistant for a document in plain language and answers its questions.
- **AI assistant**: an MCP client (for example Claude) connected to inkpdf, with or without the
  generic skills installed.
- **Template author**: writes templates for their own documents; inkpdf ships no business
  templates, only one demonstration.
- **Maintainer**: keeps the repository's example, documentation and tests consistent.

### User Story 1 - The assistant knows what each variable is and who provides it (Priority: P1)

Every variable of a template's data says what it is (its description) and who provides it (its
role). The assistant asks the end user only about the variables marked **ask**, fills in by
itself those marked **fill**, and leaves those marked **default** at their default value unless
the end user brings them up. It produces the document without unnecessary questions and without
inventing what it should have asked.

**Why this priority**: this is the user's main recommendation and the root cause found by the
audit (defect 07): without descriptions and roles, the assistant guesses, asks too much or
invents values.

**Independent Test**: read the demonstration template's details and check that every variable has
a description and a role; connect an assistant, ask for the demonstration document, and check
that it asks exactly about the **ask** variables.

**Acceptance Scenarios**:

1. **Given** the demonstration template, **When** the assistant reads its details, **Then** every
   variable of its data and layout has a description and one of the roles **ask**, **fill** or
   **default**.
2. **Given** a template whose variables carry roles, **When** the assistant prepares a document,
   **Then** it asks the end user about every **ask** variable it does not already know, about no
   **fill** variable, and about no **default** variable the end user has not mentioned.
3. **Given** a template whose variables carry no role, **When** the assistant reads it, **Then**
   it still works: a required variable without a role is treated as **ask**, an optional one with
   a default as **default**, and the server instructions say so.
4. **Given** a template whose schema gives a variable a role other than the three allowed ones,
   **When** the template is loaded, **Then** it is reported as invalid with a reason naming the
   variable and the role, like any other schema error, and the other templates stay available.

---

### User Story 2 - A generic skill to generate documents (Priority: P1)

An end user installs the "generate a document with inkpdf" skill in their assistant. When they
ask for a document, the assistant follows its steps: find the template with the tools, read it,
use the roles and the example, ask only the needed questions, render, and, on a validation error,
ask again about the faulty fields only. The skill works with any template, not only the
demonstration.

**Why this priority**: the user wants the MCP server plus generic skills; this skill turns the
tools and roles into a fast, repeatable behaviour, and keeps the assistant from exploring local
files.

**Independent Test**: install the skill, connect the assistant to inkpdf, ask for the
demonstration document in plain language, and check the sequence of tool calls and questions.

**Acceptance Scenarios**:

1. **Given** the skill installed and inkpdf connected, **When** the end user asks for a document,
   **Then** the assistant uses only the inkpdf tools to find and read the template, and reads no
   local file.
2. **Given** the end user has given every **ask** value, **When** the assistant renders, **Then**
   it succeeds at the first attempt.
3. **Given** a render returns validation errors, **When** the assistant reads them, **Then** it
   asks the end user about the faulty fields only, and renders again.
4. **Given** a template other than the demonstration, written by a user, **When** the skill is
   used with it, **Then** it follows the same steps: nothing in the skill depends on a specific
   template.

---

### User Story 3 - A generic skill to write templates (Priority: P2)

A template author installs the "write an inkpdf template" skill. When they ask their assistant
to create or improve a template for their own document, the assistant follows its steps: the
template folder (created in the templates folder, whose location it asks the author for when
it does not know it), the schema with a description and a role for every variable, the layout
settings with defaults, the example input, the use of bundled packages, and a test through the
inkpdf tools before calling it done.

**Why this priority**: inkpdf now ships no business template; every user writes their own. This
skill is how they get templates that assistants can use well.

**Independent Test**: install the skill, ask the assistant to write a template for a simple
document (for example a meeting attendance sheet), and check that the result loads, that every
variable has a description and a role, and that its example renders.

**Acceptance Scenarios**:

1. **Given** the skill installed, **When** an author asks for a new template, **Then** the result
   is a template folder that inkpdf loads without error.
2. **Given** that new template, **When** its schema is read, **Then** every variable has a
   description and a role, and the template ships an example that renders unchanged.
3. **Given** an existing template without roles, **When** the author asks the assistant to
   improve it, **Then** the assistant adds descriptions and roles without changing the data the
   template accepts.

---

### User Story 4 - An example comes with each template (Priority: P2)

A template can ship one complete, valid example input. The assistant receives it with the
template's details, sees what a correct input looks like, and builds its own from it.

**Why this priority**: an example removes most guessing about formats and structure at almost no
cost.

**Independent Test**: read the demonstration template's details, render the example it contains
unchanged, and check that it succeeds.

**Acceptance Scenarios**:

1. **Given** a template that ships an example, **When** the assistant or an HTTP caller reads its
   details, **Then** the example is included and renders without error when sent as is.
2. **Given** a template whose example does not pass its own schema, **When** it is loaded,
   **Then** the example is not offered, the problem is logged, and the template stays usable.
3. **Given** a template that ships no example, **When** its details are read, **Then** they are
   the same as today, without an example.

---

### User Story 5 - Impossible dates are refused at validation (Priority: P2)

A date that has the right shape but does not exist (for example `2026-02-30` or `2026-13-45`) is
refused at validation, like any other invalid value: a validation error with the exact path of
the field and the "fix the input" hint. It no longer ends in a render failure telling the
assistant to "tell the operator".

**Why this priority**: the audit (defect 03) found that this is the main gap in today's
validation: 25 cases out of 25 ended in a render failure without the field's name, while the
same date in the metadata is refused cleanly.

**Independent Test**: render the demonstration template with an impossible date in each of its
date variables, and check that each answer is a validation error naming the field.

**Acceptance Scenarios**:

1. **Given** a template variable declared as a date, **When** the input gives `2026-02-30`,
   **Then** the answer is a validation error on that variable's path saying it is not a valid
   calendar date, and no rendering is attempted.
2. **Given** several impossible dates and other faults in one input, **When** it is rendered,
   **Then** one answer lists all of them.
3. **Given** a valid date such as `2024-02-29`, **When** it is rendered, **Then** it is accepted.

---

### User Story 6 - One demonstration template instead of three (Priority: P2)

The repository ships a single, simple, neutral demonstration template that shows every
possibility of a template: the main kinds of variables (text, number, decimal amount as text,
date, yes/no, choice from a list, list of items, nested group), a description and a role for
each, layout settings with defaults, PDF metadata, at least one bundled package, and an example
input. The invoice, `sample` and `packages-demo` templates are removed, together with what only
existed for them.

**Why this priority**: the invoice was a misleading example (legal and payment rules that inkpdf
does not own); one complete, neutral example is what template authors and the skills need.

**Independent Test**: list the templates of the repository's example volume and find only the
demonstration; check that the documentation, the request collection and the automated checks no
longer refer to the removed templates and still pass.

**Acceptance Scenarios**:

1. **Given** the repository's example volume, **When** the templates are listed, **Then** only
   the demonstration template is returned, and it is valid.
2. **Given** the documentation site, the example request collection and the automated checks,
   **When** they are built or run, **Then** none refers to the removed templates and all pass.
3. **Given** the demonstration template, **When** it is rendered with its example and with each
   layout setting changed, **Then** every rendering succeeds.

---

### User Story 7 - Choose which version of a template to use (Priority: P2)

A template author keeps several versions of a template side by side: version 1 stays in use while
version 2 is prepared. A caller (assistant or HTTP integrator) can ask for a given version when it
reads the template or generates a document; without a version it gets the most recent one, and it
is always told which version was used.

**Why this priority**: documents evolve; callers that depend on version 1 must keep getting
exactly version 1 while version 2 is introduced, and the version number templates already carry
is not used today.

**Independent Test**: put versions 1.0.0, 1.1.0 and 2.0.0 of one template in the volume; render
without a version, with `1`, with `1.0.0` and with `3`, and check which version is used each time
and what the error says for `3`.

**Acceptance Scenarios**:

1. **Given** versions 1.0.0, 1.1.0 and 2.0.0 of a template, **When** a caller renders it without a
   version, **Then** version 2.0.0 is used and the response says so.
2. **Given** the same template, **When** a caller asks for version `1`, **Then** version 1.1.0 is
   used; **When** it asks for `1.0.0`, **Then** version 1.0.0 is used.
3. **Given** the same template, **When** a caller asks for version `3`, **Then** it gets a "version
   not found" error that lists the available versions, and no rendering is attempted.
4. **Given** the template list, **When** a caller reads it, **Then** each template shows its
   available versions and the one used by default.
5. **Given** version 2.0.0 is invalid and 1.1.0 is valid, **When** a caller renders without a
   version, **Then** 1.1.0 is used; version 2.0.0 is listed as invalid with its reason.
6. **Given** a template laid out as today (files at the root of its folder), **When** it is listed
   and rendered, **Then** it behaves exactly as before, with the version of its `template.json`, if
   any.

---

### User Story 8 - Templates are loaded at startup; hot reload is for development (Priority: P2)

In production, the service loads its templates when it starts and keeps them unchanged while it
runs: publishing a new template or version means restarting the service. A developer can turn on
hot reload with one setting, so that changes in the volume are picked up without restarting.

**Why this priority**: the user does not want hot reload in production; with versions, a template
that cannot change under a caller's feet is the safer rule.

**Independent Test**: start the service with default settings, add a template to the volume, and
check it does not appear until a restart; start it with hot reload on and check it appears.

**Acceptance Scenarios**:

1. **Given** default settings, **When** a template or version is added to or changed in the volume
   while the service runs, **Then** the service keeps serving what it loaded at startup.
2. **Given** default settings, **When** the service is restarted, **Then** the new template or
   version is available.
3. **Given** hot reload turned on, **When** a template is added, changed or removed, **Then** the
   change is picked up without restart, as today.
4. **Given** the repository's Docker Compose file, used for local development, **When** it starts
   the service, **Then** hot reload is on.

---

### Edge Cases

- A variable inside a list of items: the list and each item variable can carry their own role
  (for example the list is **ask**, an item's position number is **fill**).
- A **fill** variable whose value the assistant cannot work out: the assistant asks the end user
  instead of inventing it; the instructions and the generating skill say so.
- A **default** variable without a default value: the template is still valid, and the
  instructions tell the assistant to leave the variable out.
- A date variable in a list of items, or a date-and-time value: the calendar check applies to
  every value declared as a date, at any depth.
- A template written for an earlier version, without roles or example: it loads and renders
  exactly as before.
- A template folder that holds both files at its root and version sub-folders: the template is
  reported as invalid, with a reason saying to choose one layout.
- A sub-folder whose name is not a version number (`draft/`) inside a versioned template: it is
  ignored and reported in the logs.
- A `template.json` whose version differs from its version folder's name: that version is invalid,
  with a reason naming both numbers.
- Every version of a template is invalid: the template is listed with its invalid versions and
  rendering it without a version returns the same "template unavailable" error as today.
- A version selector that is not a version number (`latest`, `v2`): refused with a clear error on
  the version parameter.
- A template laid out as today whose `template.json` version is not a `MAJOR.MINOR.PATCH` number
  (`1.0`, `v1`): the version is shown as written and used when no version is asked for; asking for
  any version returns "version not found". A version that is such a number can be asked for like
  any other.
- The removed templates were used by automated checks and benchmarks: those checks move to the
  demonstration template or to dedicated test fixtures, and keep the same coverage.

## Requirements *(mandatory)*

### Functional Requirements

**Roles and descriptions**

- **FR-001**: The template format MUST let every variable of the data and the layout declare a
  role among **ask**, **fill** and **default**, next to its description.
- **FR-002**: A role outside these three values MUST make the template invalid when loaded, with a
  reason naming the variable and the role. A missing role MUST be allowed.
- **FR-003**: Roles and descriptions MUST reach the assistant unchanged through the template's
  details, and the HTTP API MUST expose them the same way.
- **FR-004**: The server instructions MUST explain the three roles and the fallback for variables
  without a role (required → **ask**, optional with a default → **default**), say that an
  assistant must ask rather than invent a value it cannot work out, and say that everything it
  needs is available through the tools.

**Example**

- **FR-005**: A template MUST be able to ship one example input (data, and optionally layout and
  metadata), which the template details MUST include for assistants and HTTP callers.
- **FR-006**: The example MUST be checked against the template when the template is loaded; an
  example that fails MUST NOT be offered, MUST be logged, and MUST NOT make the template
  unavailable.

**Dates**

- **FR-007**: Every value that a template declares as a date MUST be checked for calendar validity
  at validation time, at any depth of the data, with the same error form as other validation
  errors (path, message, "fix the input" hint), and reported together with all other faults of
  the input. The same applies to every other standard format a template declares (date and time,
  e-mail address, URL…).

**Demonstration template**

- **FR-008**: The repository MUST ship exactly one demonstration template, neutral (no business
  domain), covering the kinds of variables, roles, layout settings, metadata, bundled package and
  example listed in User Story 6. Every variable MUST have a description and a role.
- **FR-009**: The `sample`, `packages-demo` and `progress-invoice` templates MUST be removed, with
  their example requests, their request collection entries and their documentation pages. The
  automated checks that used them MUST keep their coverage through the demonstration template or
  dedicated test fixtures.

**Generic skills**

- **FR-010**: The repository MUST provide a "generate a document with inkpdf" skill that works
  with any template and follows the steps of User Story 2.
- **FR-011**: The repository MUST provide a "write an inkpdf template" skill that follows the
  steps of User Story 3 and produces templates meeting FR-001, FR-005 and FR-007.
- **FR-012**: Both skills MUST be installable by a user in their assistant by following the
  documentation, without cloning or building the service.

**Errors on tool arguments**

- **FR-014**: Faults in the tool arguments themselves (unknown key, missing `templateId` or
  `data`, `data` that is not an object) MUST come back as tool errors that name the faulty argument
  and say to fix the arguments, so the assistant always sees them. Only an unknown tool name stays a
  protocol error.

**Template versions**

- **FR-015**: A template MUST be able to hold several versions, one sub-folder per version named
  `MAJOR.MINOR.PATCH`; each version folder is a complete template. A template laid out as today
  MUST keep working as a single version.
- **FR-016**: Each version MUST be loaded and checked on its own; an invalid version MUST NOT hide
  the valid ones, and MUST be listed with its reason.
- **FR-017**: Reading a template's details or schema, and rendering, MUST accept an optional
  version: an exact version or a major version number. Without one, the most recent valid version
  MUST be used. An unknown version MUST return a "version not found" error listing the available
  versions.
- **FR-018**: Every render of a template that has a version MUST state it, through HTTP and MCP,
  and the logs MUST record it.
- **FR-019**: The template list MUST show, for each template, its available versions with their
  status and the version used by default.

**Loading**

- **FR-020**: By default the service MUST load the templates once, at startup, and MUST NOT watch
  the volume or reload while running.
- **FR-021**: A single setting MUST turn hot reload on (watching and reloading as today), for
  development; the repository's Docker Compose file MUST turn it on, and the documentation MUST
  say it is for development only.

**Documentation**

- **FR-013**: The template format reference, the template writing guide, the MCP guide and the
  HTTP API reference MUST describe roles, the example and date checking, and the documentation
  MUST explain how to install and use the two skills.

### Key Entities

- **Variable role**: **ask**, **fill** or **default**; declared next to a variable's description;
  tells the assistant who provides the value.
- **Template example**: one complete, valid input shipped with a template; included in the
  template details.
- **Demonstration template**: the single neutral template of the repository, showing every
  possibility.
- **Template version**: one self-contained folder of a template, named after its version number;
  selected exactly, by major number, or by default (most recent valid).
- **Generic skill**: a set of instructions installed in an assistant, independent of any specific
  template: one for generating documents, one for writing templates.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With the generating skill and the demonstration template, an assistant asks the end
  user about 100 % of the **ask** variables it does not know and about 0 **fill** or untouched
  **default** variables, then renders successfully at the first attempt.
- **SC-002**: 100 % of the demonstration template's variables have a description and a role.
- **SC-003**: The demonstration template's example renders without error, unchanged.
- **SC-004**: 100 % of the audit's impossible dates (`2026-02-30`, `2026-13-45`, `2025-02-29`,
  `2026-04-31`, `0000-00-00`), placed in any variable declared as a date at any depth, are refused
  with a validation error naming the field; 0 end in a render failure.
- **SC-005**: A template written with the writing skill for a simple document loads without error,
  has a description and a role for every variable, and its example renders, at the first try.
- **SC-006**: 0 references to the removed templates remain in the documentation, the examples and
  the automated checks, and all of them pass.
- **SC-007**: With versions 1.0.0, 1.1.0 and 2.0.0 of a template, 100 % of renders use the version
  expected for no selector, `1`, `1.0.0` and `2`, and state it; `3` returns "version not found"
  with the list.
- **SC-008**: With default settings, 0 changes made to the volume while the service runs are
  visible before a restart; with hot reload on, a change is visible within the existing rescan
  delay.

## Assumptions

- The role is a template-format convention carried in the schema next to the description; inkpdf
  checks its value but does not use it to validate input: a role guides the assistant, it does not
  change what input is accepted.
- The demonstration template's identifier is `demo`.
- The skills follow the standard skill format used by Claude and other assistants, so they are
  not tied to one product; their exact packaging and installation path are a planning decision.
- The skills live in the repository and are versioned with the service.
- Values that do not match a standard format declared by a template (for example an invalid
  e-mail address in a field declared as e-mail) are now refused with a validation error instead of
  being printed. This is a correction, stated in the release notes.
- Removing the three example templates is not a breaking change for users: they were examples in
  the repository, never shipped inside the service image.
- Version folder names follow `MAJOR.MINOR.PATCH` (numbers only); pre-release suffixes are not
  supported in this feature.
- The live OpenAPI document describes the most recent valid version of each template; other
  versions are reachable through the version parameter and the template details.
- The `demo` template uses the versioned layout (`demo/1.0.0/`) so that it shows this possibility.
- Out of scope: a separate check-without-rendering tool, a summary of the rendered document,
  returning a download link instead of the embedded PDF (later storage feature), authentication,
  and the layout and wording defects of the audit, which concerned the removed templates.
- Constitution 2.0.0 (amended for this feature): principle V now loads templates at startup and
  keeps hot reload for development; principle II allows one sub-folder per version; principle VI
  lets reads and renders take a version. The demonstration template is the "neutral demonstration
  template" that the development workflow asks for.
