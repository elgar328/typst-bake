//! Self-contained document for Typst template rendering.

use crate::error::{Diagnostic, Error, Hint, Result, Severity, SourceLocation};
use crate::fonts::{FontSet, FontStore};
#[cfg(feature = "pdf")]
use crate::pdf_config::PdfConfig;
use crate::resolver::{EmbeddedResolver, file_id_to_path, normalize_file_path};
use crate::stats::EmbedStats;
use crate::util::decompress;
use include_dir::{Dir, File};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Mutex, MutexGuard};
use typst::diag::{FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Dict, Duration};
use typst::syntax::{DiagSpan, FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World, WorldExt};
use typst_kit::datetime::Time;
use typst_layout::PagedDocument;

/// A compiled document together with the warnings typst produced for it.
///
/// Kept as one unit so that invalidating the cache always drops both.
struct Compiled {
    doc: PagedDocument,
    warnings: Vec<Diagnostic>,
}

/// A fully self-contained document ready for rendering.
///
/// Created by the [`document!`](crate::document!) macro with embedded templates, fonts,
/// and packages. All resources are compressed with zstd and decompressed lazily at runtime.
pub struct Document {
    templates: &'static Dir<'static>,
    packages: &'static Dir<'static>,
    fonts: &'static FontSet,
    entry: &'static str,
    inputs: Mutex<Option<Dict>>,
    runtime_files: Mutex<HashMap<String, Vec<u8>>>,
    stats: EmbedStats,
    compiled_cache: Mutex<Option<Compiled>>,
    /// The fixed current time (Unix seconds) set by [`Document::with_now`].
    now: Option<i64>,
    /// PDF export options. Set by [`Document::with_pdf_config`]. A plain field (no
    /// `Mutex`): the builder takes `self` by value to write it, and rendering reads it
    /// through `&self`. Affects PDF export only, so it never invalidates `compiled_cache`.
    #[cfg(feature = "pdf")]
    pdf_config: PdfConfig,
}

