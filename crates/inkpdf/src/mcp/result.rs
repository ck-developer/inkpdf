//! Tool results: the rendered PDF and pipeline errors (contracts/mcp-tools.md).

use std::fmt::Write;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rmcp::model::{CallToolResult, ContentBlock, ResourceContents};

use crate::error::{ErrorCode, TemplateError};

/// Result of a render: a one-line summary, then the PDF as an embedded resource (research R4).
pub(super) fn pdf_result(template_id: &str, file_name: &str, pdf: &[u8]) -> CallToolResult {
    let summary = format!(
        "Rendered {file_name} from template {template_id} ({:.1} KiB).",
        pdf.len() as f64 / 1024.0
    );
    let uri = format!(
        "inkpdf://templates/{template_id}/render/{}",
        uri_segment(file_name)
    );
    let resource =
        ResourceContents::blob(BASE64.encode(pdf), uri).with_mime_type("application/pdf");
    CallToolResult::success(vec![
        ContentBlock::text(summary),
        ContentBlock::resource(resource),
    ])
}

/// Pipeline error as a tool error (FR-009, FR-010).
///
/// - `structuredContent`: the problem object the HTTP API returns for the same error.
/// - Text, for the assistant: the title and the detail, one line per violation or Typst
///   diagnostic, then what to do next ([`hint`]).
pub(super) fn error_result(error: TemplateError) -> CallToolResult {
    let code = error.error.code();
    let problem = error.error.into_problem(error.template_id);

    let mut text = problem.title.clone();
    // Some details only restate the title (`template not found`).
    if let Some(detail) = problem
        .detail
        .as_ref()
        .filter(|d| !d.eq_ignore_ascii_case(&text))
    {
        let _ = write!(text, ": {detail}");
    }
    for violation in problem.violations.iter().flatten() {
        let path = match violation.path.as_str() {
            "" => "(root)",
            path => path,
        };
        let _ = write!(text, "\n- {path}: {}", violation.message);
    }
    for diagnostic in problem.diagnostics.iter().flatten() {
        text.push_str("\n- ");
        if let Some(file) = &diagnostic.file {
            text.push_str(file);
            if let (Some(line), Some(column)) = (diagnostic.line, diagnostic.column) {
                let _ = write!(text, ":{line}:{column}");
            }
            text.push_str(": ");
        }
        text.push_str(&diagnostic.message);
    }
    text.push('\n');
    text.push_str(hint(code));

    let mut result = CallToolResult::error(vec![ContentBlock::text(text)]);
    result.structured_content =
        Some(serde_json::to_value(&problem).expect("Problem is always serializable"));
    result
}

/// Closing hint of an error: whether the assistant can fix the call, should retry, or must
/// hand over to the operator (contract, "Tool errors").
fn hint(code: ErrorCode) -> &'static str {
    match code {
        // `invalid-json` and `unsupported-media-type` cannot happen through MCP: the arguments
        // arrive as parsed JSON.
        ErrorCode::ValidationFailed
        | ErrorCode::InvalidParameter
        | ErrorCode::PayloadTooLarge
        | ErrorCode::InvalidJson
        | ErrorCode::UnsupportedMediaType => "Fix the input and call render_template again.",
        ErrorCode::TemplateNotFound => "Call list_templates to see the available templates.",
        ErrorCode::TemplateInvalid => "This template cannot be used until the operator fixes it.",
        ErrorCode::Overloaded | ErrorCode::RenderTimeout => "Try again in a moment.",
        ErrorCode::RenderFailed => "Tell the operator; the template failed to render.",
    }
}

/// Percent-encodes everything but the unreserved characters of RFC 3986, so that a cleaned
/// file name (spaces, accents) stays a valid URI path segment.
fn uri_segment(raw: &str) -> String {
    raw.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::error::{ApiError, Diagnostic, Violation};

    fn text(result: &CallToolResult) -> &str {
        &result.content[0].as_text().expect("a text block").text
    }

    #[test]
    fn error_text_lists_violations_then_the_hint() {
        let violation = |path: &str, message: &str| Violation {
            path: path.into(),
            schema_path: String::new(),
            message: message.into(),
        };
        let error = ApiError::ValidationFailed {
            violations: vec![
                violation("/data", "\"title\" is a required property"),
                violation("", "\"layout\" is a required property"),
            ],
        };
        let result = error_result(error.for_template("sample"));
        assert_eq!(result.is_error, Some(true));
        assert_eq!(
            text(&result),
            "Validation failed: 2 violation(s)\n\
             - /data: \"title\" is a required property\n\
             - (root): \"layout\" is a required property\n\
             Fix the input and call render_template again."
        );
        let problem = result.structured_content.unwrap();
        assert_eq!(problem["code"], "validation-failed");
        assert_eq!(problem["templateId"], "sample");
    }

    #[test]
    fn error_text_lists_diagnostics_with_their_position() {
        let diagnostic = |file: Option<&str>, message: &str| Diagnostic {
            message: message.into(),
            file: file.map(Into::into),
            line: file.map(|_| 3),
            column: file.map(|_| 2),
            hints: Vec::new(),
        };
        let error = ApiError::RenderFailed {
            diagnostics: vec![
                diagnostic(Some("main.typ"), "unknown variable: x"),
                diagnostic(None, "internal error"),
            ],
        };
        let result = error_result(error.into());
        assert_eq!(
            text(&result),
            "Render failed: template compilation failed\n\
             - main.typ:3:2: unknown variable: x\n\
             - internal error\n\
             Tell the operator; the template failed to render."
        );
        assert!(
            result
                .structured_content
                .unwrap()
                .get("templateId")
                .is_none()
        );
    }

    #[test]
    fn detail_restating_the_title_is_left_out() {
        let result = error_result(ApiError::TemplateNotFound.into());
        assert_eq!(
            text(&result),
            "Template not found\nCall list_templates to see the available templates."
        );
    }

    #[test]
    fn uri_segment_keeps_unreserved_characters_only() {
        assert_eq!(uri_segment("sample.pdf"), "sample.pdf");
        assert_eq!(uri_segment("Invoice 042.pdf"), "Invoice%20042.pdf");
        assert_eq!(uri_segment("Été.pdf"), "%C3%89t%C3%A9.pdf");
    }
}
