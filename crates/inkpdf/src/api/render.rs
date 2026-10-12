//! `POST /templates/{templateId}/render`.

use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::{Map, Value};
use utoipa::ToSchema;
use utoipa::openapi::RefOr;
use utoipa::openapi::schema::{ObjectBuilder, Schema, Type};

use crate::AppState;
use crate::api::templates::{self, TemplatePath, parse_version};
use crate::error::{ApiError, Problem};
use crate::registry::VersionSelector;
use crate::render::{self, Interface};

/// Response header naming the version of the template that rendered the document.
pub const TEMPLATE_VERSION_HEADER: &str = "inkpdf-template-version";

/// Generic shape; the exact shape of `data` and `layout` is given by the template's schema
/// (`GET /templates/{templateId}/schema`).
#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct RenderRequest {
    /// Business data.
    data: Map<String, Value>,
    /// Layout parameters; missing properties take their `default`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    layout: Option<Map<String, Value>>,
    /// PDF metadata, all optional. Without `author`, the service's default author
    /// (`INKPDF_DEFAULT_AUTHOR`, `inkpdf` by default); without `title`, the template's title,
    /// or failing that its name. `date` in `YYYY-MM-DD` format.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    metadata: Option<DocumentMetadataSchema>,
}

/// Schema of `metadata` (fixed, defined by the service).
pub struct DocumentMetadataSchema;

impl Serialize for DocumentMetadataSchema {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}

impl utoipa::PartialSchema for DocumentMetadataSchema {
    fn schema() -> RefOr<Schema> {
        serde_json::from_value(render::metadata::schema()).expect("valid OpenAPI schema")
    }
}

impl ToSchema for DocumentMetadataSchema {
    fn name() -> std::borrow::Cow<'static, str> {
        "DocumentMetadata".into()
    }
}

/// PDF document returned as is.
pub struct PdfDocument;

impl utoipa::PartialSchema for PdfDocument {
    fn schema() -> RefOr<Schema> {
        ObjectBuilder::new()
            .schema_type(Type::String)
            .content_media_type("application/pdf")
            .into()
    }
}

impl ToSchema for PdfDocument {}

/// Generate a PDF
///
/// Validates the body against the template's schema (after applying the `default` values of
/// `layout`), then returns the PDF. No generation is attempted if validation fails.
#[utoipa::path(
    post,
    path = "/templates/{templateId}/render",
    tag = "render",
    operation_id = "renderTemplate",
    params(TemplatePath, RenderQuery),
    request_body(content = RenderRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Generated document.", content_type = "application/pdf",
            body = inline(PdfDocument),
            headers(
                ("Content-Disposition" = String, description = "`inline` (display) or `attachment` (download, `download=true`), with `filename` and `filename*` (UTF-8)."),
                ("Inkpdf-Template-Version" = String, description = "Version of the template that rendered the document; absent when the template has no version."),
            )),
        (status = 400, response = crate::api::ProblemResponse),
        (status = 404, response = crate::api::ProblemResponse),
        (status = 409, response = crate::api::ProblemResponse),
        (status = 413, response = crate::api::ProblemResponse),
        (status = 415, response = crate::api::ProblemResponse),
        (status = 422, description = "Body does not match the template's schema.",
            content_type = "application/problem+json", body = Problem),
        (status = 500, response = crate::api::ProblemResponse),
        (status = 503, response = crate::api::ProblemResponse),
        (status = 504, response = crate::api::ProblemResponse),
    ),
)]
pub async fn render_template(
    State(state): State<AppState>,
    Path(TemplatePath { template_id }): Path<TemplatePath>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    // An unknown template is reported before any query or body error, an unknown version
    // after the query's errors.
    let template = match templates::find(&state, &template_id) {
        Ok(template) => template,
        Err(error) => return error.into_response(),
    };
    let id = template.default_entry().id.clone();
    let fail = |error: ApiError| error.for_template(id.as_str()).into_response();
    let query = match RenderQuery::parse(query.as_deref()) {
        Ok(query) => query,
        Err(error) => return fail(error),
    };
    let entry = match templates::select(&template, query.version) {
        Ok(entry) => entry,
        Err(error) => return error.into_response(),
    };
    let version = entry.version.clone();

    if !is_json(&headers) {
        return fail(ApiError::UnsupportedMediaType);
    }
    let body = match body {
        Ok(body) => body,
        Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
            return fail(ApiError::PayloadTooLarge {
                limit: state.config.max_body_bytes,
            });
        }
        Err(rejection) => {
            return fail(ApiError::InvalidJson {
                message: rejection.body_text(),
            });
        }
    };
    let body: Value = match serde_json::from_slice(&body) {
        Ok(Value::Object(map)) => Value::Object(map),
        Ok(_) => {
            return fail(ApiError::InvalidJson {
                message: "the request body must be a JSON object".into(),
            });
        }
        Err(e) => {
            return fail(ApiError::InvalidJson {
                message: e.to_string(),
            });
        }
    };

    match render::run(&state, entry, body, Interface::Http).await {
        Ok(pdf) => {
            let disposition =
                content_disposition(id.as_str(), query.download, query.filename.as_deref());
            let mut response = (StatusCode::OK, pdf).into_response();
            let headers = response.headers_mut();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/pdf"),
            );
            if let Ok(value) = HeaderValue::from_str(&disposition) {
                headers.insert(header::CONTENT_DISPOSITION, value);
            }
            // A manifest version that is not a valid header value is left out.
            if let Some(value) = version.and_then(|v| HeaderValue::from_str(&v).ok()) {
                headers.insert(TEMPLATE_VERSION_HEADER, value);
            }
            response
        }
        Err(error) => fail(error),
    }
}

