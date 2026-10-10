//! Typst Universe (`@preview`): package index and archives.

use std::io::Read;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use crate::lock::Version;

const INDEX_URL: &str = "https://packages.typst.org/preview/index.json";
const ARCHIVE_URL: &str = "https://packages.typst.org/preview";
/// Upper bound for a downloaded archive or index.
const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// One entry of the Universe index.
#[derive(Debug, Clone, Deserialize)]
pub struct IndexEntry {
    pub name: String,
    pub version: String,
    /// Minimum Typst version (may be partial, e.g. `0.14`).
    #[serde(default)]
    pub compiler: Option<String>,
}

/// Where packages come from: the network in real use, an in-memory fake in tests.
pub trait Source {
    fn index(&self) -> Result<Vec<IndexEntry>>;
    fn archive(&self, name: &str, version: &str) -> Result<Vec<u8>>;
}

/// `packages.typst.org`.
pub struct Universe {
    agent: ureq::Agent,
}

impl Universe {
    pub fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(30))
                .build(),
        }
    }

    fn get(&self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .agent
            .get(url)
            .call()
            .with_context(|| format!("GET {url}"))?;
        let mut bytes = Vec::new();
        response
            .into_reader()
            .take(MAX_BYTES)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

impl Source for Universe {
    fn index(&self) -> Result<Vec<IndexEntry>> {
        serde_json::from_slice(&self.get(INDEX_URL)?).context("parsing the Universe index")
    }

    fn archive(&self, name: &str, version: &str) -> Result<Vec<u8>> {
        self.get(&format!("{ARCHIVE_URL}/{name}-{version}.tar.gz"))
            .with_context(|| format!("downloading {name} {version}"))
    }
}

/// Typst version used by the service (`typst = "=X.Y.Z"` in `crates/inkpdf/Cargo.toml`).
pub fn engine_version(root: &Path) -> Result<Version> {
    let manifest = std::fs::read_to_string(root.join("crates/inkpdf/Cargo.toml"))?;
    let table: toml::Table = manifest.parse()?;
    let requirement = table["dependencies"]["typst"]
        .as_str()
        .ok_or_else(|| anyhow!("typst dependency not found in crates/inkpdf/Cargo.toml"))?;
    Version::parse(requirement.trim_start_matches('='))
        .ok_or_else(|| anyhow!("typst must be pinned to an exact version, found `{requirement}`"))
}

/// True if a package declaring `compiler` (a minimum, possibly partial) runs on `engine`.
pub fn compatible(compiler: Option<&str>, engine: Version) -> bool {
    let Some(bound) = compiler else { return true };
    let parts: Vec<u32> = bound.split('.').filter_map(|p| p.parse().ok()).collect();
    let bound = Version(
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    );
    bound <= engine
}

/// Latest version of `name` compatible with `engine`.
pub fn latest_compatible(index: &[IndexEntry], name: &str, engine: Version) -> Option<Version> {
    index
        .iter()
        .filter(|e| e.name == name && compatible(e.compiler.as_deref(), engine))
        .filter_map(|e| Version::parse(&e.version))
        .max()
}

/// Whether `name@version` exists and runs on `engine` (`Err` explains why not).
pub fn check_available(
    index: &[IndexEntry],
    name: &str,
    version: Version,
    engine: Version,
) -> Result<()> {
    let entry = index
        .iter()
        .find(|e| e.name == name && Version::parse(&e.version) == Some(version))
        .ok_or_else(|| anyhow!("@preview/{name}:{version} does not exist on Typst Universe"))?;
    if compatible(entry.compiler.as_deref(), engine) {
        Ok(())
    } else {
        Err(anyhow!(
            "@preview/{name}:{version} requires Typst {} (the service uses {engine})",
            entry.compiler.as_deref().unwrap_or("?")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, version: &str, compiler: Option<&str>) -> IndexEntry {
        IndexEntry {
            name: name.into(),
            version: version.into(),
            compiler: compiler.map(Into::into),
        }
    }

    #[test]
    fn latest_version_skips_incompatible_ones() {
        let engine = Version(0, 15, 1);
        let index = [
            entry("zero", "0.6.1", Some("0.12.0")),
            entry("zero", "0.7.1", Some("0.15")),
            entry("zero", "0.8.0", Some("0.16.0")),
            entry("other", "9.9.9", None),
        ];
        assert_eq!(
            latest_compatible(&index, "zero", engine),
            Some(Version(0, 7, 1))
        );
        assert_eq!(latest_compatible(&index, "missing", engine), None);
        assert!(check_available(&index, "zero", Version(0, 8, 0), engine).is_err());
        assert!(check_available(&index, "zero", Version(0, 6, 1), engine).is_ok());
    }

    #[test]
    fn engine_version_is_read_from_the_service_manifest() {
        let version = engine_version(&crate::repo_root()).unwrap();
        assert!(version >= Version(0, 15, 0));
    }
}
