//! Document structure for PDF generation

use crate::resolver::EmbeddedResolver;
use include_dir::Dir;
use typst::foundations::Dict;
use typst_as_lib::TypstEngine;

/// A document ready for PDF generation.
///
/// Created by the `document!()` macro with embedded templates and packages.
pub struct Document {
    templates: &'static Dir<'static>,
    packages: &'static Dir<'static>,
    entry: String,
    fonts: Vec<&'static [u8]>,
    inputs: Option<Dict>,
}

impl Document {
    /// Internal constructor used by the macro.
    /// Do not use directly.
    #[doc(hidden)]
    pub fn __new(
        templates: &'static Dir<'static>,
        packages: &'static Dir<'static>,
        entry: &str,
    ) -> Self {
        Self {
            templates,
            packages,
            entry: entry.to_string(),
            fonts: Vec::new(),
            inputs: None,
        }
    }

    /// Add a font to the document.
    ///
    /// Fonts should be embedded using `include_bytes!()`.
    ///
    /// # Example
    /// ```rust,ignore
    /// typst_bake::document!("main.typ")
    ///     .with_font(include_bytes!("fonts/myfont.ttf"))
    /// ```
    pub fn with_font(mut self, font_data: &'static [u8]) -> Self {
        self.fonts.push(font_data);
        self
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

    /// Compile the document and generate PDF.
    ///
    /// # Returns
    /// PDF data as bytes.
    ///
    /// # Errors
    /// Returns an error if compilation or PDF generation fails.
    pub fn to_pdf(self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Read main template content
        let main_file = self
            .templates
            .get_file(&self.entry)
            .ok_or_else(|| format!("Entry file not found: {}", self.entry))?;

        let main_content = std::str::from_utf8(main_file.contents())
            .map_err(|_| "Entry file is not valid UTF-8")?;

        // Create resolver
        let resolver = EmbeddedResolver::new(self.templates, self.packages);

        // Build engine with main file and resolver
        let builder = TypstEngine::builder()
            .main_file(main_content)
            .add_file_resolver(resolver)
            .fonts(self.fonts.iter().copied());

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
        let compiled = warned_result.output.map_err(|e| format!("Compilation failed: {:?}", e))?;

        // Generate PDF
        let pdf_bytes = typst_pdf::pdf(&compiled, &typst_pdf::PdfOptions::default())
            .map_err(|e| format!("PDF generation failed: {:?}", e))?;

        Ok(pdf_bytes)
    }
}
