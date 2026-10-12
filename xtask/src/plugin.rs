//! Checks of the Claude Code plugin shipped in the repository, run by `cargo xtask docs check`:
//!
//! - `.claude-plugin/marketplace.json`, `plugins/inkpdf/.claude-plugin/plugin.json` and
//!   `plugins/inkpdf/.mcp.json` parse;
//! - `plugin.json`'s `version` is the version of the `inkpdf` crate (release-please bumps both);
//! - every skill folder has a `SKILL.md` whose frontmatter only uses the portable fields of the
//!   Agent Skills format, with a `name` equal to the folder name and a `description`.
//!
//! The generated `packages.md` of the `write-template` skill is checked with the other
//! generated pages (`docs.rs`).

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde_json::Value;

/// The plugin folder, relative to the repository root.
const PLUGIN: &str = "plugins/inkpdf";

/// Frontmatter fields that every skill host understands (Agent Skills format).
const PORTABLE_FIELDS: [&str; 6] = [
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
];

/// Longest `description` the Agent Skills format accepts, in characters.
const MAX_DESCRIPTION: usize = 1024;

/// Problems found in the plugin files, one line each; empty when everything is valid.
pub fn problems(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for file in [
        ".claude-plugin/marketplace.json".to_owned(),
        format!("{PLUGIN}/.mcp.json"),
    ] {
        if let Err(error) = read_json(root, &file) {
            problems.push(format!("{file}: {error:#}"));
        }
    }

    let manifest = format!("{PLUGIN}/.claude-plugin/plugin.json");
    match read_json(root, &manifest) {
        Ok(plugin) => {
            let expected = crate_version(root)?;
            match plugin["version"].as_str() {
                Some(version) if version == expected => {}
                Some(version) => problems.push(format!(
                    "{manifest}: version \"{version}\" differs from the inkpdf crate version \
                     \"{expected}\""
                )),
                None => problems.push(format!("{manifest}: no `version`")),
            }
        }
        Err(error) => problems.push(format!("{manifest}: {error:#}")),
    }

    let skills = root.join(PLUGIN).join("skills");
    let mut folders: Vec<String> = fs::read_dir(&skills)
        .with_context(|| format!("reading {}", skills.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.'))
        .collect();
    folders.sort();
    for folder in folders {
        let path = format!("{PLUGIN}/skills/{folder}/SKILL.md");
        match fs::read_to_string(root.join(&path)) {
            Ok(text) => problems.extend(
                skill_problems(&folder, &text)
                    .into_iter()
                    .map(|problem| format!("{path}: {problem}")),
            ),
            Err(_) => problems.push(format!("{path}: missing")),
        }
    }
    Ok(problems)
}

fn read_json(root: &Path, file: &str) -> Result<Value> {
    let bytes = fs::read(root.join(file)).context("unreadable")?;
    serde_json::from_slice(&bytes).context("invalid JSON")
}

/// `package.version` of `crates/inkpdf/Cargo.toml`.
fn crate_version(root: &Path) -> Result<String> {
    let path = root.join("crates/inkpdf/Cargo.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let manifest: toml::Table =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    manifest
        .get("package")
        .and_then(|package| package.get("version"))
        .and_then(|version| version.as_str())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("{}: no `package.version` string", path.display()))
}

/// Problems of one `SKILL.md`, whose folder is named `folder`.
fn skill_problems(folder: &str, text: &str) -> Vec<String> {
    let fields = match frontmatter(text) {
        Ok(fields) => fields,
        Err(problem) => return vec![problem],
    };
    let mut problems = Vec::new();
    for (key, _) in &fields {
        if !PORTABLE_FIELDS.contains(&key.as_str()) {
            problems.push(format!(
                "frontmatter field `{key}` is not portable (allowed: {})",
                PORTABLE_FIELDS.join(", ")
            ));
        }
    }
    let value = |key: &str| {
        fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, value)| value.as_str())
            .filter(|value| !value.is_empty())
    };
    match value("name") {
        None => problems.push("frontmatter has no `name`".into()),
        Some(name) if name != folder => problems.push(format!(
            "`name` is \"{name}\", expected the folder name \"{folder}\""
        )),
        Some(name) if !valid_name(name) => problems.push(format!(
            "`name` \"{name}\" must be 1 to 64 lowercase letters, digits and single hyphens"
        )),
        Some(_) => {}
    }
    match value("description") {
        None => problems.push("frontmatter has no `description`".into()),
        Some(description) if description.chars().count() > MAX_DESCRIPTION => {
            problems.push(format!(
                "`description` has {} characters, at most {MAX_DESCRIPTION}",
                description.chars().count()
            ))
        }
        Some(_) => {}
    }
    problems
}

