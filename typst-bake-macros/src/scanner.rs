//! Scan .typ files and parse package imports

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use typst_syntax::ast::{Expr, Markup};
use typst_syntax::Source;
use walkdir::WalkDir;

/// Package info (namespace, name, version)
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct PackageSpec {
    pub namespace: String,
    pub name: String,
    pub version: String,
}

impl std::fmt::Display for PackageSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{}/{}:{}", self.namespace, self.name, self.version)
    }
}

/// Check if valid identifier
fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Check if valid version string
fn is_valid_version(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_numeric() || c == '.')
}

/// Parse package specifier (@namespace/name:version)
pub fn parse_package_specifier(path: &str) -> Option<PackageSpec> {
    // Package imports start with @
    if !path.starts_with('@') {
        return None;
    }

    let path = &path[1..]; // Remove @

    // Split namespace/name:version
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() != 2 {
        return None;
    }

    let namespace = parts[0].to_string();
    let name_version = parts[1];

    // Split name and version
    let nv_parts: Vec<&str> = name_version.split(':').collect();
    if nv_parts.len() != 2 {
        return None;
    }

    let name = nv_parts[0].to_string();
    let version = nv_parts[1].to_string();

    // Validate format
    if !is_valid_identifier(&namespace)
        || !is_valid_identifier(&name)
        || !is_valid_version(&version)
    {
        return None;
    }

    Some(PackageSpec {
        namespace,
        name,
        version,
    })
}

/// Extract package imports from source code
pub fn parse_packages_from_source(content: &str) -> Result<Vec<PackageSpec>, String> {
    // Parse source into AST
    let source = Source::detached(content);
    let root_node = source.root();

    // Cast SyntaxNode to Markup AST node
    let root: Markup = match root_node.cast() {
        Some(markup) => markup,
        None => return Ok(Vec::new()), // Not a valid markup, skip
    };

    let mut packages = Vec::new();

    // Iterate through all expressions
    for expr in root.exprs() {
        // Look for import expressions
        if let Expr::ModuleImport(import) = expr {
            // Extract the import source
            let source_expr = import.source();

            // Check if source is a string literal
            if let Expr::Str(str_node) = source_expr {
                let import_path = str_node.get();

                // Parse package specifier
                if let Some(pkg) = parse_package_specifier(&import_path) {
                    packages.push(pkg);
                }
            }
        }
    }

    Ok(packages)
}

/// Extract all package imports from directory
pub fn extract_packages(dir: &Path) -> Vec<PackageSpec> {
    let mut packages = HashSet::new();

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "typ"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            match parse_packages_from_source(&content) {
                Ok(found_packages) => {
                    packages.extend(found_packages);
                }
                Err(e) => {
                    // Log but don't fail - graceful degradation
                    eprintln!("Warning: Failed to parse {}: {}", entry.path().display(), e);
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
                namespace: "preview".to_string(),
                name: "cetz".to_string(),
                version: "0.3.2".to_string(),
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
}
