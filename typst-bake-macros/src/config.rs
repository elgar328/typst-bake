//! Parse Cargo.toml metadata

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Read and parse the Cargo.toml in the given manifest directory.
fn read_manifest(manifest_dir: &Path) -> Result<toml::Table, String> {
    let cargo_toml_path = manifest_dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)
        .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;
    content
        .parse()
        .map_err(|e| format!("Failed to parse Cargo.toml: {}", e))
}

/// Get a value from [package.metadata.typst-bake] section.
fn get_metadata_value<'a>(manifest: &'a toml::Table, key: &str) -> Option<&'a toml::Value> {
    manifest
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("typst-bake"))
        .and_then(|t| t.get(key))
}

/// Get a string value from [package.metadata.typst-bake] section.
fn get_metadata_str<'a>(manifest: &'a toml::Table, key: &str) -> Option<&'a str> {
    get_metadata_value(manifest, key).and_then(|v| v.as_str())
}

/// Resolve a path relative to the manifest directory (absolute paths pass through).
fn resolve_path(manifest_dir: &Path, path: &str) -> PathBuf {
    if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        manifest_dir.join(path)
    }
}

/// Shared logic for resolving a config directory from env var or Cargo.toml metadata.
///
/// Priority:
/// 1. Environment variable (`env_var`)
/// 2. Cargo.toml `[package.metadata.typst-bake]` key (`metadata_key`)
fn get_config_dir(
    env_var: &str,
    metadata_key: &str,
    not_configured_msg: &str,
    dir_kind: &str,
) -> Result<PathBuf, String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| "CARGO_MANIFEST_DIR not set")?;
    let manifest_dir = Path::new(&manifest_dir);

    // Priority 1: Environment variable
    let path = if let Ok(dir) = env::var(env_var) {
        resolve_path(manifest_dir, &dir)
    } else {
        // Priority 2: Cargo.toml metadata
        let manifest = read_manifest(manifest_dir)?;
        let dir = get_metadata_str(&manifest, metadata_key)
            .ok_or_else(|| not_configured_msg.to_string())?;
        resolve_path(manifest_dir, dir)
    };

    if !path.exists() {
        return Err(format!(
            "{} directory does not exist: {}",
            dir_kind,
            path.display()
        ));
    }

    Ok(path)
}

/// Get template directory path.
///
/// Priority:
/// 1. Environment variable TYPST_TEMPLATE_DIR
/// 2. Cargo.toml [package.metadata.typst-bake] template-dir
pub fn get_template_dir() -> Result<PathBuf, String> {
    get_config_dir(
        "TYPST_TEMPLATE_DIR",
        "template-dir",
        "Template directory not configured.\n\n\
            Add to your Cargo.toml:\n\n\
            [package.metadata.typst-bake]\n\
            template-dir = \"./templates\"\n\n\
            Or set environment variable:\n\
            export TYPST_TEMPLATE_DIR=./templates",
        "Template",
    )
}

/// Check if cache refresh is needed
pub fn should_refresh_cache() -> bool {
    env::var("TYPST_BAKE_REFRESH").is_ok()
}

/// Get fonts directory path.
///
/// Priority:
/// 1. Environment variable TYPST_FONTS_DIR
/// 2. Cargo.toml [package.metadata.typst-bake] fonts-dir
///
/// At least one font file (.ttf, .otf, .ttc) must exist.
pub fn get_fonts_dir() -> Result<PathBuf, String> {
    let path = get_config_dir(
        "TYPST_FONTS_DIR",
        "fonts-dir",
        "Fonts directory not configured.\n\n\
            Add to your Cargo.toml:\n\n\
            [package.metadata.typst-bake]\n\
            template-dir = \"./templates\"\n\
            fonts-dir = \"./fonts\"\n\n\
            Or set environment variable:\n\
            export TYPST_FONTS_DIR=./fonts",
        "Fonts",
    )?;

    // Check for at least one font file
    let has_fonts = fs::read_dir(&path)
        .map_err(|e| format!("Failed to read fonts directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .any(|entry| is_font_file(&entry.path()));

    if !has_fonts {
        return Err(format!(
            "No font files found in fonts directory: {}\n\n\
            Supported formats: .ttf, .otf, .ttc",
            path.display()
        ));
    }

    Ok(path)
}

/// Check if a path refers to a hidden file or directory (name starts with '.').
pub fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}

/// Check if file is a font file.
pub fn is_font_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext.to_lowercase().as_str(), "ttf" | "otf" | "ttc")
}

const ZSTD_LEVEL_MIN: i32 = 1;
const ZSTD_LEVEL_MAX: i32 = 22;
const ZSTD_LEVEL_DEFAULT: i32 = 19;

/// Get compression level.
///
/// Priority:
/// 1. Environment variable TYPST_BAKE_COMPRESSION_LEVEL
/// 2. Cargo.toml [package.metadata.typst-bake] compression-level
/// 3. Default: 19
pub fn get_compression_level() -> i32 {
    // Priority 1: Environment variable
    if let Ok(val) = env::var("TYPST_BAKE_COMPRESSION_LEVEL") {
        if let Ok(level) = val.parse::<i32>() {
            return level.clamp(ZSTD_LEVEL_MIN, ZSTD_LEVEL_MAX);
        }
    }

    // Priority 2: Cargo.toml metadata
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        if let Ok(manifest) = read_manifest(Path::new(&manifest_dir)) {
            if let Some(level) =
                get_metadata_value(&manifest, "compression-level").and_then(|v| v.as_integer())
            {
                return (level as i32).clamp(ZSTD_LEVEL_MIN, ZSTD_LEVEL_MAX);
            }
        }
    }

    ZSTD_LEVEL_DEFAULT
}

/// Get the compression cache directory.
///
/// Returns `target/typst-bake-cache/{CARGO_PKG_NAME}/`.
/// Falls back to `dirs::cache_dir()/typst-bake/compression-cache/{CARGO_PKG_NAME}/`
/// if the target directory cannot be determined.
pub fn get_compression_cache_dir() -> Result<PathBuf, String> {
    let pkg_name = env::var("CARGO_PKG_NAME").map_err(|_| "CARGO_PKG_NAME not set".to_string())?;

    // 1. CARGO_TARGET_DIR environment variable
    if let Ok(target_dir) = env::var("CARGO_TARGET_DIR") {
        return Ok(PathBuf::from(target_dir)
            .join("typst-bake-cache")
            .join(&pkg_name));
    }

    let manifest_dir =
        env::var("CARGO_MANIFEST_DIR").map_err(|_| "CARGO_MANIFEST_DIR not set".to_string())?;
    let manifest_dir = Path::new(&manifest_dir);

    // 2. CARGO_MANIFEST_DIR/target/ (standalone project)
    let local_target = manifest_dir.join("target");
    if local_target.is_dir() {
        return Ok(local_target.join("typst-bake-cache").join(&pkg_name));
    }

    // 3. Walk up from CARGO_MANIFEST_DIR to find target/ (workspace)
    let mut dir = manifest_dir.parent();
    while let Some(d) = dir {
        let candidate = d.join("target");
        if candidate.is_dir() {
            return Ok(candidate.join("typst-bake-cache").join(&pkg_name));
        }
        dir = d.parent();
    }

    // 4. Fallback: dirs::cache_dir()
    let cache_base =
        dirs::cache_dir().ok_or_else(|| "Could not determine cache directory".to_string())?;
    Ok(cache_base
        .join("typst-bake")
        .join("compression-cache")
        .join(&pkg_name))
}
