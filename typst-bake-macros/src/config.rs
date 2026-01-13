//! Cargo.toml 메타데이터 파싱

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// 템플릿 디렉토리 경로를 가져옵니다.
///
/// 우선순위:
/// 1. 환경변수 TYPST_TEMPLATE_DIR
/// 2. Cargo.toml의 [package.metadata.typst-bake] template-dir
pub fn get_template_dir() -> Result<PathBuf, String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "CARGO_MANIFEST_DIR not set")?;
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
            format!(
                "Template directory not configured.\n\n\
                Add to your Cargo.toml:\n\n\
                [package.metadata.typst-bake]\n\
                template-dir = \"./templates\"\n\n\
                Or set environment variable:\n\
                export TYPST_TEMPLATE_DIR=./templates"
            )
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

/// 캐시 새로고침 여부 확인
pub fn should_refresh_cache() -> bool {
    env::var("TYPST_BAKE_REFRESH").is_ok()
}