impl Document {
    /// Internal constructor used by the macro.
    /// Do not use directly.
    #[doc(hidden)]
    pub fn __new(
        templates: &'static Dir<'static>,
        packages: &'static Dir<'static>,
        fonts: &'static FontSet,
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
            now: None,
            #[cfg(feature = "pdf")]
            pdf_config: PdfConfig::default(),
        }
    }

    fn lock_inputs(&self) -> MutexGuard<'_, Option<Dict>> {
        self.inputs.lock().expect("lock poisoned")
    }

    fn lock_runtime_files(&self) -> MutexGuard<'_, HashMap<String, Vec<u8>>> {
        self.runtime_files.lock().expect("lock poisoned")
    }

    fn lock_cache(&self) -> MutexGuard<'_, Option<Compiled>> {
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

    /// Fix the current time that templates see through `datetime.today()`.
    ///
    /// `unix_secs` is seconds since 1970-01-01T00:00:00Z. `datetime.today()` then returns the
    /// UTC date of that instant, and `datetime.today(offset: 9)` the date at UTC+9.
    ///
    /// Without it, the system clock is read during compilation and `datetime.today()` returns
    /// the local date. On `wasm32-unknown-unknown` there is no clock unless the `js-clock`
    /// feature is enabled, so there `datetime.today()` is a compile error without this.
    ///
    /// This does not set the PDF creation date (see `PdfConfig::timestamp`). An instant
    /// outside the years ±9999 also makes `datetime.today()` an error.
    ///
    /// # Example
    /// ```rust,ignore
    /// // 2026-11-11T00:00:00Z
    /// let pdf = typst_bake::document!("main.typ")
    ///     .with_now(1_794_355_200)
    ///     .to_pdf()?;
    /// ```
    pub fn with_now(mut self, unix_secs: i64) -> Self {
        self.now = Some(unix_secs);
        *self.lock_cache() = None;
        self
    }

    /// Set PDF export options.
    ///
    /// Configures PDF-only settings such as tagging, conformance standard, document
    /// identifier, and creation timestamp. See [`PdfConfig`]. These options affect
    /// [`to_pdf`](Self::to_pdf) only; SVG/PNG output ignores them.
    ///
    /// This does not invalidate the compiled cache (options apply at the PDF export
    /// stage, not during compilation). Invalid configurations are reported when
    /// [`to_pdf`](Self::to_pdf) is called, not here; the default config never errors.
    ///
    /// # Example
    /// ```rust,ignore
    /// use typst_bake::{PdfConfig, PdfStandard};
    ///
    /// // Disable tagging to shrink the PDF (bookmarks are preserved).
    /// let pdf = typst_bake::document!("main.typ")
    ///     .with_pdf_config(PdfConfig {
    ///         tagged: false,
    ///         standard: PdfStandard::A2b,
    ///         ..Default::default()
    ///     })
    ///     .to_pdf()?;
    /// ```
    ///
    /// Note: a page selection (via [`select_pages`](Self::select_pages)) always forces
    /// tagging off and drops bookmarks for excluded pages; prefer `tagged: false` on the
    /// full document if you only want to disable tagging.
    #[cfg(feature = "pdf")]
    #[cfg_attr(docsrs, doc(cfg(feature = "pdf")))]
    pub fn with_pdf_config(mut self, config: PdfConfig) -> Self {
        self.pdf_config = config;
        self
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

    /// Select specific pages for output, returning a [`Pages`] view.
    ///
    /// Pages are 0-indexed. Duplicates are removed and pages are always
    /// output in document order regardless of input order.
    ///
    /// # Errors
    /// Returns [`Error::InvalidPageSelection`] at render time if any index
    /// is out of range or the selection is empty.
    ///
    /// # Example
    /// ```rust,ignore
    /// // Select specific pages
    /// let pdf = typst_bake::document!("main.typ")
    ///     .select_pages([0, 2, 4])
    ///     .to_pdf()?;
    ///
    /// // Works with ranges too
    /// let svgs = typst_bake::document!("main.typ")
    ///     .select_pages(0..3)
    ///     .to_svg()?;
    ///
    /// // Reuse with different selections
    /// let doc = typst_bake::document!("main.typ");
    /// let cover = doc.select_pages([0]).to_pdf()?;
    /// let body = doc.select_pages(1..5).to_pdf()?;
    /// ```
    pub fn select_pages(&self, pages: impl IntoIterator<Item = usize>) -> Pages<'_> {
        Pages {
            doc: self,
            indices: pages.into_iter().collect(),
        }
    }

    /// Get the total number of pages in the compiled document.
    ///
    /// Compiles the document if not already compiled.
    /// Returns the total page count regardless of `select_pages`.
    ///
    /// # Example
    /// ```rust,ignore
    /// let doc = typst_bake::document!("main.typ");
    /// let count = doc.page_count()?;
    /// let last_page = doc.select_pages([count - 1]).to_pdf()?;
    /// ```
    pub fn page_count(&self) -> Result<usize> {
        self.with_compiled(|compiled| Ok(compiled.pages().len()))
    }

    /// Get the warnings Typst produced while compiling the document.
    ///
    /// Compiles the document if not already compiled. Returns an empty vector when
    /// the document compiled cleanly, and `Err(Error::Compilation(..))` if it failed
    /// to compile at all.
    ///
    /// Warnings are never printed by this crate; read them here and report them
    /// however you like.
    ///
    /// # Example
    /// ```rust,ignore
    /// // Bind the document: `document!(..).to_pdf()?` drops it before you can ask.
    /// let doc = typst_bake::document!("main.typ");
    /// let pdf = doc.to_pdf()?;
    /// for warning in doc.warnings()? {
    ///     eprintln!("{warning}");
    /// }
    /// ```
    pub fn warnings(&self) -> Result<Vec<Diagnostic>> {
        self.compile_cached()?;
        let cache = self.lock_cache();
        let compiled = cache
            .as_ref()
            .expect("compiled_cache must be Some after successful compile_cached()");
        Ok(compiled.warnings.clone())
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
        let main_content = String::from_utf8(main_bytes).map_err(|_| Error::InvalidUtf8)?;

        // A path typst cannot represent cannot name the entry either.
        let main_id = VirtualPath::new(self.entry)
            .map(|vpath| RootedPath::new(VirtualRoot::Project, vpath).intern())
            .map_err(|_| Error::EntryNotFound(self.entry))?;

        let mut files = EmbeddedResolver::new(self.templates, self.packages);
        for (path, data) in self.lock_runtime_files().iter() {
            files.insert_runtime_file(path.clone(), data.clone());
        }

        // Fonts come from the process-wide store: its book is prebuilt and each font is
        // decompressed only when typst first uses it.
        let fonts = self.fonts.store();

        // Clone inputs (preserve for retry on failure)
        let inputs = self.lock_inputs().clone();

        let baked = BakedWorld::new(
            Source::new(main_id, main_content),
            files,
            fonts,
            inputs,
            clock(self.now),
        );

        let warned = typst::compile::<PagedDocument>(&baked);
        // Drop memoized results right away. The comemo cache is global, so keeping entries
        // would hold memory across documents; don't enlarge this blindly.
        typst::comemo::evict(0);

        // A font that failed to decompress is the root cause of whatever typst reported.
        if let Some(error) = fonts.decode_error() {
            return Err(error);
        }

        let world = &baked;
        let main = world.main();
        let doc = warned.output.map_err(|diagnostics| {
            Error::Compilation(
                diagnostics
                    .iter()
                    .map(|d| diagnostic_from(world, self.entry, main, d))
                    .collect(),
            )
        })?;

        // Resolve now: `world` is local, and spans cannot be resolved without it.
        let warnings = warned
            .warnings
            .iter()
            .map(|d| diagnostic_from(world, self.entry, main, d))
            .collect();

        *self.lock_cache() = Some(Compiled { doc, warnings });

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
        f(&compiled.doc)
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
        self.render_pdf(None)
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
        self.render_svg(None)
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
        self.render_png(None, dpi)
    }

    #[cfg(feature = "pdf")]
    fn render_pdf(&self, selected: Option<&BTreeSet<usize>>) -> Result<Vec<u8>> {
        self.with_compiled(|compiled| {
            // Base options come from the stored config (incl. `tagged`, standard, ident,
            // timestamp, creator).
            let mut options = self.pdf_config.to_typst()?;

            let indices = validate_page_selection(selected, compiled.pages().len())?;
            if let Some(indices) = indices {
                use std::num::NonZeroUsize;
                use typst::layout::PageRanges;

                let ranges = indices
                    .iter()
                    .map(|&i| {
                        let n = Some(NonZeroUsize::new(i + 1).unwrap());
                        n..=n
                    })
                    .collect();
                options.page_ranges = Some(PageRanges::new(ranges));

                // --- Single safety net: page selection forces tagging off ---
                //
                // Page selection sets `page_ranges`, which is incompatible with tagged
                // PDF. typst-pdf does NOT error on `tagged: true` + `page_ranges`; it
                // silently emits a structure tree referencing ALL pages while only a
                // subset is exported, yielding a malformed/misaligned tag tree. See:
                //   - typst/typst#7743 (tagged PDF incompatible with page ranges)
                // So we defensively force tagging off here, overriding `PdfConfig.tagged`
                // — but ONLY on the page-selection path. Full-document `tagged: false` is
                // handled by `to_typst` and is unaffected.
                //
                // Bookmarks: the document outline (/Outlines) is independent of tagging
                // (typst-pdf sets the outline unconditionally; the tag tree only when
                // enabled), so disabling tagging keeps bookmarks. Bookmarks pointing at
                // EXCLUDED pages are still dropped here — that loss is caused by
                // `page_ranges`, not by tagging.
                //
                // Accessible standards (PDF/A-*a, PDF/UA-1) mandate tagging, so they
                // cannot coexist with page selection; reject them explicitly rather than
                // emit a non-conformant PDF.
                if self.pdf_config.standard.requires_tagging() {
                    return Err(Error::InvalidPdfConfig(format!(
                        "page selection is incompatible with {:?} (requires tagging)",
                        self.pdf_config.standard
                    )));
                }
                options.tagged = false;
            }

            // Invariant backstop: tagged PDF + page ranges must never escape together.
            debug_assert!(!(options.tagged && options.page_ranges.is_some()));

            typst_pdf::pdf(compiled, &options).map_err(|e| Error::PdfGeneration(format!("{e:?}")))
        })
    }

    #[cfg(feature = "svg")]
    fn render_svg(&self, selected: Option<&BTreeSet<usize>>) -> Result<Vec<String>> {
        self.with_compiled(|compiled| {
            // Defaults match 0.14 behaviour: no bleed, no pretty-printing.
            let options = typst_svg::SvgOptions::default();
            let indices = validate_page_selection(selected, compiled.pages().len())?;
            match indices {
                Some(indices) => Ok(indices
                    .iter()
                    .map(|&i| typst_svg::svg(&compiled.pages()[i], &options))
                    .collect()),
                None => Ok(compiled
                    .pages()
                    .iter()
                    .map(|page| typst_svg::svg(page, &options))
                    .collect()),
            }
        })
    }

    #[cfg(feature = "png")]
    fn render_png(&self, selected: Option<&BTreeSet<usize>>, dpi: f32) -> Result<Vec<Vec<u8>>> {
        self.with_compiled(|compiled| {
            // `RenderOptions::default()` uses 2.0 pixels per point, so always set it
            // explicitly from the requested DPI.
            let options = typst_render::RenderOptions {
                pixel_per_pt: typst::utils::Scalar::new(f64::from(dpi) / 72.0),
                ..Default::default()
            };
            let indices = validate_page_selection(selected, compiled.pages().len())?;
            let pages: Box<dyn Iterator<Item = &_>> = match &indices {
                Some(indices) => Box::new(indices.iter().map(|&i| &compiled.pages()[i])),
                None => Box::new(compiled.pages().iter()),
            };
            pages
                .map(|page| {
                    typst_render::render(page, &options)
                        .encode_png()
                        .map_err(|e| Error::PngEncoding(e.to_string()))
                })
                .collect()
        })
    }
}

