//! Scan .typ files and parse package imports.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use typst_syntax::ast::{self, Expr};
use typst_syntax::{Source, SyntaxNode};
use walkdir::WalkDir;

use crate::config::is_hidden;

const PACKAGES_BASE_URL: &str = "https://packages.typst.org";

/// A Typst package specifier: `@namespace/name:version`.
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct PackageSpec {
    pub namespace: String,
    pub name: String,
    pub version: String,
}

impl PackageSpec {
    /// Build the on-disk directory path for this package under a base directory.
    pub fn package_dir(&self, base: &Path) -> PathBuf {
        base.join(&self.namespace)
            .join(&self.name)
            .join(&self.version)
    }

    /// Build the download URL for this package archive.
    pub fn download_url(&self) -> String {
        format!(
            "{PACKAGES_BASE_URL}/{}/{}-{}.tar.gz",
            self.namespace, self.name, self.version
        )
    }

    /// Whether this package can be downloaded from the Typst Universe registry.
    ///
    /// Currently only `@preview` packages are hosted on `packages.typst.org`.
    pub fn is_downloadable(&self) -> bool {
        self.namespace == "preview"
    }
}

/// A resolved package: spec paired with its actual on-disk path.
#[derive(Clone, Debug)]
pub struct ResolvedPackage {
    pub spec: PackageSpec,
    pub path: PathBuf,
}

impl std::fmt::Display for PackageSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{}/{}:{}", self.namespace, self.name, self.version)
    }
}

/// Check if a string is a valid package identifier.
fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Check if a string is a valid version specifier.
fn is_valid_version(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_numeric() || c == '.')
}

/// Parse package specifier (`@namespace/name:version`).
pub fn parse_package_specifier(path: &str) -> Option<PackageSpec> {
    let path = path.strip_prefix('@')?;
    let (namespace_str, name_version) = path.split_once('/')?;
    let (name_str, version_str) = name_version.split_once(':')?;

    if !is_valid_identifier(namespace_str)
        || !is_valid_identifier(name_str)
        || !is_valid_version(version_str)
    {
        return None;
    }

    Some(PackageSpec {
        namespace: namespace_str.to_owned(),
        name: name_str.to_owned(),
        version: version_str.to_owned(),
    })
}

/// Extract package imports from Typst source code.
///
/// Finds every `import` and `include` of a string literal anywhere in the file: at the
/// top level, in code blocks, function bodies, and conditionals. Comments and raw text
/// are not code, so imports written there are ignored. Computed paths cannot be known.
pub fn parse_packages_from_source(content: &str) -> Vec<PackageSpec> {
    let source = Source::detached(content);
    let mut packages = Vec::new();
    collect_packages(source.root(), &mut packages);
    packages
}

fn collect_packages(node: &SyntaxNode, packages: &mut Vec<PackageSpec>) {
    let target = if let Some(import) = node.cast::<ast::ModuleImport>() {
        Some(import.source())
    } else {
        node.cast::<ast::ModuleInclude>()
            .map(|include| include.source())
    };
    if let Some(Expr::Str(path)) = target {
        packages.extend(parse_package_specifier(&path.get()));
    }
    for child in node.children() {
        collect_packages(child, packages);
    }
}

/// Extract all package imports from `.typ` files in a directory.
///
/// Like embedding, this follows symlinks and skips hidden entries, so a file that is not
/// embedded does not pull in packages.
pub fn extract_packages(dir: &Path) -> Vec<PackageSpec> {
    let mut packages = HashSet::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        // The root itself may look hidden (`template-dir = "."`).
        .filter_entry(|e| e.depth() == 0 || !is_hidden(e.path()))
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "typ"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            packages.extend(parse_packages_from_source(&content));
        }
    }

    packages.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_specifier_valid() {
        assert_eq!(
            parse_package_specifier("@preview/cetz:0.3.2"),
            Some(PackageSpec {
                namespace: "preview".to_owned(),
                name: "cetz".to_owned(),
                version: "0.3.2".to_owned(),
            })
        );
    }

    #[test]
    fn test_parse_package_specifier_invalid() {
        assert_eq!(parse_package_specifier("preview/cetz:0.3.2"), None);
        assert_eq!(parse_package_specifier("@preview"), None);
        assert_eq!(parse_package_specifier("@preview/cetz"), None);
    }

    fn names(content: &str) -> Vec<String> {
        let mut names: Vec<_> = parse_packages_from_source(content)
            .into_iter()
            .map(|p| p.name)
            .collect();
        names.sort();
        names
    }

    #[test]
    fn test_parse_packages_from_source() {
        assert_eq!(names(r#"#import "@preview/cetz:0.3.2""#), ["cetz"]);
    }

    #[test]
    fn imports_anywhere_in_code_are_found() {
        let content = r#"
#{ import "@preview/a:0.1.0" }
#let f() = { import "@preview/b:0.1.0": x; x }
#if true [ #import "@preview/c:0.1.0" ]
#include "@preview/d:0.1.0"
#import "local.typ"
"#;
        assert_eq!(names(content), ["a", "b", "c", "d"]);
    }

    #[test]
    fn imports_in_comments_and_raw_text_are_ignored() {
        let content = r#"
// #import "@preview/a:0.1.0"
/* #import "@preview/b:0.1.0" */
```typ
#import "@preview/c:0.1.0"
```
"#;
        assert!(names(content).is_empty());
    }

    #[test]
    fn hidden_files_are_not_scanned() {
        let dir = std::env::temp_dir().join(format!("typst-bake-scan-{}", std::process::id()));
        fs::create_dir_all(dir.join(".hidden")).unwrap();
        fs::write(dir.join("main.typ"), r#"#import "@preview/shown:0.1.0""#).unwrap();
        fs::write(dir.join(".hidden/x.typ"), r#"#import "@preview/a:0.1.0""#).unwrap();
        fs::write(dir.join(".x.typ"), r#"#import "@preview/b:0.1.0""#).unwrap();

        let found: Vec<_> = extract_packages(&dir).into_iter().map(|p| p.name).collect();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(found, ["shown"]);
    }

    #[test]
    fn test_is_downloadable() {
        let preview_pkg = PackageSpec {
            namespace: "preview".to_owned(),
            name: "cetz".to_owned(),
            version: "0.3.2".to_owned(),
        };
        assert!(preview_pkg.is_downloadable());

        let local_pkg = PackageSpec {
            namespace: "local".to_owned(),
            name: "mypkg".to_owned(),
            version: "0.1.0".to_owned(),
        };
        assert!(!local_pkg.is_downloadable());
    }
}
