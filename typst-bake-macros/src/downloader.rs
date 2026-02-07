//! Package download and cache management

use crate::scanner::{extract_packages, PackageSpec};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Get system cache directory
pub fn get_cache_dir() -> Result<PathBuf, String> {
    let cache_dir = dirs::cache_dir()
        .ok_or("Could not determine system cache directory".to_owned())?
        .join("typst-bake")
        .join("packages");

    fs::create_dir_all(&cache_dir).map_err(|e| format!("Failed to create cache directory: {e}"))?;

    Ok(cache_dir)
}

/// Resolve dependencies from a downloaded package directory.
///
/// Collects both explicit dependencies (from `typst.toml`) and implicit ones
/// (from `#import` in `.typ` files).
fn resolve_dependencies(pkg_dir: &Path) -> Vec<PackageSpec> {
    let mut deps = Vec::new();

    // 1. Parse typst.toml for explicit dependencies
    let manifest = fs::read_to_string(pkg_dir.join("typst.toml"))
        .ok()
        .and_then(|c| c.parse::<toml::Table>().ok());
    if let Some(table) = manifest
        .as_ref()
        .and_then(|m| m.get("package"))
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_table())
    {
        deps.extend(table.iter().filter_map(|(dep_name, dep_value)| {
            let (dep_ns, dep_ver) = dep_value.as_str()?.split_once(':')?;
            Some(PackageSpec {
                namespace: dep_ns.to_owned(),
                name: dep_name.to_owned(),
                version: dep_ver.to_owned(),
            })
        }));
    }

    // 2. Scan package's .typ files for implicit dependencies
    deps.extend(extract_packages(pkg_dir));

    deps
}

/// Download packages and resolve dependencies.
/// Returns the list of all resolved packages (direct + transitive).
pub fn download_packages(
    packages: &[PackageSpec],
    cache_dir: &Path,
    refresh: bool,
) -> Result<Vec<PackageSpec>, String> {
    if packages.is_empty() {
        return Ok(Vec::new());
    }

    let mut to_download: VecDeque<_> = packages.iter().cloned().collect();
    let mut downloaded: HashSet<PackageSpec> = HashSet::new();
    let mut failed_packages = Vec::new();

    while let Some(pkg) = to_download.pop_front() {
        if !downloaded.insert(pkg.clone()) {
            continue;
        }
        let pkg_dir = pkg.cache_path(cache_dir);

        // Check cache (unless refresh requested)
        if pkg_dir.exists() && !refresh {
            eprintln!("  Cached: {pkg}");
        } else {
            eprintln!("  Downloading: {pkg}");

            if let Err(e) = download_and_extract(&pkg.download_url(), &pkg_dir) {
                eprintln!("  ✗ Failed: {pkg}: {e}");
                failed_packages.push(pkg.to_string());
                continue;
            }
            eprintln!("  ✓ {pkg}");
        }

        for dep in resolve_dependencies(&pkg_dir) {
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

    Ok(downloaded.into_iter().collect())
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