/// A lightweight view into a [`Document`] with a page selection filter.
///
/// Created by [`Document::select_pages`]. Holds a reference to the
/// document and an owned set of page indices.
pub struct Pages<'a> {
    doc: &'a Document,
    indices: BTreeSet<usize>,
}

impl Pages<'_> {
    /// Compile the document and generate PDF for the selected pages.
    ///
    /// # Errors
    /// Returns an error if compilation, PDF generation, or page selection fails.
    #[cfg(feature = "pdf")]
    #[cfg_attr(docsrs, doc(cfg(feature = "pdf")))]
    pub fn to_pdf(&self) -> Result<Vec<u8>> {
        self.doc.render_pdf(Some(&self.indices))
    }

    /// Compile the document and generate SVG for the selected pages.
    ///
    /// # Errors
    /// Returns an error if compilation or page selection fails.
    #[cfg(feature = "svg")]
    #[cfg_attr(docsrs, doc(cfg(feature = "svg")))]
    pub fn to_svg(&self) -> Result<Vec<String>> {
        self.doc.render_svg(Some(&self.indices))
    }

    /// Compile the document and generate PNG for the selected pages.
    ///
    /// # Arguments
    /// * `dpi` - Resolution in dots per inch (e.g., 72 for 1:1, 144 for Retina, 300 for print)
    ///
    /// # Errors
    /// Returns an error if compilation, PNG encoding, or page selection fails.
    #[cfg(feature = "png")]
    #[cfg_attr(docsrs, doc(cfg(feature = "png")))]
    pub fn to_png(&self, dpi: f32) -> Result<Vec<Vec<u8>>> {
        self.doc.render_png(Some(&self.indices), dpi)
    }
}

