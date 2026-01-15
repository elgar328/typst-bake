//! Document structure for PDF generation

use crate::resolver::EmbeddedResolver;
use crate::stats::EmbedStats;
use include_dir::Dir;
use std::io::Cursor;
use typst::foundations::Dict;
use typst_as_lib::TypstEngine;

/// A document ready for PDF generation.
///
/// Created by the `document!()` macro with embedded templates, fonts, and packages.
pub struct Document {
    templates: &'static Dir<'static>,
    packages: &'static Dir<'static>,
    fonts: &'static Dir<'static>,
    entry: String,
    inputs: Option<Dict>,
    stats: EmbedStats,
}

impl Document {
    /// Internal constructor used by the macro.
    /// Do not use directly.
    #[doc(hidden)]
    pub fn __new(
        templates: &'static Dir<'static>,
        packages: &'static Dir<'static>,
        fonts: &'static Dir<'static>,
        entry: &str,
        stats: EmbedStats,
    ) -> Self {
        Self {
            templates,
            packages,
            fonts,
            entry: entry.to_string(),
            inputs: None,
            stats,
        }
    }

    /// Add input data to the document.
    ///
    /// The data must implement `IntoDict` from `derive_typst_intoval`.
    ///
    /// # Example
    /// ```rust,ignore
    /// use derive_typst_intoval::{IntoValue, IntoDict};
    ///
    /// #[derive(IntoValue, IntoDict)]
    /// struct Inputs {
    ///     title: String,
    /// }
    ///
    /// typst_bake::document!("main.typ")
    ///     .with_inputs(Inputs { title: "Hello".into() })
    /// ```
    pub fn with_inputs<T: Into<Dict>>(mut self, inputs: T) -> Self {
        self.inputs = Some(inputs.into());
        self
    }

    /// Get compression statistics for embedded content.
    pub fn stats(&self) -> &EmbedStats {
        &self.stats
    }

    /// Compile the document and generate PDF.
    ///
    /// # Returns
    /// PDF data as bytes.
    ///
    /// # Errors
    /// Returns an error if compilation or PDF generation fails.
    pub fn to_pdf(self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Read main template content (compressed)
        let main_file = self
            .templates
            .get_file(&self.entry)
            .ok_or_else(|| format!("Entry file not found: {}", self.entry))?;

        // Decompress main file
        let main_bytes = decompress(main_file.contents())?;
        let main_content =
            std::str::from_utf8(&main_bytes).map_err(|_| "Entry file is not valid UTF-8")?;

        // Create resolver
        let resolver = EmbeddedResolver::new(self.templates, self.packages);

        // Collect and decompress fonts from the embedded fonts directory
        let font_data: Vec<Vec<u8>> = self
            .fonts
            .files()
            .map(|f| decompress(f.contents()).expect("Font decompression failed"))
            .collect();

        let font_refs: Vec<&[u8]> = font_data.iter().map(|v| v.as_slice()).collect();

        // Build engine with main file, resolver, and fonts
        let builder = TypstEngine::builder()
            .main_file(main_content)
            .add_file_resolver(resolver)
            .fonts(font_refs);

        let engine = builder.build();

        // Compile (with or without inputs)
        // Use PagedDocument as the concrete document type
        use typst::layout::PagedDocument;
        let warned_result = if let Some(inputs) = self.inputs {
            engine.compile_with_input::<_, PagedDocument>(inputs)
        } else {
            engine.compile::<PagedDocument>()
        };

        // Handle the Warned wrapper and extract result
        let compiled = warned_result
            .output
            .map_err(|e| format!("Compilation failed: {:?}", e))?;

        // Generate PDF
        let pdf_bytes = typst_pdf::pdf(&compiled, &typst_pdf::PdfOptions::default())
            .map_err(|e| format!("PDF generation failed: {:?}", e))?;

        Ok(pdf_bytes)
    }
}

/// Decompress zstd compressed data
fn decompress(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let decompressed = zstd::decode_all(Cursor::new(data))?;
    Ok(decompressed)
}
