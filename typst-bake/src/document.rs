//! Self-contained document for Typst template rendering.

use crate::error::{Error, Result};
use crate::resolver::{normalize_file_path, EmbeddedResolver};
use crate::stats::EmbedStats;
use crate::util::decompress;
use include_dir::{Dir, File};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use typst::foundations::Dict;
use typst::layout::PagedDocument;
use typst_as_lib::{TypstAsLibError, TypstEngine};

/// A fully self-contained document ready for rendering.
///
/// Created by the [`document!`](crate::document!) macro with embedded templates, fonts,
/// and packages. All resources are compressed with zstd and decompressed lazily at runtime.
pub struct Document {
    templates: &'static Dir<'static>,
    packages: &'static Dir<'static>,
    fonts: &'static Dir<'static>,
    entry: &'static str,
    inputs: Mutex<Option<Dict>>,
    runtime_files: Mutex<HashMap<String, Vec<u8>>>,
    stats: EmbedStats,
    compiled_cache: Mutex<Option<PagedDocument>>,
}

impl Document {
    /// Internal constructor used by the macro.
    /// Do not use directly.
    #[doc(hidden)]
    pub fn __new(
        templates: &'static Dir<'static>,
        packages: &'static Dir<'static>,
        fonts: &'static Dir<'static>,
        entry: &'static str,
        stats: EmbedStats,
    ) -> Self {
        Self {
            templates,
            packages,
            fonts,
            entry,
            inputs: Mutex::new(None),
            runtime_files: Mutex::new(HashMap::new()),
            stats,
            compiled_cache: Mutex::new(None),
        }
    }

