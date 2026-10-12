//! The three tools: list the templates, read one template's schema, render one template
//! (contracts/mcp-tools.md). Each one reuses the HTTP API's code for the same operation.

use std::io;
use std::sync::Arc;
use std::time::Instant;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::schemars::{self, JsonSchema, Schema, SchemaGenerator};
use rmcp::{tool, tool_router};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use super::InkpdfMcp;
use super::result::{error_result, pdf_result};
use crate::api::render::{check_filename, pdf_file_name};
use crate::api::templates::{self, TemplateDetail, parse_version};
use crate::error::{ApiError, TemplateError};
use crate::registry::{TemplateEntry, TemplateVersions};
use crate::render::{self, Interface};

/// Arguments of `list_templates`: none.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(crate = "rmcp::schemars")]
pub struct ListTemplatesArgs {}

/// Arguments of `get_template`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[schemars(crate = "rmcp::schemars")]
pub struct GetTemplateArgs {
    /// Template id, from list_templates.
    pub template_id: String,
    /// Template version, `2` or `2.0.0`; defaults to the highest valid version.
    #[serde(default)]
    #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Arguments of `render_template`: the HTTP render body (`data`, `layout`, `metadata`), the
/// template id and the `filename` query parameter.
///
/// Only the keys, and that `data` is an object, are checked here. What `data`, `layout` and
/// `metadata` hold is validated by the shared pipeline, so that it is reported exactly as over
/// HTTP: `layout` and `metadata` are taken as any JSON value, an explicit `null` included.
///
/// On the optional fields, `skip_serializing_if` only keeps schemars from writing
/// `"default": null` into the input schema.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[schemars(crate = "rmcp::schemars")]
pub struct RenderTemplateArgs {
    /// Template id, from list_templates.
    pub template_id: String,
    /// Business data, as described by the template's schema.
    #[schemars(schema_with = "object_schema")]
    pub data: Map<String, Value>,
    /// Layout parameters; omitted properties take their default.
    #[serde(default, deserialize_with = "present")]
    #[schemars(schema_with = "object_schema", skip_serializing_if = "Option::is_none")]
    pub layout: Option<Value>,
    /// PDF metadata (title, author, subject, keywords, date as `YYYY-MM-DD`), all optional.
    #[serde(default, deserialize_with = "present")]
    #[schemars(
        schema_with = "metadata_schema",
        skip_serializing_if = "Option::is_none"
    )]
    pub metadata: Option<Value>,
    /// File name without extension; defaults to the template id.
    #[serde(default)]
    #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Template version, `2` or `2.0.0`; defaults to the highest valid version.
    #[serde(default)]
    #[schemars(with = "String", skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// A key that is present, `null` included; a missing key stays `None` (`#[serde(default)]`).
fn present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

/// Any JSON object; its content is described by the template's schema.
fn object_schema(_: &mut SchemaGenerator) -> Schema {
    schemars::json_schema!({ "type": "object" })
}

/// The HTTP API's `DocumentMetadata` schema.
fn metadata_schema(_: &mut SchemaGenerator) -> Schema {
    Schema::try_from(render::metadata::schema()).expect("the metadata schema is an object")
}

/// Outcome of a tool: its result with the template (id and version) it concerns, or a
/// pipeline error.
type Outcome = Result<(Option<Concerned>, CallToolResult), TemplateError>;

/// The template a tool call concerns, as logged.
struct Concerned {
    id: String,
    version: Option<String>,
}

impl From<&TemplateEntry> for Concerned {
    fn from(entry: &TemplateEntry) -> Self {
        Self {
            id: entry.id.to_string(),
            version: entry.version.clone(),
        }
    }
}

#[tool_router(vis = "pub(super)")]
impl InkpdfMcp {
    #[tool(
        description = "List the document templates this inkpdf server can render, with their \
            id, name, description and status. Start here to find the template that matches \
            the user's request.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    async fn list_templates(&self, _: Parameters<ListTemplatesArgs>) -> CallToolResult {
        let started = Instant::now();
        let list = templates::template_list(&self.state);
        let empty = list.templates.is_empty();
        let mut result = CallToolResult::structured(to_json(&list));
        if empty {
            result
                .content
                .push(ContentBlock::text("No template is loaded."));
        }
        finish("list_templates", started, Ok((None, result)))
    }

    #[tool(
        description = "Get one template's details: its JSON Schema (`data` and `layout`) with \
            a description and an optional `x-role` (`ask`, `fill` or `default`) for each \
            variable, and, when the template ships one, a complete `example` input to start \
            from. `metadata` is described in render_template's input schema.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    async fn get_template(&self, Parameters(args): Parameters<GetTemplateArgs>) -> CallToolResult {
        let started = Instant::now();
        let outcome =
            self.find(&args.template_id, args.version.as_deref())
                .map(|(template, entry)| {
                    let detail = TemplateDetail::new(&template, &entry);
                    (
                        Some(Concerned::from(entry.as_ref())),
                        CallToolResult::structured(to_json(&detail)),
                    )
                });
        finish("get_template", started, outcome)
    }

    #[tool(
        description = "Render a template to PDF with the given data and return the PDF file. \
            The input must match the template's schema (see get_template). Nothing is stored.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    async fn render_template(
        &self,
        Parameters(args): Parameters<RenderTemplateArgs>,
    ) -> CallToolResult {
        let started = Instant::now();
        let outcome = self.render(args).await;
        finish("render_template", started, outcome)
    }
}

