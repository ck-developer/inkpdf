//! Tool results: the rendered PDF and pipeline errors (contracts/mcp-tools.md).

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rmcp::model::{CallToolResult, ContentBlock, ResourceContents};

use crate::error::TemplateError;

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

/// Pipeline error as a tool error.
pub(super) fn error_result(error: TemplateError) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(error.error.to_string())])
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

    #[test]
    fn uri_segment_keeps_unreserved_characters_only() {
        assert_eq!(uri_segment("sample.pdf"), "sample.pdf");
        assert_eq!(uri_segment("Invoice 042.pdf"), "Invoice%20042.pdf");
        assert_eq!(uri_segment("Été.pdf"), "%C3%89t%C3%A9.pdf");
    }
}
