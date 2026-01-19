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
                pkg.namespace.as_str(),
                pkg.name.as_str(),
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
}

impl FileResolver for EmbeddedResolver {
    fn resolve_binary(&self, id: FileId) -> FileResult<Cow<'_, Bytes>> {
        let compressed = self.lookup(id).ok_or_else(|| not_found(id))?;

        // Decompress on access (lazy decompression)
        let data = decompress(compressed)
            .map_err(|_| FileError::Other(Some("Decompression failed".into())))?;

        Ok(Cow::Owned(Bytes::new(data)))
    }

    fn resolve_source(&self, id: FileId) -> FileResult<Cow<'_, Source>> {
        let compressed = self.lookup(id).ok_or_else(|| not_found(id))?;

        // Decompress on access
        let bytes = decompress(compressed)
            .map_err(|_| FileError::Other(Some("Decompression failed".into())))?;

        let source = bytes_to_source(id, &bytes)?;
        Ok(Cow::Owned(source))
    }
}

/// Recursively collect files from include_dir with path prefix tracking
fn collect_files(
    dir: &'static Dir<'static>,
    prefix: &str,
    map: &mut HashMap<String, &'static [u8]>,
) {
    for file in dir.files() {
        let file_path = file.path().display().to_string().replace('\\', "/");
        let full_path = if prefix.is_empty() {
            file_path
        } else {
            format!("{}/{}", prefix, file_path)
        };
        map.insert(full_path, file.contents());
    }

    for subdir in dir.dirs() {
        let subdir_name = subdir.path().display().to_string().replace('\\', "/");
        let new_prefix = if prefix.is_empty() {
            subdir_name
        } else {
            format!("{}/{}", prefix, subdir_name)
        };
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
