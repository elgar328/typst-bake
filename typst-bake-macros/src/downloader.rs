//! Package download and cache management

use crate::scanner::{extract_packages, PackageSpec};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Get system cache directory
pub fn get_cache_dir() -> Result<PathBuf, String> {
    let cache_dir = dirs::cache_dir()
        .ok_or("Could not determine system cache directory")?
        .join("typst-bake")
        .join("packages");

    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    Ok(cache_dir)
}

/// Download packages and resolve dependencies
pub fn download_packages(
    packages: &[PackageSpec],
    cache_dir: &Path,
    refresh: bool,
) -> Result<(), String> {
    if packages.is_empty() {
        return Ok(());
    }

    let mut to_download = VecDeque::from(packages.to_vec());
    let mut downloaded = HashSet::new();
    let mut failed_packages = Vec::new();

    while let Some((namespace, name, version)) = to_download.pop_front() {
        // Skip if already downloaded
        let pkg_key = format!("{}/{}/{}", namespace, name, version);
        if !downloaded.insert(pkg_key.clone()) {
            continue;
        }

        let pkg_dir = cache_dir.join(&namespace).join(&name).join(&version);

        // Check cache (unless refresh requested)
        if pkg_dir.exists() && !refresh {
            eprintln!("  Cached: @{}/{}/{}", namespace, name, version);
        } else {
            eprintln!("  Downloading: @{}/{}/{}", namespace, name, version);

            let url = format!(
                "https://packages.typst.org/{}/{}-{}.tar.gz",
                namespace, name, version
            );

            match download_and_extract(&url, &pkg_dir) {
                Ok(_) => {
                    eprintln!("  ✓ @{}/{}/{}", namespace, name, version);
                }
                Err(e) => {
                    eprintln!("  ✗ Failed: @{}/{}/{}: {}", namespace, name, version, e);
                    failed_packages.push(pkg_key);
                    continue;
                }
            }
        }

        // Resolve dependencies

        // 1. Parse typst.toml for explicit dependencies
        let toml_path = pkg_dir.join("typst.toml");
        if let Ok(content) = fs::read_to_string(&toml_path) {
            if let Ok(manifest) = content.parse::<toml::Table>() {
                if let Some(deps) = manifest
                    .get("package")
                    .and_then(|p| p.get("dependencies"))
                    .and_then(|d| d.as_table())
                {
                    for (dep_name, dep_value) in deps {
                        if let Some(dep_str) = dep_value.as_str() {
                            if let Some((dep_ns, dep_ver)) = dep_str.split_once(':') {
                                to_download.push_back((
                                    dep_ns.to_string(),
                                    dep_name.clone(),
                                    dep_ver.to_string(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        // 2. Scan package's .typ files for implicit dependencies
        let pkg_deps = extract_packages(&pkg_dir);
        for dep in pkg_deps {
            to_download.push_back(dep);
        }
    }

    if !failed_packages.is_empty() {
        return Err(format!(
            "Failed to download {} package(s):\n  - {}\n\n\
            Please check your internet connection.",
            failed_packages.len(),
            failed_packages.join("\n  - ")
        ));
    }

    Ok(())
}

/// Download and extract tar.gz from URL
fn download_and_extract(url: &str, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let response = ureq::get(url).call()?;
    let (_, body) = response.into_parts();
    let mut bytes = Vec::new();
    body.into_reader().read_to_end(&mut bytes)?;
    extract_tar_gz(&bytes, dest)?;
    Ok(())
}

/// Extract tar.gz archive
fn extract_tar_gz(bytes: &[u8], dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use binstall_tar::Archive;
    use flate2::read::GzDecoder;

    // Remove existing directory if present
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::create_dir_all(dest)?;

    let gz = GzDecoder::new(bytes);
    let mut archive = Archive::new(gz);
    archive.unpack(dest)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cache_dir() {
        let cache_dir = get_cache_dir();
        assert!(cache_dir.is_ok());
    }
}