/// Validate page selection and return indices to render.
/// Returns `None` if no selection (= all pages).
fn validate_page_selection(
    selected: Option<&BTreeSet<usize>>,
    total_pages: usize,
) -> Result<Option<Vec<usize>>> {
    if total_pages == 0 {
        return Err(Error::InvalidPageSelection("document has no pages".into()));
    }
    match selected {
        None => Ok(None),
        Some(pages) => {
            if pages.is_empty() {
                return Err(Error::InvalidPageSelection(
                    "page selection is empty".into(),
                ));
            }
            if let Some(&max) = pages.last()
                && max >= total_pages
            {
                return Err(Error::InvalidPageSelection(format!(
                    "page index {max} out of range (valid: 0..={})",
                    total_pages - 1
                )));
            }
            Ok(Some(pages.iter().copied().collect()))
        }
    }
}

/// Resolve a span into a [`SourceLocation`] using the compilation world.
///
/// The entry file's `FileId` is mapped back to the user-facing entry path so it
/// matches exactly what was requested (including nested entries).
fn span_to_location(
    world: &BakedWorld,
    entry: &str,
    main: FileId,
    span: impl Into<DiagSpan>,
) -> Option<SourceLocation> {
    let span = span.into();
    let id = span.id()?;
    let range = world.range(span)?;
    let source = world.source(id).ok()?;
    let (line, column) = source.lines().byte_to_line_column(range.start)?;
    let file = if id == main {
        entry.to_string()
    } else {
        file_id_to_path(id)
    };
    Some(SourceLocation {
        file,
        line: line + 1,
        column: column + 1,
    })
}

