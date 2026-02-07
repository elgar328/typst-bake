//! Build script helpers for automatic rebuild on file changes.
//!
//! By default, `typst-bake` automatically rebuilds when template or font files
//! are **modified**. However, it won't detect when files are **added or removed**.
//!
//! For complete file change detection, add a `build.rs` to your project:
//!
//! ```rust,ignore
//! // build.rs
//! fn main() {
//!     typst_bake::rebuild_if_changed();
//! }
//! ```
//!
//! Then add `build = "build.rs"` to your `Cargo.toml`:
//!
//! ```toml
//! [package]
//! name = "my-project"
//! build = "build.rs"
//! ```

use std::env;
use std::fs;
use std::path::Path;

fn read_manifest(manifest_dir: &Path) -> toml::Table {
    let cargo_toml_path = manifest_dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("Failed to read Cargo.toml");
    content.parse().expect("Failed to parse Cargo.toml")
}

fn get_metadata_str<'a>(manifest: &'a toml::Table, key: &str) -> Option<&'a str> {
    manifest
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("typst-bake"))
        .and_then(|t| t.get(key))
        .and_then(|v| v.as_str())
}

fn resolve_path_string(manifest_dir: &Path, path: &str) -> String {
    if Path::new(path).is_absolute() {
        path.to_string()
    } else {
        manifest_dir.join(path).to_string_lossy().into_owned()
    }
}

/// Emits `cargo:rerun-if-changed` directives for template and font directories.
///
/// This function reads the `template-dir` and `fonts-dir` paths from your
/// `Cargo.toml` metadata and tells Cargo to watch those directories for changes.
///
/// # When to use
///
/// Without this, `typst-bake` detects file **modifications** but not file
/// **additions or removals**. In practice, this is rarely an issue because
/// adding a new file usually requires modifying an existing file to use it.
///
/// Use this function if you want complete coverage for all file changes.
///
/// # Example
///
/// Create a `build.rs` file in your project root:
///
/// ```rust,ignore
/// fn main() {
///     typst_bake::rebuild_if_changed();
/// }
/// ```
///
/// # Panics
///
/// Panics if `CARGO_MANIFEST_DIR` is not set or if `Cargo.toml` cannot be read.
pub fn rebuild_if_changed() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_dir = Path::new(&manifest_dir);
    let manifest = read_manifest(manifest_dir);

    if let Some(template_dir) = get_metadata_str(&manifest, "template-dir") {
        println!(
            "cargo:rerun-if-changed={}",
            resolve_path_string(manifest_dir, template_dir)
        );
    }

    if let Some(fonts_dir) = get_metadata_str(&manifest, "fonts-dir") {
        println!(
            "cargo:rerun-if-changed={}",
            resolve_path_string(manifest_dir, fonts_dir)
        );
    }
}
