//! Package download and cache management.

use crate::scanner::{PackageSpec, ResolvedPackage, extract_packages};
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

/// The dependencies of a package in `pkg_dir`.
///
/// Returns the ones its `typst.toml` declares and the ones its `.typ` files import,
/// separately: imports can sit in code the package never runs (benchmarks, tests), so
/// failing to resolve those is not fatal.
fn resolve_dependencies(pkg_dir: &Path) -> (Vec<PackageSpec>, Vec<PackageSpec>) {
    let mut declared = Vec::new();

    let manifest = fs::read_to_string(pkg_dir.join("typst.toml"))
        .ok()
        .and_then(|c| c.parse::<toml::Table>().ok());
    if let Some(table) = manifest
        .as_ref()
        .and_then(|m| m.get("package"))
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_table())
    {
        declared.extend(table.iter().filter_map(|(dep_name, dep_value)| {
            let (dep_ns, dep_ver) = dep_value.as_str()?.split_once(':')?;
            Some(PackageSpec {
                namespace: dep_ns.to_owned(),
                name: dep_name.to_owned(),
                version: dep_ver.to_owned(),
            })
        }));
    }

    (declared, extract_packages(pkg_dir))
}

/// A package waiting to be resolved.
struct Pending {
    spec: PackageSpec,
    /// The package whose sources import it, if that is the only reason to bundle it.
    /// Such a package is skipped with a warning when it cannot be resolved.
    imported_by: Option<PackageSpec>,
}

/// Resolve packages using a multi-step lookup and download if needed.
///
/// For each package, resolution follows this priority:
/// 1. Local data directory (e.g. `@local` packages)
/// 2. Cache directory (previously downloaded)
/// 3. Download from Typst Universe (only `@preview` packages)
///
/// The given packages and the dependencies packages declare must resolve. Packages
/// found only by scanning another package's sources are bundled when they resolve and
/// skipped with a warning otherwise.
pub fn resolve_packages(
    packages: &[PackageSpec],
    data_dir: Option<&Path>,
    cache_dir: &Path,
    refresh: bool,
) -> Result<Vec<ResolvedPackage>, String> {
    let mut queue: VecDeque<_> = packages
        .iter()
        .map(|spec| Pending {
            spec: spec.clone(),
            imported_by: None,
        })
        .collect();
    let mut seen: HashSet<PackageSpec> = HashSet::new();
    let mut skipped: HashSet<PackageSpec> = HashSet::new();
    let mut resolved = Vec::new();
    let mut failed_packages = Vec::new();

    while let Some(Pending {
        spec: pkg,
        imported_by,
    }) = queue.pop_front()
    {
        if seen.contains(&pkg) || (imported_by.is_some() && skipped.contains(&pkg)) {
            continue;
        }

        let path = match locate_package(&pkg, data_dir, cache_dir, refresh) {
            Ok(path) => path,
            Err(reason) => {
                if let Some(parent) = imported_by {
                    eprintln!(
                        "typst-bake: warning: skipping {pkg}, imported by {parent}: {reason}"
                    );
                    skipped.insert(pkg);
                } else {
                    failed_packages.push(format!("{pkg}: {reason}"));
                    seen.insert(pkg);
                }
                continue;
            }
        };

        let (declared, imported) = resolve_dependencies(&path);
        queue.extend(declared.into_iter().map(|spec| Pending {
            spec,
            imported_by: None,
        }));
        queue.extend(imported.into_iter().map(|spec| Pending {
            spec,
            imported_by: Some(pkg.clone()),
        }));
        seen.insert(pkg.clone());
        resolved.push(ResolvedPackage { spec: pkg, path });
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

/// Find a package in the data or cache directory, downloading it if needed.
fn locate_package(
    pkg: &PackageSpec,
    data_dir: Option<&Path>,
    cache_dir: &Path,
    refresh: bool,
) -> Result<PathBuf, String> {
    // 1. Check local data directory
    if let Some(data) = data_dir {
        let local_path = pkg.package_dir(data);
        if local_path.exists() {
            eprintln!("  Local: {pkg}");
            return Ok(local_path);
        }
    }

    // 2. Check cache directory
    let cache_path = pkg.package_dir(cache_dir);
    if cache_path.exists() && !refresh {
        eprintln!("  Cached: {pkg}");
        return Ok(cache_path);
    }

    // 3. Download from Universe (only for downloadable namespaces)
    if pkg.is_downloadable() {
        eprintln!("  Downloading: {pkg}");
        return match download_and_extract(&pkg.download_url(), &cache_path, refresh) {
            Ok(()) => {
                eprintln!("  ✓ {pkg}");
                Ok(cache_path)
            }
            Err(e) => {
                eprintln!("  ✗ Failed: {pkg}: {e}");
                Err(format!("download failed: {e}"))
            }
        };
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
    Err(format!("not found, searched:\n{paths}"))
}

/// Download and extract a tar.gz archive from a URL.
///
/// Uses a per-package file lock to prevent race conditions when multiple
/// processes (e.g. parallel cargo builds) try to download the same package.
fn download_and_extract(
    url: &str,
    dest: &Path,
    refresh: bool,
) -> Result<(), Box<dyn std::error::Error>> {
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
    if dest.exists() && !refresh {
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

    fn spec(name: &str) -> PackageSpec {
        PackageSpec {
            namespace: "local".to_owned(),
            name: name.to_owned(),
            version: "0.1.0".to_owned(),
        }
    }

    #[test]
    fn unresolvable_imports_found_in_packages_are_skipped() {
        let root = std::env::temp_dir().join(format!("typst-bake-deps-{}", std::process::id()));
        let data = root.join("data");
        let pkg = spec("app").package_dir(&data);
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("lib.typ"),
            r#"#let f() = { import "@local/missing:0.1.0" }"#,
        )
        .unwrap();

        let resolved = resolve_packages(&[spec("app")], Some(&data), &root.join("cache"), false);
        // Asked for directly, the same package is an error.
        let direct = resolve_packages(&[spec("missing")], Some(&data), &root.join("cache"), false);
        fs::remove_dir_all(&root).unwrap();

        let names: Vec<_> = resolved.unwrap().into_iter().map(|p| p.spec.name).collect();
        assert_eq!(names, ["app"]);
        assert!(direct.unwrap_err().contains("@local/missing:0.1.0"));
    }

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