impl InkpdfMcp {
    /// The template `template_id` and the version of it asked for, as over HTTP: an unknown
    /// template first, then a malformed `version`, then a version that does not exist.
    fn find(
        &self,
        template_id: &str,
        version: Option<&str>,
    ) -> Result<(Arc<TemplateVersions>, Arc<TemplateEntry>), TemplateError> {
        let template = templates::find(&self.state, template_id)?;
        let selector = version
            .map(parse_version)
            .transpose()
            .map_err(|e| e.for_template(template.default_entry().id.as_str()))?;
        let entry = templates::select(&template, selector)?;
        Ok((template, entry))
    }

    /// Same steps as `POST /templates/{templateId}/render`, in the same order: template
    /// lookup, `filename` and `version`, body size, then the shared pipeline. The template id
    /// is attached to every error after the lookup, as over HTTP.
    async fn render(&self, args: RenderTemplateArgs) -> Outcome {
        let template = templates::find(&self.state, &args.template_id)?;
        let id = template.default_entry().id.to_string();
        let fail = |error: ApiError| error.for_template(&id);
        if let Some(filename) = &args.filename {
            check_filename(filename).map_err(fail)?;
        }
        let selector = args
            .version
            .as_deref()
            .map(parse_version)
            .transpose()
            .map_err(fail)?;
        let entry = templates::select(&template, selector)?;
        let concerned = Concerned::from(entry.as_ref());

        let mut body = Map::new();
        body.insert("data".into(), Value::Object(args.data));
        if let Some(layout) = args.layout {
            body.insert("layout".into(), layout);
        }
        if let Some(metadata) = args.metadata {
            body.insert(render::metadata::KEY.into(), metadata);
        }
        let body = Value::Object(body);
        // The HTTP API's body limit, on the same JSON (FR-011).
        let limit = self.state.config.max_body_bytes;
        if json_len(&body) > limit {
            return Err(fail(ApiError::PayloadTooLarge { limit }));
        }

        let pdf = render::run(&self.state, entry, body, Interface::Mcp)
            .await
            .map_err(fail)?;
        let file_name = pdf_file_name(&id, args.filename.as_deref());
        let result = pdf_result(&id, concerned.version.as_deref(), &file_name, &pdf);
        Ok((Some(concerned), result))
    }
}

/// Turns a tool's outcome into its result and logs the call (FR-014). No argument value is
/// logged; the template id only once it is known to be well-formed, and the template version
/// only once it is selected.
fn finish(tool: &str, started: Instant, outcome: Outcome) -> CallToolResult {
    let (template_id, version, result, outcome) = match outcome {
        Ok((concerned, result)) => {
            let (id, version) = concerned.map_or((None, None), |c| (Some(c.id), c.version));
            (id, version, result, "ok")
        }
        Err(error) => {
            let code = error.error.code().as_str();
            (error.template_id.clone(), None, error_result(error), code)
        }
    };
    log_call(
        tool,
        template_id.as_deref(),
        version.as_deref(),
        started,
        outcome,
    );
    result
}

/// The `mcp.tool` log event of one call (FR-014), with `templateVersion` (feature 005).
pub(super) fn log_call(
    tool: &str,
    template_id: Option<&str>,
    template_version: Option<&str>,
    started: Instant,
    outcome: &str,
) {
    tracing::info!(
        event = "mcp.tool",
        tool,
        templateId = template_id,
        templateVersion = template_version,
        durationMs = started.elapsed().as_millis() as u64,
        outcome,
    );
}

/// JSON of a response body of the HTTP API.
fn to_json(body: &impl serde::Serialize) -> Value {
    serde_json::to_value(body).expect("API bodies are always serializable")
}

/// Length of the compact JSON of `value`, counted without building it.
fn json_len(value: &Value) -> usize {
    struct Counter(usize);
    impl io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value).expect("a JSON value always serializes");
    counter.0
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn json_len_is_the_compact_length() {
        let value = json!({ "data": { "title": "Été", "items": [1, 2.5, null] } });
        assert_eq!(json_len(&value), serde_json::to_vec(&value).unwrap().len());
    }
}