    fn lock_inputs(&self) -> MutexGuard<'_, Option<Dict>> {
        self.inputs.lock().expect("lock poisoned")
    }

    fn lock_runtime_files(&self) -> MutexGuard<'_, HashMap<String, Vec<u8>>> {
        self.runtime_files.lock().expect("lock poisoned")
    }

    fn lock_cache(&self) -> MutexGuard<'_, Option<PagedDocument>> {
        self.compiled_cache.lock().expect("lock poisoned")
    }

    /// Add input data to the document.
    ///
    /// Define your data structs using the derive macros:
    /// - **Top-level struct**: Use both [`IntoValue`](crate::IntoValue) and [`IntoDict`](crate::IntoDict)
    /// - **Nested structs**: Use [`IntoValue`](crate::IntoValue) only
    ///
    /// In `.typ` files, access the data via `sys.inputs`:
    /// ```typ
    /// #import sys: inputs
    /// = #inputs.title
    /// ```
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use typst_bake::{IntoValue, IntoDict};
    ///
    /// #[derive(IntoValue, IntoDict)]  // Top-level: both macros
    /// struct Inputs {
    ///     title: String,
    ///     products: Vec<Product>,
    /// }
    ///
    /// #[derive(IntoValue)]  // Nested: IntoValue only
    /// struct Product {
    ///     name: String,
    ///     price: f64,
    /// }
    ///
    /// let inputs = Inputs {
    ///     title: "Catalog".to_string(),
    ///     products: vec![
    ///         Product { name: "Apple".to_string(), price: 1.50 },
    ///     ],
    /// };
    ///
    /// let pdf = typst_bake::document!("main.typ")
    ///     .with_inputs(inputs)
    ///     .to_pdf()?;
    /// ```
    pub fn with_inputs<T: Into<Dict>>(self, inputs: T) -> Self {
        *self.lock_inputs() = Some(inputs.into());
        *self.lock_cache() = None;
        self
    }

    /// Add or replace a runtime file at the given path.
    ///
    /// The file becomes available to Typst templates via `#image("path")`,
    /// `#read("path")`, etc. Runtime files take priority over embedded files
    /// with the same path.
    ///
    /// # Errors
    /// Returns [`Error::InvalidFilePath`] if the path is empty, absolute, or
    /// contains `..` segments.
    ///
    /// # Example
    /// ```rust,ignore
    /// let pdf = typst_bake::document!("main.typ")
    ///     .add_file("images/chart.png", chart_bytes)?
    ///     .to_pdf()?;
    /// ```
    pub fn add_file(self, path: impl Into<String>, data: impl Into<Vec<u8>>) -> Result<Self> {
        let raw = path.into();
        let normalized = normalize_file_path(&raw);

        if normalized.is_empty() {
            return Err(Error::InvalidFilePath("path is empty".into()));
        }
        if normalized.starts_with('/') {
            return Err(Error::InvalidFilePath(format!(
                "absolute path not allowed: {normalized}"
            )));
        }
        if normalized.split('/').any(|s| s == "..") {
            return Err(Error::InvalidFilePath(format!(
                "path with '..' not allowed: {normalized}"
            )));
        }

        self.lock_runtime_files().insert(normalized, data.into());
        *self.lock_cache() = None;
        Ok(self)
    }

    /// Check if a file exists at the given path.
    ///
    /// Checks both embedded (compile-time) and runtime files.
    pub fn has_file(&self, path: impl AsRef<str>) -> bool {
        let normalized = normalize_file_path(path.as_ref());

        // Check runtime files first.
        if self.lock_runtime_files().contains_key(&normalized) {
            return true;
        }

        // Check embedded templates.
        if find_entry(self.templates, &normalized).is_some() {
            return true;
        }

        false
    }

    /// Get compression statistics for embedded content.
    pub fn stats(&self) -> &EmbedStats {
        &self.stats
    }

    /// Compile the document, reusing the cached result if available.
    fn compile_cached(&self) -> Result<()> {
        if self.lock_cache().is_some() {
            return Ok(());
        }

        // Read main template content (compressed)
        let main_file =
            find_entry(self.templates, self.entry).ok_or(Error::EntryNotFound(self.entry))?;

        let main_bytes = decompress(main_file.contents())?;
        let main_content = std::str::from_utf8(&main_bytes).map_err(|_| Error::InvalidUtf8)?;

        let mut resolver = EmbeddedResolver::new(self.templates, self.packages);
        for (path, data) in self.lock_runtime_files().iter() {
            resolver.insert_runtime_file(path.clone(), data.clone());
        }

        // Collect and decompress fonts from the embedded fonts directory
        let font_data: Vec<Vec<u8>> = self
            .fonts
            .files()
            .map(|f| decompress(f.contents()).map_err(Error::from))
            .collect::<Result<Vec<_>>>()?;

        let font_refs: Vec<&[u8]> = font_data.iter().map(Vec::as_slice).collect();

        let engine = TypstEngine::builder()
            .main_file((self.entry, main_content))
            .add_file_resolver(resolver)
            .fonts(font_refs)
            .build();

        // Clone inputs (preserve for retry on failure)
        let inputs = self.lock_inputs().clone();

        let warned_result = if let Some(inputs) = inputs {
            engine.compile_with_input::<_, PagedDocument>(inputs)
        } else {
            engine.compile::<PagedDocument>()
        };

        // Handle the Warned wrapper and extract result
        let compiled = warned_result.output.map_err(|e| {
            let msg = match e {
                TypstAsLibError::TypstSource(diagnostics) => diagnostics
                    .iter()
                    .map(|d| d.message.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
                other => other.to_string(),
            };
            Error::Compilation(msg)
        })?;

        *self.lock_cache() = Some(compiled);

        Ok(())
    }

    /// Compile if needed, then call `f` with a reference to the compiled document.
    fn with_compiled<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&PagedDocument) -> Result<T>,
    {
        self.compile_cached()?;
        let cache = self.lock_cache();
        let compiled = cache
            .as_ref()
            .expect("compiled_cache must be Some after successful compile_cached()");
        f(compiled)
    }

    /// Compile the document and generate PDF.
    ///
    /// # Returns
    /// PDF data as bytes.
    ///
    /// # Errors
    /// Returns an error if compilation or PDF generation fails.
    #[cfg(feature = "pdf")]
    #[cfg_attr(docsrs, doc(cfg(feature = "pdf")))]
    pub fn to_pdf(&self) -> Result<Vec<u8>> {
        self.with_compiled(|compiled| {
            typst_pdf::pdf(compiled, &typst_pdf::PdfOptions::default())
                .map_err(|e| Error::PdfGeneration(format!("{e:?}")))
        })
    }

    /// Compile the document and generate SVG for each page.
    ///
    /// # Returns
    /// A vector of SVG strings, one per page.
    ///
    /// # Errors
    /// Returns an error if compilation fails.
    #[cfg(feature = "svg")]
    #[cfg_attr(docsrs, doc(cfg(feature = "svg")))]
    pub fn to_svg(&self) -> Result<Vec<String>> {
        self.with_compiled(|compiled| Ok(compiled.pages.iter().map(typst_svg::svg).collect()))
    }

    /// Compile the document and generate PNG for each page.
    ///
    /// # Arguments
    /// * `dpi` - Resolution in dots per inch (e.g., 72 for 1:1, 144 for Retina, 300 for print)
    ///
    /// # Returns
    /// A vector of PNG bytes, one per page.
    ///
    /// # Errors
    /// Returns an error if compilation or PNG encoding fails.
    #[cfg(feature = "png")]
    #[cfg_attr(docsrs, doc(cfg(feature = "png")))]
    pub fn to_png(&self, dpi: f32) -> Result<Vec<Vec<u8>>> {
        self.with_compiled(|compiled| {
            let pixel_per_pt = dpi / 72.0;
            compiled
                .pages
                .iter()
                .map(|page| {
                    typst_render::render(page, pixel_per_pt)
                        .encode_png()
                        .map_err(|e| Error::PngEncoding(e.to_string()))
                })
                .collect()
        })
    }
}

/// Find a file in a `Dir` tree by a potentially nested path (e.g. "dir/main.typ").
fn find_entry<'a>(dir: &'a Dir<'a>, path: &str) -> Option<&'a File<'a>> {
    let normalized = path.trim_start_matches("./").replace('\\', "/");
    let (dir_path, file_name) = match normalized.rsplit_once('/') {
        Some((d, f)) => (Some(d), f),
        None => (None, normalized.as_str()),
    };

    let target_dir = match dir_path {
        Some(dir_path) => {
            let mut current = dir;
            for segment in dir_path.split('/') {
                current = current
                    .dirs()
                    .find(|d| d.path().file_name().and_then(|n| n.to_str()) == Some(segment))?;
            }
            current
        }
        None => dir,
    };

    target_dir
        .files()
        .find(|f| f.path().file_name().and_then(|n| n.to_str()) == Some(file_name))
}
