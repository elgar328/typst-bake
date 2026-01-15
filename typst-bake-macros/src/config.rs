//! Parse Cargo.toml metadata

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Get template directory path.
///
/// Priority:
/// 1. Environment variable TYPST_TEMPLATE_DIR
/// 2. Cargo.toml [package.metadata.typst-bake] template-dir
pub fn get_template_dir() -> Result<PathBuf, String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| "CARGO_MANIFEST_DIR not set")?;
    let manifest_dir = Path::new(&manifest_dir);

    // Priority 1: Environment variable
    if let Ok(template_dir) = env::var("TYPST_TEMPLATE_DIR") {
        let path = if Path::new(&template_dir).is_absolute() {
            PathBuf::from(&template_dir)
        } else {
            manifest_dir.join(&template_dir)
        };
        return Ok(path);
    }

    // Priority 2: Cargo.toml metadata
    let cargo_toml_path = manifest_dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)
        .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;

    let manifest: toml::Table = content
        .parse()
        .map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?;

    let template_dir = manifest
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("typst-bake"))
        .and_then(|t| t.get("template-dir"))
        .and_then(|d| d.as_str())
        .ok_or_else(|| {
            "Template directory not configured.\n\n\
                Add to your Cargo.toml:\n\n\
                [package.metadata.typst-bake]\n\
                template-dir = \"./templates\"\n\n\
                Or set environment variable:\n\
                export TYPST_TEMPLATE_DIR=./templates"
                .to_string()
        })?;

    let path = if Path::new(template_dir).is_absolute() {
        PathBuf::from(template_dir)
    } else {
        manifest_dir.join(template_dir)
    };

    if !path.exists() {
        return Err(format!(
            "Template directory does not exist: {}",
            path.display()
        ));
    }

    Ok(path)
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
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| "CARGO_MANIFEST_DIR not set")?;
    let manifest_dir = Path::new(&manifest_dir);

    // Priority 1: Environment variable
    let path = if let Ok(fonts_dir) = env::var("TYPST_FONTS_DIR") {
        if Path::new(&fonts_dir).is_absolute() {
            PathBuf::from(&fonts_dir)
        } else {
            manifest_dir.join(&fonts_dir)
        }
    } else {
        // Priority 2: Cargo.toml metadata
        let cargo_toml_path = manifest_dir.join("Cargo.toml");
        let content = fs::read_to_string(&cargo_toml_path)
            .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;

        let manifest: toml::Table = content
            .parse()
            .map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?;

        let fonts_dir = manifest
            .get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get("typst-bake"))
            .and_then(|t| t.get("fonts-dir"))
            .and_then(|d| d.as_str())
            .ok_or_else(|| {
                "Fonts directory not configured.\n\n\
                    Add to your Cargo.toml:\n\n\
                    [package.metadata.typst-bake]\n\
                    template-dir = \"./templates\"\n\
                    fonts-dir = \"./fonts\"\n\n\
                    Or set environment variable:\n\
                    export TYPST_FONTS_DIR=./fonts"
                    .to_string()
            })?;

        if Path::new(fonts_dir).is_absolute() {
            PathBuf::from(fonts_dir)
        } else {
            manifest_dir.join(fonts_dir)
        }
    };

    if !path.exists() {
        return Err(format!(
            "Fonts directory does not exist: {}",
            path.display()
        ));
    }

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

/// Check if file is a font file.
pub fn is_font_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext.to_lowercase().as_str(), "ttf" | "otf" | "ttc")
}