/// The world typst compiles against: embedded templates and packages, runtime files,
/// lazily decoded fonts, and the current date.
struct BakedWorld<'a> {
    library: LazyHash<Library>,
    main: Source,
    files: EmbeddedResolver,
    fonts: &'a FontStore,
    /// `None`: no clock, so `datetime.today()` fails (only if a template calls it).
    time: Option<Time>,
}

impl<'a> BakedWorld<'a> {
    fn new(
        main: Source,
        files: EmbeddedResolver,
        fonts: &'a FontStore,
        inputs: Option<Dict>,
        time: Option<Time>,
    ) -> Self {
        let mut library = Library::builder();
        if let Some(inputs) = inputs {
            library = library.with_inputs(inputs);
        }
        Self {
            library: LazyHash::new(library.build()),
            main,
            files,
            fonts,
            time,
        }
    }
}

/// The clock for `datetime.today()`: the fixed time from [`Document::with_now`], else the
/// system clock. wasm32-unknown-unknown has none unless `js-clock` reads it from JavaScript.
fn clock(now: Option<i64>) -> Option<Time> {
    match now {
        Some(secs) => Time::fixed_timestamp(secs).ok(),
        None if cfg!(all(
            target_family = "wasm",
            target_os = "unknown",
            not(feature = "js-clock")
        )) =>
        {
            None
        }
        // Read lazily, at most once per compilation: local date, exact offsets.
        None => Some(Time::system()),
    }
}

impl World for BakedWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            self.files.source(id)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.file(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.time.as_ref()?.today(offset)
    }
}

