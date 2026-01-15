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
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = Path::new(&manifest_dir).join("Cargo.toml");

    let content = fs::read_to_string(&manifest_path)
        .expect("Failed to read Cargo.toml");

    let manifest: toml::Table = content
        .parse()
        .expect("Failed to parse Cargo.toml");

    // Get template-dir
    if let Some(template_dir) = manifest
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("typst-bake"))
        .and_then(|t| t.get("template-dir"))
        .and_then(|d| d.as_str())
    {
        let path = if Path::new(template_dir).is_absolute() {
            template_dir.to_string()
        } else {
            Path::new(&manifest_dir)
                .join(template_dir)
                .to_string_lossy()
                .to_string()
        };
        println!("cargo:rerun-if-changed={}", path);
    }

    // Get fonts-dir
    if let Some(fonts_dir) = manifest
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("typst-bake"))
        .and_then(|t| t.get("fonts-dir"))
        .and_then(|d| d.as_str())
    {
        let path = if Path::new(fonts_dir).is_absolute() {
            fonts_dir.to_string()
        } else {
            Path::new(&manifest_dir)
                .join(fonts_dir)
                .to_string_lossy()
                .to_string()
        };
        println!("cargo:rerun-if-changed={}", path);
    }
}