/// Skill name of the Agent Skills format: lowercase letters, digits and single hyphens, neither
/// first nor last, at most 64 characters.
fn valid_name(name: &str) -> bool {
    name.len() <= 64
        && name.split('-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// Top-level fields of a `SKILL.md` frontmatter (the YAML block between the first two `---`
/// lines), each with its value on one line. Only the YAML that skills use is read: `key: value`
/// lines with a plain or quoted value, block scalars (`>`, `|`) and nested maps, whose indented
/// lines belong to the field above them.
fn frontmatter(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return Err("no frontmatter: the file must start with a `---` line".into());
    }
    // Key, raw value, and whether the value is a block scalar.
    let mut fields: Vec<(String, String, bool)> = Vec::new();
    for line in lines {
        if line.trim_end() == "---" {
            return Ok(fields
                .into_iter()
                .map(|(key, raw, block)| {
                    let value = if block { raw } else { unquote(&raw).to_owned() };
                    (key, value)
                })
                .collect());
        }
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if line.starts_with([' ', '\t']) {
            // Continuation of a block scalar or of a multi-line value; a nested map's entries
            // (under a key with no value) are not part of the field's value.
            if let Some((_, raw, block)) = fields.last_mut()
                && (*block || !raw.is_empty())
            {
                if !raw.is_empty() {
                    raw.push(' ');
                }
                raw.push_str(line.trim());
            }
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| format!("frontmatter line `{line}` is not `key: value`"))?;
        let value = value.trim();
        let block = value.starts_with(['>', '|']);
        let raw = if block {
            String::new()
        } else {
            value.to_owned()
        };
        fields.push((key.trim().to_owned(), raw, block));
    }
    Err("the frontmatter has no closing `---` line".into())
}

fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    const SKILL: &str = "---\n\
name: generate-document\n\
description: \"Generate a PDF: ask, fill, render.\"\n\
license: MIT OR Apache-2.0\n\
metadata:\n  author: inkpdf\n  homepage: https://example.org\n\
---\n\n# Generate a document\n";

    #[test]
    fn a_portable_frontmatter_passes() {
        assert_eq!(
            skill_problems("generate-document", SKILL),
            Vec::<String>::new()
        );
        assert_eq!(
            frontmatter(SKILL).unwrap(),
            [
                ("name".into(), "generate-document".into()),
                (
                    "description".into(),
                    "Generate a PDF: ask, fill, render.".into()
                ),
                ("license".into(), "MIT OR Apache-2.0".into()),
                ("metadata".into(), String::new()),
            ]
        );
    }

    #[test]
    fn a_field_outside_the_portable_set_is_refused() {
        let text = SKILL.replace("license:", "version: 1\nlicense:");
        let problems = skill_problems("generate-document", &text);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].contains("`version` is not portable"),
            "{problems:?}"
        );
    }

    #[test]
    fn name_and_description_are_required() {
        let text = "---\nlicense: MIT\n---\n";
        let problems = skill_problems("generate-document", text);
        assert_eq!(
            problems,
            [
                "frontmatter has no `name`",
                "frontmatter has no `description`"
            ]
        );
    }

    #[test]
    fn the_name_must_be_the_folder_name() {
        let problems = skill_problems("write-template", SKILL);
        assert_eq!(
            problems,
            ["`name` is \"generate-document\", expected the folder name \"write-template\""]
        );
        assert!(valid_name("write-template"));
        for name in ["Write", "write--template", "-write", "write_template", ""] {
            assert!(!valid_name(name), "{name}");
        }
    }

    #[test]
    fn a_long_description_is_refused() {
        let text = SKILL.replace("Generate a PDF: ask, fill, render.", &"a".repeat(1025));
        let problems = skill_problems("generate-document", &text);
        assert_eq!(
            problems,
            ["`description` has 1025 characters, at most 1024"]
        );
    }

    #[test]
    fn a_missing_or_unclosed_frontmatter_is_refused() {
        assert!(skill_problems("x", "# Title\n")[0].contains("no frontmatter"));
        assert!(skill_problems("x", "---\nname: x\n")[0].contains("no closing"));
    }

    #[test]
    fn block_and_multi_line_values_are_joined() {
        let text = "---\nname: x\ndescription: >\n  First line\n  second line.\n---\n";
        assert_eq!(frontmatter(text).unwrap()[1].1, "First line second line.");
        let text = "---\nname: x\ndescription: \"First line\n  second line.\"\n---\n";
        assert_eq!(frontmatter(text).unwrap()[1].1, "First line second line.");
    }

    #[test]
    fn the_repository_plugin_is_valid() {
        assert_eq!(problems(&crate::repo_root()).unwrap(), Vec::<String>::new());
    }
}
