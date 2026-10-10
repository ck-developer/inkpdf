//! `packages/lock.toml` and the vendored archives of the bundled Typst packages.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

/// Header kept at the top of `packages/lock.toml`.
pub const HEADER: &str = "# Typst packages embedded in inkpdf. Edit via `cargo xtask packages`.\n\
# Contract: specs/002-typst-packages/contracts/lock-file.md\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Selected,
    Dependency,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub license: String,
    pub role: Role,
}

impl Entry {
    pub fn archive_name(&self) -> String {
        archive_name(&self.name, &self.version)
    }

    pub fn key(&self) -> (String, Version) {
        (
            self.name.clone(),
            Version::parse(&self.version).unwrap_or_default(),
        )
    }
}

pub fn archive_name(name: &str, version: &str) -> String {
    format!("{name}-{version}.tar.gz")
}

/// `MAJOR.MINOR.PATCH`, ordered numerically.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u32, pub u32, pub u32);

impl Version {
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.trim().split('.').map(|p| p.parse::<u32>().ok());
        let version = Version(parts.next()??, parts.next()??, parts.next()??);
        parts.next().is_none().then_some(version)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Lock {
    #[serde(default)]
    pub package: Vec<Entry>,
}

impl Lock {
    pub fn path(root: &Path) -> PathBuf {
        root.join("packages/lock.toml")
    }

    pub fn vendor(root: &Path) -> PathBuf {
        root.join("packages/vendor")
    }

    pub fn read(root: &Path) -> Result<Self> {
        let path = Self::path(root);
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    /// Serialized lock file: header, then entries sorted by name and version.
    pub fn to_toml(&self) -> String {
        let mut entries = self.package.clone();
        entries.sort_by_key(Entry::key);
        let mut out = HEADER.to_owned();
        for entry in entries {
            out.push_str(&format!(
                "\n[[package]]\nnamespace = \"{}\"\nname = \"{}\"\nversion = \"{}\"\nsha256 = \"{}\"\nlicense = \"{}\"\nrole = \"{}\"\n",
                entry.namespace,
                entry.name,
                entry.version,
                entry.sha256,
                entry.license,
                match entry.role {
                    Role::Selected => "selected",
                    Role::Dependency => "dependency",
                },
            ));
        }
        out
    }
}

/// Files of a package archive (`.tar.gz`), by relative path.
pub fn archive_files(archive: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(archive));
    for item in tar.entries().context("unreadable archive")? {
        let mut item = item.context("unreadable archive")?;
        if !item.header().entry_type().is_file() {
            continue;
        }
        let path = item
            .path()?
            .to_string_lossy()
            .trim_start_matches("./")
            .to_owned();
        let mut data = Vec::new();
        item.read_to_end(&mut data)?;
        files.insert(path, data);
    }
    Ok(files)
}

/// `[package]` table of the archive's `typst.toml`.
pub fn manifest(files: &BTreeMap<String, Vec<u8>>) -> Result<toml::Table> {
    let text = files
        .get("typst.toml")
        .ok_or_else(|| anyhow!("typst.toml missing"))
        .and_then(|bytes| Ok(std::str::from_utf8(bytes)?))?;
    let table: toml::Table = text.parse()?;
    table
        .get("package")
        .and_then(|p| p.as_table())
        .cloned()
        .ok_or_else(|| anyhow!("[package] missing in typst.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_order_numerically() {
        assert!(Version::parse("0.10.0") > Version::parse("0.9.9"));
        assert_eq!(Version::parse("1.2"), None);
        assert_eq!(Version::parse("1.2.3.4"), None);
    }

    #[test]
    fn lock_is_written_sorted_with_header() {
        let entry = |name: &str, version: &str| Entry {
            namespace: "preview".into(),
            name: name.into(),
            version: version.into(),
            sha256: "00".into(),
            license: "MIT".into(),
            role: Role::Dependency,
        };
        let lock = Lock {
            package: vec![
                entry("zero", "0.10.0"),
                entry("cetz", "0.5.2"),
                entry("zero", "0.9.0"),
            ],
        };
        let text = lock.to_toml();
        assert!(text.starts_with(HEADER));
        let reparsed: Lock = toml::from_str(&text).unwrap();
        let order: Vec<_> = reparsed
            .package
            .iter()
            .map(|e| format!("{}@{}", e.name, e.version))
            .collect();
        assert_eq!(order, ["cetz@0.5.2", "zero@0.9.0", "zero@0.10.0"]);
    }
}
