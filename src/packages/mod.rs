//! Paquets Typst intégrés au binaire (specs/002-typst-packages).
//!
//! La liste vient de `packages/lock.toml` ; `build.rs` vérifie les archives et génère une table
//! statique. Un paquet `selected` est mis à disposition des templates (une version par nom) ;
//! les autres ne servent qu'aux imports internes des paquets.

use std::collections::HashMap;
use std::sync::LazyLock;

use typst::foundations::Bytes;
use typst::syntax::package::PackageSpec;

/// Namespace des paquets intégrés (Typst Universe).
pub const NAMESPACE: &str = "preview";

/// Entrée générée par `build.rs`.
pub(crate) struct BundledPackageDef {
    name: &'static str,
    version: &'static str,
    description: &'static str,
    license: &'static str,
    selected: bool,
    entrypoint: &'static str,
    files: &'static [(&'static str, &'static [u8])],
}

include!(concat!(env!("OUT_DIR"), "/bundled_packages.rs"));

/// Paquet intégré, prêt à être servi au moteur.
pub struct BundledPackage {
    def: &'static BundledPackageDef,
    spec: PackageSpec,
    /// Chemin relatif (séparé par `/`) → contenu, sans copie des données statiques.
    files: HashMap<&'static str, Bytes>,
}

impl BundledPackage {
    pub fn name(&self) -> &'static str {
        self.def.name
    }

    pub fn version(&self) -> &'static str {
        self.def.version
    }

    pub fn description(&self) -> &'static str {
        self.def.description
    }

    pub fn license(&self) -> &'static str {
        self.def.license
    }

    /// Vrai si cette version est celle mise à disposition des templates.
    pub fn is_selected(&self) -> bool {
        self.def.selected
    }

    pub fn entrypoint(&self) -> &'static str {
        self.def.entrypoint
    }

    /// `@preview/<name>:<version>`.
    pub fn spec(&self) -> &PackageSpec {
        &self.spec
    }

    /// Fichier du paquet par chemin relatif (sans `/` initial).
    pub fn file(&self, path: &str) -> Option<&Bytes> {
        self.files.get(path)
    }

    /// Chemins des fichiers du paquet.
    pub fn paths(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.def.files.iter().map(|(path, _)| *path)
    }
}

static PACKAGES: LazyLock<Vec<BundledPackage>> = LazyLock::new(|| {
    BUNDLED
        .iter()
        .map(|def| BundledPackage {
            def,
            spec: format!("@{NAMESPACE}/{}:{}", def.name, def.version)
                .parse()
                .expect("build.rs validates names and versions"),
            files: def
                .files
                .iter()
                .map(|(path, data)| (*path, Bytes::new(*data)))
                .collect(),
        })
        .collect()
});

/// Tous les paquets intégrés, triés par nom puis version.
pub fn all() -> &'static [BundledPackage] {
    &PACKAGES
}

/// Paquet correspondant exactement à `spec` (namespace, nom, version).
pub fn get(spec: &PackageSpec) -> Option<&'static BundledPackage> {
    if spec.namespace != NAMESPACE {
        return None;
    }
    all().iter().find(|p| p.spec == *spec)
}

/// Version mise à disposition des templates pour `name`.
pub fn selected(name: &str) -> Option<&'static BundledPackage> {
    all().iter().find(|p| p.is_selected() && p.name() == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(s: &str) -> PackageSpec {
        s.parse().unwrap()
    }

    #[test]
    fn validated_list_is_bundled() {
        assert_eq!(all().len(), 28);
        assert_eq!(all().iter().filter(|p| p.is_selected()).count(), 20);
    }

    #[test]
    fn selected_version_is_the_one_offered_to_templates() {
        assert_eq!(selected("zero").unwrap().version(), "0.7.1");
        assert!(selected("komet").is_none(), "dependency only");
        assert!(selected("does-not-exist").is_none());
    }

    #[test]
    fn get_matches_exactly() {
        assert!(get(&spec("@preview/zero:0.6.1")).is_some());
        assert!(get(&spec("@preview/zero:0.5.0")).is_none());
        assert!(get(&spec("@local/zero:0.7.1")).is_none());
    }

    #[test]
    fn entrypoint_and_manifest_are_served() {
        for package in all() {
            assert!(package.file(package.entrypoint()).is_some(), "{}", package.name());
            assert!(package.file("typst.toml").is_some(), "{}", package.name());
        }
    }
}
