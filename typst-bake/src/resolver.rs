//! Embedded file resolver for templates and packages
//!
//! Uses lazy decompression - files are decompressed only when accessed.

use crate::util::decompress;
use include_dir::Dir;
use std::borrow::Cow;
use std::collections::HashMap;
use typst::diag::{FileError, FileResult};
use typst::foundations::Bytes;
use typst::syntax::{FileId, Source};

// Re-export FileResolver trait from typst-as-lib
pub use typst_as_lib::file_resolver::FileResolver;

/// Resolver for embedded templates and packages.
///
/// Files are stored in compressed form and decompressed lazily on access.
/// Typst's internal caching prevents redundant decompression.
pub struct EmbeddedResolver {
    template_files: HashMap<String, &'static [u8]>,
    package_files: HashMap<String, &'static [u8]>,
}

impl EmbeddedResolver {
    /// Create a new resolver from embedded directories
    pub fn new(templates: &'static Dir<'static>, packages: &'static Dir<'static>) -> Self {
        let mut template_files = HashMap::new();
        let mut package_files = HashMap::new();

        collect_files(templates, "", &mut template_files);
        collect_files(packages, "", &mut package_files);

        Self {
            template_files,
            package_files,
        }
    }

    /// Get file path from FileId
    fn get_path(&self, id: FileId) -> String {
        if let Some(pkg) = id.package() {
            // Package file: namespace/name/version/vpath
            format!(
                "{}/{}/{}/{}",
                pkg.namespace,
                pkg.name,
                pkg.version,
                id.vpath().as_rootless_path().display()
            )
        } else {
            // Template file: just vpath
            id.vpath().as_rootless_path().display().to_string()
        }
    }

    /// Look up compressed file bytes
    fn lookup(&self, id: FileId) -> Option<&'static [u8]> {
        let path = self.get_path(id);

        if id.package().is_some() {
            self.package_files.get(&path).copied()
        } else {
            self.template_files.get(&path).copied()
        }
    }

    /// Look up and decompress a file by its FileId.
    fn decompress_file(&self, id: FileId) -> FileResult<Vec<u8>> {
        let compressed = self.lookup(id).ok_or_else(|| not_found(id))?;
        decompress(compressed).map_err(|e| {
            FileError::Other(Some(
                format!("Decompression failed for {}: {e}", self.get_path(id)).into(),
            ))
        })
    }
}

impl FileResolver for EmbeddedResolver {
    fn resolve_binary(&self, id: FileId) -> FileResult<Cow<'_, Bytes>> {
        let data = self.decompress_file(id)?;
        Ok(Cow::Owned(Bytes::new(data)))
    }

    fn resolve_source(&self, id: FileId) -> FileResult<Cow<'_, Source>> {
        let bytes = self.decompress_file(id)?;
        let source = bytes_to_source(id, &bytes)?;
        Ok(Cow::Owned(source))
    }
}

/// Convert a Path to a forward-slash string.
fn normalize_path(path: &std::path::Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// Join a prefix and name with `/`, or return name alone if prefix is empty.
fn join_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", prefix, name)
    }
}

/// Recursively collect files from include_dir with path prefix tracking
fn collect_files(
    dir: &'static Dir<'static>,
    prefix: &str,
    map: &mut HashMap<String, &'static [u8]>,
) {
    for file in dir.files() {
        let full_path = join_path(prefix, &normalize_path(file.path()));
        map.insert(full_path, file.contents());
    }

    for subdir in dir.dirs() {
        let new_prefix = join_path(prefix, &normalize_path(subdir.path()));
        collect_files(subdir, &new_prefix, map);
    }
}

/// Create a "not found" error
fn not_found(id: FileId) -> FileError {
    FileError::NotFound(id.vpath().as_rootless_path().into())
}

/// Convert bytes to Source, handling UTF-8 BOM
fn bytes_to_source(id: FileId, bytes: &[u8]) -> FileResult<Source> {
    // Handle UTF-8 BOM
    let text = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        std::str::from_utf8(&bytes[3..])
    } else {
        std::str::from_utf8(bytes)
    };

    let text = text.map_err(|_| FileError::InvalidUtf8)?;
    Ok(Source::new(id, text.to_string()))
}
