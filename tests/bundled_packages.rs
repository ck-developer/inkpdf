//! Vérifications des paquets Typst intégrés (FR-010, R11) : chaque paquet s'importe, chaque
//! paquet mis à disposition s'utilise depuis un template, l'ensemble est fermé.

mod common;

use std::collections::BTreeSet;
use std::fs;

use common::*;
use inkpdf::packages;
use inkpdf::registry::{LoadOutcome, loader};
use typst::syntax::{SyntaxNode, ast, package::PackageSpec};

/// R11.1 — chacun des paquets intégrés (dépendances comprises) s'importe et compile.
#[test]
fn every_bundled_package_imports() {
    let failures: Vec<String> = packages::all()
        .iter()
        .filter_map(|package| {
            let main = format!("#import \"{}\"\n#[ok]\n", package.spec());
            let entry = entry_with_main("import-check", &main);
            compile(entry).err().map(|e| format!("{}: {e:?}", package.spec()))
        })
        .collect();
    assert!(failures.is_empty(), "packages failing to import:\n{}", failures.join("\n"));
}

/// Charge `main` comme un template déposé dans le volume (donc avec la résolution des imports).
fn load_template(id: &str, main: &str) -> std::sync::Arc<inkpdf::registry::TemplateEntry> {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.typ"), main).unwrap();
    fs::write(dir.path().join("schema.json"), MINIMAL_SCHEMA).unwrap();
    match loader::load(dir.path(), id.parse().unwrap(), u64::MAX) {
        LoadOutcome::Loaded(entry) => std::sync::Arc::from(entry),
        LoadOutcome::Unstable => panic!("unstable template directory"),
    }
}

fn smoke_fixture(name: &str) -> String {
    fs::read_to_string(repo_root().join(format!("tests/fixtures/package-smoke/{name}.typ")))
        .unwrap_or_else(|_| panic!("missing tests/fixtures/package-smoke/{name}.typ"))
}

/// R11.2 — chaque paquet mis à disposition s'utilise depuis un template (import par le nom).
#[test]
fn every_selected_package_is_usable_from_a_template() {
    let mut failures = Vec::new();
    for package in packages::all().iter().filter(|p| p.is_selected()) {
        let entry = load_template("smoke", &smoke_fixture(package.name()));
        if let Some(reason) = entry.invalid_reason() {
            failures.push(format!("{}: invalid template: {reason}", package.name()));
            continue;
        }
        match compile(entry) {
            Ok(pdf) if pdf.starts_with(b"%PDF") => {}
            Ok(_) => failures.push(format!("{}: output is not a PDF", package.name())),
            Err(e) => failures.push(format!("{}: {e:?}", package.name())),
        }
    }
    assert!(failures.is_empty(), "selected packages failing:\n{}", failures.join("\n"));
}

/// FR-015 — un rendu utilisant un paquet reste déterministe.
#[test]
fn rendering_with_a_package_is_deterministic() {
    let entry = load_template("determinism", &smoke_fixture("cetz"));
    let first = compile(entry.clone()).unwrap();
    let second = compile(entry).unwrap();
    assert!(first == second, "two renders of the same template differ");
}

/// Imports littéraux `@…` d'une source Typst.
fn literal_package_imports(text: &str) -> Vec<String> {
    fn walk(node: &SyntaxNode, found: &mut Vec<String>) {
        let source = node
            .cast::<ast::ModuleImport>()
            .map(|import| import.source())
            .or_else(|| node.cast::<ast::ModuleInclude>().map(|include| include.source()));
        if let Some(ast::Expr::Str(string)) = source {
            let value = string.get();
            if value.starts_with('@') {
                found.push(value.to_string());
            }
        }
        for child in node.children() {
            walk(child, found);
        }
    }
    let mut found = Vec::new();
    walk(&typst::syntax::parse(text), &mut found);
    found
}

/// R11.3 — tout `@preview/…` importé par le code d'un paquet est lui-même intégré.
#[test]
fn bundled_set_is_closed_under_imports() {
    const SKIPPED_DIRS: [&str; 5] = ["tests/", "docs/", "examples/", "gallery/", "template/"];
    let mut missing = BTreeSet::new();
    for package in packages::all() {
        for path in package.paths().filter(|p| p.ends_with(".typ")) {
            if SKIPPED_DIRS.iter().any(|dir| path.starts_with(dir)) {
                continue;
            }
            let text = std::str::from_utf8(package.file(path).unwrap()).unwrap();
            for import in literal_package_imports(text) {
                let Ok(spec) = import.parse::<PackageSpec>() else {
                    continue;
                };
                if packages::get(&spec).is_none() {
                    missing.insert(format!("{} ({}/{path})", spec, package.spec()));
                }
            }
        }
    }
    assert!(missing.is_empty(), "imports not bundled:\n{}", missing.into_iter().collect::<Vec<_>>().join("\n"));
}