/// `Content-Type: application/json`, parameters (`charset`) tolerated.
fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
}

/// Query parameters of the render request (R18).
#[derive(Debug, Default, PartialEq, Eq, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RenderQuery {
    /// `true`: respond as a download (`Content-Disposition: attachment`) instead of
    /// inline display. Values: `true`, `false`, `1`, `0`.
    #[param(required = false, default = false)]
    download: bool,
    /// File name (no path; `.pdf` appended). Default: `<templateId>.pdf`.
    #[param(max_length = 200)]
    filename: Option<String>,
    /// Template version: a major version (`2`, the highest valid `2.x.y`) or an exact version
    /// (`2.0.0`). Default: the highest valid version.
    #[param(value_type = Option<String>, pattern = r"^(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*))?$", example = "2.0.0")]
    version: Option<VersionSelector>,
}

impl RenderQuery {
    fn parse(query: Option<&str>) -> Result<Self, ApiError> {
        let mut parsed = Self::default();
        let pairs = query.map(form_urlencoded_pairs).unwrap_or_default();
        for (key, value) in pairs {
            match key.as_str() {
                "download" => {
                    parsed.download = match value.as_str() {
                        "true" | "1" => true,
                        "false" | "0" => false,
                        _ => {
                            return Err(ApiError::InvalidParameter {
                                name: "download",
                                message: format!("expected `true` or `false`, got `{value}`"),
                            });
                        }
                    }
                }
                "filename" => {
                    check_filename(&value)?;
                    parsed.filename = Some(value);
                }
                "version" => parsed.version = Some(parse_version(&value)?),
                _ => {}
            }
        }
        Ok(parsed)
    }
}

/// Rule of the `filename` parameter, shared with the MCP render tool: at most 200 characters.
pub(crate) fn check_filename(value: &str) -> Result<(), ApiError> {
    if value.chars().count() > 200 {
        return Err(ApiError::InvalidParameter {
            name: "filename",
            message: "at most 200 characters".into(),
        });
    }
    Ok(())
}