/// Convert a Typst [`SourceDiagnostic`] into a typst-bake [`Diagnostic`] with
/// resolved source locations.
fn diagnostic_from(
    world: &BakedWorld,
    entry: &str,
    main: FileId,
    diagnostic: &SourceDiagnostic,
) -> Diagnostic {
    Diagnostic {
        severity: match diagnostic.severity {
            typst::diag::Severity::Error => Severity::Error,
            typst::diag::Severity::Warning => Severity::Warning,
        },
        location: span_to_location(world, entry, main, diagnostic.span),
        message: diagnostic.message.to_string(),
        hints: diagnostic
            .hints
            .iter()
            .map(|h| Hint {
                message: h.v.to_string(),
                location: span_to_location(world, entry, main, h.span),
            })
            .collect(),
        trace: diagnostic
            .trace
            .iter()
            .filter_map(|t| span_to_location(world, entry, main, t.span))
            .collect(),
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

#[cfg(test)]
mod tests {
    use super::*;

    static EMPTY: Dir<'static> = Dir::new("", &[]);
    static NO_FONTS: FontSet = FontSet::new(&[]);

    /// 2026-11-11T20:30:00Z.
    const EVENING_OF_NOV_11: i64 = 1_794_429_000;

    /// A world with `src` as its main file and no other files or fonts.
    fn test_world(
        entry: &str,
        src: &str,
        inputs: Option<Dict>,
        time: Option<Time>,
    ) -> BakedWorld<'static> {
        let vpath = VirtualPath::new(entry).expect("valid path");
        let id = RootedPath::new(VirtualRoot::Project, vpath).intern();
        BakedWorld::new(
            Source::new(id, src.to_owned()),
            EmbeddedResolver::new(&EMPTY, &EMPTY),
            NO_FONTS.store(),
            inputs,
            time,
        )
    }

    /// Compile `world`, returning the error messages if it fails.
    fn compile(world: &BakedWorld) -> std::result::Result<(), Vec<String>> {
        let warned = typst::compile::<PagedDocument>(world);
        typst::comemo::evict(0);
        warned
            .output
            .map(|_| ())
            .map_err(|errors| errors.iter().map(|e| e.message.to_string()).collect())
    }

    #[test]
    fn today_uses_the_fixed_time() {
        let src = "\
#let day(d) = datetime(year: 2026, month: 11, day: d)
#assert.eq(datetime.today(), day(11))
#assert.eq(datetime.today(offset: 3), day(11))
#assert.eq(datetime.today(offset: 4), day(12))
#assert.eq(datetime.today(offset: duration(hours: 3, minutes: 30)), day(12))
";
        let world = test_world("main.typ", src, None, clock(Some(EVENING_OF_NOV_11)));
        compile(&world).expect("dates match");
    }

    #[test]
    fn a_fixed_time_past_year_9999_has_no_date() {
        // 9999-12-31T23:59:59Z still has a date; one second later does not.
        let src = "#assert.eq(datetime.today(), datetime(year: 9999, month: 12, day: 31))";
        let world = test_world("main.typ", src, None, clock(Some(253_402_300_799)));
        compile(&world).expect("last representable date");

        let world = test_world(
            "main.typ",
            "#datetime.today()",
            None,
            clock(Some(253_402_300_800)),
        );
        let errors = compile(&world).expect_err("past year 9999");
        assert!(errors[0].contains("current date"), "{errors:?}");
    }

    #[test]
    fn without_a_clock_today_fails_only_when_used() {
        let world = test_world("main.typ", "Hello", None, None);
        compile(&world).expect("no clock needed");

        let world = test_world("main.typ", "#datetime.today()", None, None);
        let errors = compile(&world).expect_err("no clock");
        assert!(errors[0].contains("current date"), "{errors:?}");
    }

    #[test]
    fn inputs_reach_sys_inputs() {
        let src = "\
#assert.eq(sys.inputs.title, \"x\")
#assert.eq(std.sys.inputs.title, \"x\")
";
        let mut inputs = Dict::new();
        inputs.insert(
            "title".into(),
            typst::foundations::IntoValue::into_value("x"),
        );
        let world = test_world("main.typ", src, Some(inputs), None);
        compile(&world).expect("inputs are visible");
    }

    /// A `Document` whose only template is `main.typ` with the given source.
    fn test_document(src: &str) -> Document {
        use crate::stats::{CategoryStats, DedupStats, PackageStats};
        use include_dir::DirEntry;

        let blob = zstd::bulk::compress(src.as_bytes(), 3).expect("compress");
        let entries = vec![DirEntry::File(File::new("main.typ", blob.leak()))];
        let templates: &'static Dir<'static> = Box::leak(Box::new(Dir::new("", entries.leak())));
        let empty = CategoryStats {
            original_size: 0,
            compressed_size: 0,
            file_count: 0,
        };
        let stats = EmbedStats {
            templates: empty,
            packages: PackageStats {
                packages: Vec::new(),
                original_size: 0,
                compressed_size: 0,
            },
            fonts: empty,
            dedup: DedupStats {
                total_files: 0,
                unique_blobs: 0,
                duplicate_count: 0,
                saved_bytes: 0,
            },
            compression_level: 3,
        };
        Document::__new(templates, &EMPTY, &NO_FONTS, "main.typ", stats)
    }

    #[test]
    fn with_now_resets_the_cache() {
        let doc =
            test_document("#assert.eq(datetime.today(), datetime(year: 2026, month: 11, day: 11))")
                .with_now(EVENING_OF_NOV_11);
        doc.warnings().expect("compiles on the 11th");

        let doc = doc.with_now(EVENING_OF_NOV_11 + 86_400);
        assert!(
            matches!(doc.warnings(), Err(Error::Compilation(_))),
            "a cached result was reused"
        );
    }

    /// Compile a self-contained broken source and resolve its diagnostics. No
    /// embedded resolver or fonts are needed for an eval-time error.
    fn compile_error(entry: &'static str, src: &'static str) -> Vec<Diagnostic> {
        let world = test_world(entry, src, None, None);
        let warned = typst::compile::<PagedDocument>(&world);
        typst::comemo::evict(0);
        let main = world.main();
        let diagnostics = warned.output.expect_err("source should fail to compile");
        diagnostics
            .iter()
            .map(|d| diagnostic_from(&world, entry, main, d))
            .collect()
    }

    #[test]
    fn compilation_error_exposes_source_location() {
        // `bad_call` is an unknown variable; the error span points at it on line 2.
        let diagnostics = compile_error("test.typ", "Hello\n#bad_call()\n");
        assert!(!diagnostics.is_empty());
        let loc = diagnostics[0]
            .location
            .as_ref()
            .expect("diagnostic carries a source location");
        // The entry file path matches exactly what was requested.
        assert_eq!(loc.file, "test.typ");
        assert_eq!(loc.line, 2);
        assert!(loc.column >= 1);
        assert!(!diagnostics[0].message.is_empty());
    }

    #[test]
    fn nested_entry_path_is_preserved() {
        let diagnostics = compile_error("reports/report.typ", "#oops\n");
        let loc = diagnostics[0].location.as_ref().expect("has location");
        assert_eq!(loc.file, "reports/report.typ");
        assert_eq!(loc.line, 1);
    }

    #[test]
    fn diagnostic_display_with_location_hints_and_trace() {
        let diagnostic = Diagnostic {
            severity: Severity::Error,
            location: Some(SourceLocation {
                file: "report.typ".to_string(),
                line: 42,
                column: 12,
            }),
            message: "boom".to_string(),
            hints: vec![Hint {
                message: "try wrapping it".to_string(),
                location: None,
            }],
            trace: vec![SourceLocation {
                file: "main.typ".to_string(),
                line: 5,
                column: 1,
            }],
        };
        assert_eq!(
            diagnostic.to_string(),
            "report.typ:42:12: error: boom\n  hint: try wrapping it\n  called from: main.typ:5:1"
        );
    }

    #[test]
    fn diagnostic_display_without_location() {
        let diagnostic = Diagnostic {
            severity: Severity::Error,
            location: None,
            message: "boom".to_string(),
            hints: Vec::new(),
            trace: Vec::new(),
        };
        assert_eq!(diagnostic.to_string(), "error: boom");
    }

    #[test]
    fn diagnostic_display_uses_warning_severity() {
        let diagnostic = Diagnostic {
            severity: Severity::Warning,
            location: Some(SourceLocation {
                file: "main.typ".to_string(),
                line: 12,
                column: 3,
            }),
            message: "heading did not stabilize".to_string(),
            hints: Vec::new(),
            trace: Vec::new(),
        };
        assert_eq!(
            diagnostic.to_string(),
            "main.typ:12:3: warning: heading did not stabilize"
        );
    }

    #[test]
    fn located_hint_is_rendered_with_its_position() {
        let diagnostic = Diagnostic {
            severity: Severity::Error,
            location: None,
            message: "boom".to_string(),
            hints: vec![
                Hint {
                    message: "general advice".to_string(),
                    location: None,
                },
                Hint {
                    message: "defined here".to_string(),
                    location: Some(SourceLocation {
                        file: "styles.typ".to_string(),
                        line: 8,
                        column: 20,
                    }),
                },
            ],
            trace: Vec::new(),
        };
        assert_eq!(
            diagnostic.to_string(),
            "error: boom\n  hint: general advice\n  hint at styles.typ:8:20: defined here"
        );
    }

    /// Compile a source that yields warnings but still succeeds, and resolve them.
    fn compile_warnings(entry: &'static str, src: &'static str) -> Vec<Diagnostic> {
        let world = test_world(entry, src, None, None);
        let warned = typst::compile::<PagedDocument>(&world);
        typst::comemo::evict(0);
        let main = world.main();
        assert!(warned.output.is_ok(), "source should compile");
        warned
            .warnings
            .iter()
            .map(|d| diagnostic_from(&world, entry, main, d))
            .collect()
    }

    #[test]
    fn warnings_are_resolved_with_severity_location_and_hints() {
        // `show page` warns (with a hint) but still compiles.
        let diagnostics = compile_warnings("test.typ", "#show page: it => it\nHello\n");
        assert!(
            !diagnostics.is_empty(),
            "source should produce at least one warning"
        );

        let warning = &diagnostics[0];
        assert_eq!(warning.severity, Severity::Warning);
        assert!(warning.message.contains("show page"));

        let loc = warning
            .location
            .as_ref()
            .expect("warning carries a source location");
        assert_eq!(loc.file, "test.typ");
        assert_eq!(loc.line, 1);

        // Hints survive the conversion and render through `Display`.
        assert!(!warning.hints.is_empty(), "warning carries a hint");
        assert!(warning.to_string().contains("\n  hint: "));
    }
}
