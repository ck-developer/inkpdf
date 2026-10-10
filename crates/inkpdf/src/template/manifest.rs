//! Optional `template.json` manifest.

use serde::Deserialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
}

impl Manifest {
    /// Parses and checks the length bounds; the error is meant for the template author.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let manifest: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        check_len("name", manifest.name.as_deref(), 1, 120)?;
        check_len("description", manifest.description.as_deref(), 0, 2000)?;
        check_len("version", manifest.version.as_deref(), 0, 64)?;
        Ok(manifest)
    }
}

fn check_len(field: &str, value: Option<&str>, min: usize, max: usize) -> Result<(), String> {
    let Some(value) = value else { return Ok(()) };
    let len = value.chars().count();
    if len < min || len > max {
        return Err(format!(
            "`{field}` must be between {min} and {max} characters, got {len}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_fields() {
        let m = Manifest::parse(br#"{"name":"N","description":"D","version":"1.0.0"}"#).unwrap();
        assert_eq!(m.name.as_deref(), Some("N"));
        assert_eq!(m.version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn rejects_unknown_keys_and_bad_lengths() {
        assert!(Manifest::parse(br#"{"nmae":"typo"}"#).is_err());
        assert!(Manifest::parse(br#"{"name":""}"#).is_err());
        let long = format!(r#"{{"version":"{}"}}"#, "1".repeat(65));
        assert!(Manifest::parse(long.as_bytes()).is_err());
    }
}
