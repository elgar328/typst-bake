//! Scan .typ files and parse package imports.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use typst_syntax::ast::{Expr, Markup};
use typst_syntax::Source;
use walkdir::WalkDir;

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
pub fn parse_packages_from_source(content: &str) -> Result<Vec<PackageSpec>, String> {
    let source = Source::detached(content);
    let root_node = source.root();

    let Some(root): Option<Markup> = root_node.cast() else {
        return Ok(Vec::new()); // Not a valid markup, skip
    };

    let mut packages = Vec::new();

    for expr in root.exprs() {
        let Expr::ModuleImport(import) = expr else {
            continue;
        };
        let Expr::Str(str_node) = import.source() else {
            continue;
        };
        packages.extend(parse_package_specifier(&str_node.get()));
    }

    Ok(packages)
}

/// Extract all package imports from `.typ` files in a directory.
pub fn extract_packages(dir: &Path) -> Vec<PackageSpec> {
    let mut packages = HashSet::new();

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "typ"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            match parse_packages_from_source(&content) {
                Ok(found_packages) => {
                    packages.extend(found_packages);
                }
                Err(e) => {
                    // Log but don't fail - graceful degradation
                    eprintln!("Warning: Failed to parse {}: {e}", entry.path().display());
                }
            }
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

    #[test]
    fn test_parse_packages_from_source() {
        let content = r#"#import "@preview/cetz:0.3.2""#;
        let packages = parse_packages_from_source(content).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "cetz");
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
