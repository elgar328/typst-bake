//! Embedded file resolver for templates and packages

use include_dir::Dir;
use std::borrow::Cow;
use std::collections::HashMap;
use typst::diag::{FileError, FileResult};
use typst::foundations::Bytes;
use typst::syntax::{FileId, Source};

// Re-export FileResolver trait from typst-as-lib
pub use typst_as_lib::file_resolver::FileResolver;

/// Resolver for embedded templates and packages
pub struct EmbeddedResolver {
    template_files: HashMap<String, &'static [u8]>,
    package_files: HashMap<String, &'static [u8]>,
}

impl EmbeddedResolver {
    /// Create a new resolver from embedded directories
    pub fn new(templates: &'static Dir<'static>, packages: &'static Dir<'static>) -> Self {
        let mut template_files = HashMap::new();
        let mut package_files = HashMap::new();

        collect_files(templates, &mut template_files);
        collect_files(packages, &mut package_files);

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

    /// Look up file bytes
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
        self.lookup(id)
            .map(|bytes| Cow::Owned(Bytes::new(bytes)))
            .ok_or_else(|| not_found(id))
    }

    fn resolve_source(&self, id: FileId) -> FileResult<Cow<'_, Source>> {
        let bytes = self.lookup(id).ok_or_else(|| not_found(id))?;
        let source = bytes_to_source(id, bytes)?;
        Ok(Cow::Owned(source))
    }
}

/// Recursively collect files from include_dir
fn collect_files(dir: &'static Dir<'static>, map: &mut HashMap<String, &'static [u8]>) {
    // file.path() already returns the full path from the include_dir root
    for file in dir.files() {
        let path = file.path().display().to_string();
        // Normalize path separators
        let path = path.replace('\\', "/");
        map.insert(path, file.contents());
    }

    for subdir in dir.dirs() {
        collect_files(subdir, map);
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
