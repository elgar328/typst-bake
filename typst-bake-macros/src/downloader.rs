//! Package download and cache management.

use crate::scanner::{extract_packages, PackageSpec, ResolvedPackage};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Get the cache directory for downloaded packages.
///
/// Resolution order:
/// 1. `TYPST_PACKAGE_CACHE_PATH` environment variable
/// 2. `{system-cache-dir}/typst/packages/`
pub fn get_cache_dir() -> Result<PathBuf, String> {
    let cache_dir = if let Ok(env_path) = std::env::var("TYPST_PACKAGE_CACHE_PATH") {
        PathBuf::from(env_path)
    } else {
        dirs::cache_dir()
            .ok_or("Could not determine system cache directory".to_owned())?
            .join("typst")
            .join("packages")
    };

    fs::create_dir_all(&cache_dir).map_err(|e| format!("Failed to create cache directory: {e}"))?;

    Ok(cache_dir)
}

/// Get the data directory for locally installed packages.
///
/// Resolution order:
/// 1. `TYPST_PACKAGE_PATH` environment variable
/// 2. `{system-data-dir}/typst/packages/`
///
/// Returns `None` if neither is available. Does not create the directory.
pub fn get_data_dir() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("TYPST_PACKAGE_PATH") {
        Some(PathBuf::from(env_path))
    } else {
        dirs::data_dir().map(|d| d.join("typst").join("packages"))
    }
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

/// Resolve packages using a multi-step lookup and download if needed.
///
/// For each package, resolution follows this priority:
/// 1. Local data directory (e.g. `@local` packages)
/// 2. Cache directory (previously downloaded)
/// 3. Download from Typst Universe (only `@preview` packages)
pub fn resolve_packages(
    packages: &[PackageSpec],
    data_dir: Option<&Path>,
    cache_dir: &Path,
    refresh: bool,
) -> Result<Vec<ResolvedPackage>, String> {
    if packages.is_empty() {
        return Ok(Vec::new());
    }

    let mut queue: VecDeque<_> = packages.iter().cloned().collect();
    let mut seen: HashSet<PackageSpec> = HashSet::new();
    let mut resolved = Vec::new();
    let mut failed_packages = Vec::new();

    while let Some(pkg) = queue.pop_front() {
        if !seen.insert(pkg.clone()) {
            continue;
        }

        // 1. Check local data directory
        if let Some(data) = data_dir {
            let local_path = pkg.package_dir(data);
            if local_path.exists() {
                eprintln!("  Local: {pkg}");
                for dep in resolve_dependencies(&local_path) {
                    queue.push_back(dep);
                }
                resolved.push(ResolvedPackage {
                    spec: pkg,
                    path: local_path,
                });
                continue;
            }
        }

        // 2. Check cache directory
        let cache_path = pkg.package_dir(cache_dir);
        if cache_path.exists() && !refresh {
            eprintln!("  Cached: {pkg}");
            for dep in resolve_dependencies(&cache_path) {
                queue.push_back(dep);
            }
            resolved.push(ResolvedPackage {
                spec: pkg,
                path: cache_path,
            });
            continue;
        }

        // 3. Download from Universe (only for downloadable namespaces)
        if pkg.is_downloadable() {
            eprintln!("  Downloading: {pkg}");
            if let Err(e) = download_and_extract(&pkg.download_url(), &cache_path) {
                eprintln!("  ✗ Failed: {pkg}: {e}");
                failed_packages.push(format!("{pkg}: download failed: {e}"));
                continue;
            }
            eprintln!("  ✓ {pkg}");
            for dep in resolve_dependencies(&cache_path) {
                queue.push_back(dep);
            }
            resolved.push(ResolvedPackage {
                spec: pkg,
                path: cache_path,
            });
            continue;
        }

        // 4. Not found anywhere
        let mut searched = Vec::new();
        if let Some(data) = data_dir {
            searched.push(pkg.package_dir(data));
        }
        searched.push(cache_path);
        let paths: String = searched
            .iter()
            .map(|p| format!("      {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");
        failed_packages.push(format!("{pkg}: not found, searched:\n{paths}"));
    }

    if !failed_packages.is_empty() {
        return Err(format!(
            "Failed to resolve {} package(s):\n  - {}",
            failed_packages.len(),
            failed_packages.join("\n  - ")
        ));
    }

    Ok(resolved)
}

/// Download and extract a tar.gz archive from a URL.
///
/// Uses a per-package file lock to prevent race conditions when multiple
/// processes (e.g. parallel cargo builds) try to download the same package.
fn download_and_extract(url: &str, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure parent directory exists for the lock file
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    // Acquire per-package exclusive lock (other processes block here)
    let lock_path = dest.with_file_name(format!(
        "{}.lock",
        dest.file_name().unwrap().to_string_lossy()
    ));
    let mut lock = fd_lock::RwLock::new(fs::File::create(&lock_path)?);
    let _guard = lock.write()?;

    // After acquiring lock: check if another process already completed
    if dest.exists() {
        return Ok(());
    }

    // Download
    let response = ureq::get(url).call()?;
    let (_, body) = response.into_parts();
    let mut bytes = Vec::new();
    body.into_reader().read_to_end(&mut bytes)?;

    // Extract atomically
    extract_tar_gz(&bytes, dest)?;
    Ok(())
    // _guard dropped here → lock released
}

/// Extract a tar.gz archive to the destination directory atomically.
///
/// Extracts into a PID-stamped temp directory first, then renames to the
/// final destination. This ensures other processes never see a half-extracted
/// package directory.
fn extract_tar_gz(bytes: &[u8], dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use binstall_tar::Archive;
    use flate2::read::GzDecoder;

    // PID-based unique temp directory (same parent = same filesystem → rename is atomic)
    let temp = dest.with_file_name(format!(
        "{}.tmp.{}",
        dest.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));

    // Clean up any leftover temp directory and create fresh
    if temp.exists() {
        fs::remove_dir_all(&temp)?;
    }
    fs::create_dir_all(&temp)?;

    // Extract into temp directory (clean up on failure)
    let gz = GzDecoder::new(bytes);
    let mut archive = Archive::new(gz);
    if let Err(e) = archive.unpack(&temp) {
        let _ = fs::remove_dir_all(&temp);
        return Err(e.into());
    }

    // Remove existing dest (refresh case)
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }

    // Atomic rename
    fs::rename(&temp, dest)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cache_dir() {
        let cache_dir = get_cache_dir().unwrap();
        assert!(cache_dir.ends_with("typst/packages"));
    }

    #[test]
    fn test_get_data_dir() {
        let data_dir = get_data_dir();
        if let Some(dir) = data_dir {
            assert!(dir.ends_with("typst/packages"));
        }
    }
}