/// Decodes `a=b&c=d` (`application/x-www-form-urlencoded`).
pub(crate) fn form_urlencoded_pairs(query: &str) -> Vec<(String, String)> {
    fn decode(raw: &str) -> String {
        let bytes = raw.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            let hex = bytes
                .get(i + 1..i + 3)
                .and_then(|h| std::str::from_utf8(h).ok())
                .and_then(|h| u8::from_str_radix(h, 16).ok());
            match (bytes[i], hex) {
                (b'+', _) => out.push(b' '),
                (b'%', Some(byte)) => {
                    out.push(byte);
                    i += 2;
                }
                (byte, _) => out.push(byte),
            }
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(key), decode(value))
        })
        .collect()
}

/// `Content-Disposition` header (RFC 6266): cleaned name, ASCII fallback and UTF-8 `filename*`.
fn content_disposition(id: &str, download: bool, filename: Option<&str>) -> String {
    let name = pdf_file_name(id, filename);
    let ascii: String = name
        .chars()
        .map(|c| if c.is_ascii() { c } else { '_' })
        .collect();
    let encoded: String = name
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'!'
            | b'#'
            | b'$'
            | b'&'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    let kind = if download { "attachment" } else { "inline" };
    format!("{kind}; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

/// Suggested file name of a render: the cleaned `filename`, or the template id when it is
/// absent or empty after cleaning, with `.pdf` appended.
pub(crate) fn pdf_file_name(id: &str, filename: Option<&str>) -> String {
    let name = filename.map(clean_filename).filter(|n| !n.is_empty());
    format!("{}.pdf", name.as_deref().unwrap_or(id))
}

/// Strips paths, quotes and control characters; drops the `.pdf` extension.
fn clean_filename(raw: &str) -> String {
    let kept: String = raw
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '/' | '\\' | '"' | ':' | '*' | '?' | '<' | '>' | '|')
        })
        .collect();
    let trimmed = kept.trim_matches(|c: char| c.is_whitespace() || c == '.');
    let stem = if trimmed.to_ascii_lowercase().ends_with(".pdf") {
        &trimmed[..trimmed.len() - 4]
    } else {
        trimmed
    };
    stem.trim_matches(|c: char| c.is_whitespace() || c == '.')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_is_parsed() {
        assert_eq!(RenderQuery::parse(None).unwrap(), RenderQuery::default());
        let query = RenderQuery::parse(Some("download=1&filename=a%20b+c&x=y")).unwrap();
        assert!(query.download);
        assert_eq!(query.filename.as_deref(), Some("a b c"));
        assert!(RenderQuery::parse(Some("download=yes")).is_err());
    }

    #[test]
    fn version_is_a_major_or_an_exact_version() {
        let version = |query: &str| RenderQuery::parse(Some(query)).map(|q| q.version);
        assert_eq!(
            version("version=2").unwrap(),
            Some(VersionSelector::Major(2))
        );
        assert_eq!(
            version("version=1.0.0").unwrap(),
            Some(VersionSelector::Exact("1.0.0".parse().unwrap()))
        );
        for bad in ["latest", "v2", "1.2", "", "..%2F1.0.0", "1.0.0%2F.."] {
            let error = version(&format!("version={bad}")).unwrap_err();
            assert!(
                matches!(
                    error,
                    ApiError::InvalidParameter {
                        name: "version",
                        ..
                    }
                ),
                "{bad}: {error:?}"
            );
        }
    }

    #[test]
    fn filenames_are_cleaned() {
        assert_eq!(clean_filename("../../etc/\"pass\"\n.PDF"), "etcpass");
        assert_eq!(clean_filename("  Invoice 042.pdf "), "Invoice 042");
        assert_eq!(clean_filename("..."), "");
    }

    #[test]
    fn disposition_has_ascii_fallback_and_utf8_name() {
        assert_eq!(
            content_disposition("sample", false, None),
            "inline; filename=\"sample.pdf\"; filename*=UTF-8''sample.pdf"
        );
        assert_eq!(
            content_disposition("sample", true, Some("Été")),
            "attachment; filename=\"_t_.pdf\"; filename*=UTF-8''%C3%89t%C3%A9.pdf"
        );
        assert_eq!(
            content_disposition("sample", true, Some("///")),
            "attachment; filename=\"sample.pdf\"; filename*=UTF-8''sample.pdf"
        );
    }
}
