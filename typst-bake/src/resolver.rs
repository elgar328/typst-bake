//! Embedded file resolver for templates and packages
//!
//! Uses lazy decompression - files are decompressed only when accessed.

use crate::error::Error;
use crate::util::decompress;
use include_dir::Dir;
use std::collections::HashMap;
use typst::diag::{FileError, FileResult};
use typst::foundations::Bytes;
use typst::syntax::{FileId, Source, VirtualPath, VirtualRoot};

/// Resolver for embedded templates and packages.
///
/// Files are stored in compressed form and decompressed lazily on access.
/// Typst's internal caching prevents redundant decompression.
pub struct EmbeddedResolver {
    files: HashMap<String, &'static [u8]>,
    runtime_files: HashMap<String, Bytes>,
}

impl EmbeddedResolver {
    /// Create a new resolver from embedded directories.
    pub fn new(templates: &'static Dir<'static>, packages: &'static Dir<'static>) -> Self {
        let mut files = HashMap::new();

        collect_files(templates, "", &mut files);
        collect_files(packages, "", &mut files);

        Self {
            files,
            runtime_files: HashMap::new(),
        }
    }

    /// Get the file path string from a `FileId`.
    fn get_path(&self, id: FileId) -> String {
        file_id_to_path(id)
    }

    /// Insert a runtime file (uncompressed).
    pub(crate) fn insert_runtime_file(&mut self, path: String, data: Bytes) {
        self.runtime_files.insert(path, data);
    }

    /// The file with the given id as raw bytes, decompressing an embedded file.
    /// Runtime files take priority over embedded files.
    pub(crate) fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = self.get_path(id);

        // Runtime files are stored uncompressed; cloning `Bytes` is cheap.
        if let Some(data) = self.runtime_files.get(&path) {
            return Ok(data.clone());
        }

        let compressed = self
            .files
            .get(&path)
            .copied()
            .ok_or_else(|| not_found(id))?;
        decompress(compressed).map(Bytes::new).map_err(|e| {
            FileError::Other(Some(format!("Decompression failed for {path}: {e}").into()))
        })
    }

    /// The file with the given id as a Typst source.
    pub(crate) fn source(&self, id: FileId) -> FileResult<Source> {
        bytes_to_source(id, &self.file(id)?)
    }
}

/// Convert a Path to a forward-slash string.
fn normalize_path(path: &std::path::Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// Map a `FileId` to its path string used for lookup and diagnostics.
///
/// Package files become `namespace/name/version/vpath`; template files use the
/// virtual path without its leading slash.
pub(crate) fn file_id_to_path(id: FileId) -> String {
    let vpath = id.vpath().get_without_slash();
    match id.root() {
        VirtualRoot::Package(pkg) => {
            format!("{}/{}/{}/{}", pkg.namespace, pkg.name, pkg.version, vpath)
        }
        VirtualRoot::Project => vpath.to_owned(),
    }
}

/// The key a runtime file is stored under: `path` normalized like a Typst path
/// (`./img//a.png` and `img/x/../a.png` are both `img/a.png`), with `\\` read as `/`.
///
/// Rejects an empty or absolute path, and one that escapes the template directory.
pub(crate) fn runtime_key(path: &str) -> Result<String, Error> {
    let path = path.replace('\\', "/");
    if path.starts_with('/') {
        return Err(Error::InvalidFilePath(format!(
            "absolute path not allowed: {path}"
        )));
    }
    let vpath = VirtualPath::new(&path).map_err(|_| {
        Error::InvalidFilePath(format!("path escapes the template directory: {path}"))
    })?;
    let key = vpath.get_without_slash();
    if key.is_empty() {
        return Err(Error::InvalidFilePath(format!(
            "path names no file: {path:?}"
        )));
    }
    Ok(key.to_owned())
}

/// Join a prefix and name with `/`, or return name alone if prefix is empty.
fn join_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}/{name}")
    }
}

/// Recursively collect files from include_dir with path prefix tracking.
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

/// Create a "not found" error.
fn not_found(id: FileId) -> FileError {
    FileError::NotFound(id.vpath().get_without_slash().into())
}

/// Convert bytes to Source, handling UTF-8 BOM.
pub(crate) fn bytes_to_source(id: FileId, bytes: &[u8]) -> FileResult<Source> {
    // Handle UTF-8 BOM
    let text = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        std::str::from_utf8(&bytes[3..])
    } else {
        std::str::from_utf8(bytes)
    };

    let text = text.map_err(|_| FileError::InvalidUtf8)?;
    Ok(Source::new(id, text.to_string()))
}
