//! Imports de paquets d'un template : `#import "@preview/<nom>"`, sans version.
//!
//! Résolus une seule fois, au chargement : chaque import est réécrit avec la version installée
//! (`@preview/zero` → `@preview/zero:0.7.1`) dans l'instantané, ce qui ne laisse au rendu qu'une
//! correspondance exacte. Tout autre import est une erreur. Contrat :
//! specs/002-typst-packages/contracts/template-imports.md.

use typst::syntax::{SyntaxKind, SyntaxNode, ast, is_ident};

use crate::packages;

/// Import incorrect, localisé pour l'auteur du template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportError {
    pub file: String,
    /// Ligne, à partir de 1.
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.message)
    }
}

/// Chaîne d'import littérale trouvée dans la source.
struct Literal {
    /// Plage d'octets du littéral, guillemets compris.
    range: std::ops::Range<usize>,
    value: String,
}

/// Résout les imports de paquets de `text` (fichier `path` du template).
///
/// Renvoie le texte réécrit, `None` s'il n'y a aucun import de paquet, ou la liste des imports
/// incorrects.
pub fn resolve_imports(path: &str, text: &str) -> Result<Option<String>, Vec<ImportError>> {
    let mut literals = Vec::new();
    collect(&typst::syntax::parse(text), 0, &mut literals);
    if literals.is_empty() {
        return Ok(None);
    }

    let mut replacements = Vec::new();
    let mut errors = Vec::new();
    for literal in &literals {
        match resolve(&literal.value) {
            Ok(spec) => replacements.push((literal.range.clone(), format!("\"{spec}\""))),
            Err(message) => errors.push(ImportError {
                file: path.to_owned(),
                line: text[..literal.range.start].matches('\n').count() + 1,
                message,
            }),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut rewritten = text.to_owned();
    for (range, replacement) in replacements.into_iter().rev() {
        rewritten.replace_range(range, &replacement);
    }
    Ok(Some(rewritten))
}

/// Littéraux `@…` des `import` et `include` ; `offset` = position de `node` dans le texte.
fn collect(node: &SyntaxNode, offset: usize, found: &mut Vec<Literal>) {
    if node.is::<ast::ModuleImport>() || node.is::<ast::ModuleInclude>() {
        // La source est le premier enfant expression ; un import calculé (parenthèses,
        // variable…) n'est pas une chaîne et reste hors de portée (règle 4 du contrat).
        let mut child_offset = offset;
        for child in node.children() {
            if child.kind() == SyntaxKind::Str {
                if let Some(string) = child.cast::<ast::Str>() {
                    let value = string.get();
                    if value.starts_with('@') {
                        found.push(Literal {
                            range: child_offset..child_offset + child.len(),
                            value: value.to_string(),
                        });
                    }
                }
                break;
            }
            if child.kind().is_keyword() || child.kind().is_trivia() {
                child_offset += child.len();
                continue;
            }
            break;
        }
    }
    let mut child_offset = offset;
    for child in node.children() {
        collect(child, child_offset, found);
        child_offset += child.len();
    }
}

/// `@preview/<nom>` → `@preview/<nom>:<version installée>`, ou message d'erreur du contrat.
fn resolve(value: &str) -> Result<String, String> {
    let Some((namespace, rest)) = value[1..].split_once('/') else {
        return Err(format!("invalid package import \"{value}\""));
    };
    if namespace != packages::NAMESPACE {
        return Err(format!("only @preview packages are available: {value}"));
    }
    if let Some((name, _)) = rest.split_once(':') {
        return Err(format!(
            "remove the version: write @preview/{name} (inkpdf uses its installed version)"
        ));
    }
    if !is_ident(rest) {
        return Err(format!("invalid package import \"{value}\""));
    }
    match packages::selected(rest) {
        Some(package) => Ok(format!("@{}/{}:{}", packages::NAMESPACE, rest, package.version())),
        None => Err(format!(
            "package @preview/{rest} is not available in inkpdf (see GET /packages)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(name: &str) -> &'static str {
        packages::selected(name).unwrap().version()
    }

    fn errors(text: &str) -> Vec<String> {
        resolve_imports("main.typ", text)
            .unwrap_err()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn name_only_import_is_rewritten_with_installed_version() {
        let text = "= Titre\n#import \"@preview/zero\": num\n#num(1)\n";
        let rewritten = resolve_imports("main.typ", text).unwrap().unwrap();
        assert_eq!(
            rewritten,
            format!(
                "= Titre\n#import \"@preview/zero:{}\": num\n#num(1)\n",
                version("zero")
            )
        );
    }

    #[test]
    fn several_imports_and_include_are_rewritten() {
        let text = "#import \"@preview/zero\"; #import \"@preview/cetz\" as c\n#include \"@preview/oxifmt\"\n";
        let rewritten = resolve_imports("main.typ", text).unwrap().unwrap();
        assert!(rewritten.contains(&format!("\"@preview/zero:{}\"", version("zero"))));
        assert!(rewritten.contains(&format!("\"@preview/cetz:{}\" as c", version("cetz"))));
        assert!(rewritten.contains(&format!("#include \"@preview/oxifmt:{}\"", version("oxifmt"))));
    }

    #[test]
    fn text_without_package_import_is_untouched() {
        let text = "// #import \"@preview/zero:1.0.0\"\n#import \"parts/x.typ\"\n\"@preview/zero:1.0.0\"\n";
        assert_eq!(resolve_imports("main.typ", text).unwrap(), None);
    }

    #[test]
    fn written_version_is_refused() {
        assert_eq!(
            errors("\n#import \"@preview/zero:0.7.1\": num\n"),
            ["main.typ:2: remove the version: write @preview/zero (inkpdf uses its installed version)"]
        );
    }

    #[test]
    fn unavailable_package_is_refused() {
        assert_eq!(
            errors("#import \"@preview/does-not-exist\"\n"),
            ["main.typ:1: package @preview/does-not-exist is not available in inkpdf (see GET /packages)"]
        );
        // Une dépendance interne n'est pas mise à disposition des templates.
        assert_eq!(errors("#import \"@preview/komet\"").len(), 1);
    }

    #[test]
    fn other_namespace_and_malformed_imports_are_refused() {
        assert_eq!(
            errors("#import \"@local/zero\"\n#import \"@preview/\"\n#import \"@\"\n"),
            [
                "main.typ:1: only @preview packages are available: @local/zero",
                "main.typ:2: invalid package import \"@preview/\"",
                "main.typ:3: invalid package import \"@\"",
            ]
        );
    }

    #[test]
    fn computed_import_is_not_seen() {
        let text = "#import (\"@preview/\" + \"zero\")\n";
        assert_eq!(resolve_imports("main.typ", text).unwrap(), None);
    }
}
