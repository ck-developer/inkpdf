---
name: generate-document
description: "Generate a PDF document with an inkpdf server: find the right template, ask the user only what the template marks as `ask`, fill in `fill` values, keep defaults, render, and fix validation errors with the user. Use when the user asks for a document, a PDF, a letter, a report or any file that an inkpdf template can produce."
license: MIT OR Apache-2.0
metadata:
  author: inkpdf
  homepage: https://ck-developer.github.io/inkpdf/
---

# Generate a document with inkpdf

An inkpdf server turns templates into PDF documents. Each template describes the input it needs
in a JSON Schema: every variable has a description and, usually, a role that says who provides
its value. These steps work for any template: nothing here depends on a particular one.

## 1. Use the inkpdf tools only

The inkpdf MCP server offers three tools: `list_templates`, `get_template` and
`render_template`. Everything you need is available through them. Do not look for templates,
schemas or examples in local files, folders or repositories, even when you can: the server's
answers are the only reference.

If these tools are not available, tell the user that the inkpdf server is not connected, and
stop.

## 2. Find the template

Call `list_templates`, and pick the template whose name and description match the request.

- One template fits: use it.
- None fits, or several could: ask the user, with the candidates' names in a short list. Do not
  pick one at random.
- The template that fits is `invalid`: tell the user, with its `reason`. It cannot be rendered
  until whoever runs the server fixes it.

Use the template's default version unless the user asks for a specific one. `get_template` and
`render_template` take an optional `version`: an exact version (`1.2.0`) or a major version
(`1`, the most recent `1.x.y`).

## 3. Read the template

Call `get_template` with the `templateId` (and the `version`, if the user asked for one).
Read:

- the schema of `data` and `layout`: required properties, types, allowed values, patterns,
  formats, bounds and defaults;
- the `description` of every variable: it says what the value is, and sometimes how to build
  it;
- the `x-role` of every variable (step 4);
- the `example`, when there is one: a complete, valid input. Start from its structure, but
  never send its values as if the user had given them.

## 4. Collect the values

The role of a variable says who provides it:

| `x-role` | What you do |
|---|---|
| `ask` | Ask the user, unless they already gave the value. |
| `fill` | Fill it in yourself from the context: today's date, a value derived from the user's answers, a rule given in the description. Ask the user if you cannot work it out. |
| `default` | Leave the variable out, so that its default applies, unless the user mentions it. If it has no default, leave it out. |

A variable without a role: treat it as `ask` when it is required, and as `default` when it is
optional.

- **Ask rather than invent.** Never make up a value the user should give: a name, an address,
  an amount, a date, a line of a list. A placeholder ("John Doe", "123 Main Street") is an
  invented value.
- **One message for all the questions.** List every missing `ask` value at once, with what the
  description says about it (format, unit, allowed values). Skip what the user already said,
  including in their first request.
- **Roles apply at every depth.** A list and each property of its items can have their own role:
  ask for the items of a list together.
- **Respect the schema** when you fill in or convert a value: dates as `YYYY-MM-DD`, amounts and
  codes in the pattern the schema gives, choices spelled exactly as in `enum`.
- **`layout`** only changes the appearance. Leave it out unless the user asks for a change;
  then send only the properties they mention.
- **`metadata`** (the PDF's own properties, described in `render_template`'s input schema) is
  optional. Set it only when the user asks for it, or when the template's description says to.

## 5. Render

Call `render_template` with the `templateId`, the `data`, only the `layout` and `metadata` you
need, and the `version` if the user asked for one. A `filename` (without `.pdf`) that names the
document is better than the default, the template id.

The answer is a short text naming the file, the template and its version, and the PDF itself.

When the call fails, the answer is an error whose last line says what to do:

- **`validation-failed`**: every faulty value is listed with its path (`/data/…`) and a message.
  Ask the user about those fields only, in one message, and call `render_template` again with
  the corrected values; keep every other value as it was. If a faulty value is one you filled
  in, correct it yourself.
- **The arguments of the call are wrong** (the error ends with "Fix the arguments and call the
  tool again."): correct your call and try again. The user has nothing to do.
- **`template-not-found`**: call `list_templates` again; the template may have another id.
- **`template-version-not-found`**: tell the user which versions exist (the error lists them)
  and ask which one to use.
- **`overloaded` or `render-timeout`**: try once more after a moment. If it fails again, tell
  the user.
- **Any other error** (`template-invalid`, `render-failed`, …): report it to the user as is. It
  needs whoever runs the inkpdf server; do not retry with other data.

If the same fields fail twice, show the user the messages and ask how to proceed.

## 6. Hand over the document

Tell the user the file name and offer the file, as your environment allows. Describe only what
you actually sent: you may summarize the values you rendered with, but never claim content,
totals or wording that are neither in your input nor in the tool's answer.

When the user asks for a change, render again with that change and every other value as
before.
