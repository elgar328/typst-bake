//! Error types for typst-bake.

use std::fmt;

use thiserror::Error;

/// A source location (file, line, column) within a Typst source file.
///
/// Line and column are 1-based; the column counts characters from the start of
/// the line, matching the Typst CLI's reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// Path of the source file, e.g. `reports/event_report/report.typ`.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number (character count within the line).
    pub column: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// A single Typst compilation diagnostic with resolved source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Where the error occurred, if it points into a source file.
    pub location: Option<SourceLocation>,
    /// The diagnostic message.
    pub message: String,
    /// Additional hints the compiler provided.
    pub hints: Vec<String>,
    /// The chain of call/import sites leading to the error (may be empty).
    pub trace: Vec<SourceLocation>,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.location {
            Some(loc) => write!(f, "{loc}: error: {}", self.message)?,
            None => write!(f, "error: {}", self.message)?,
        }
        for hint in &self.hints {
            write!(f, "\n  hint: {hint}")?;
        }
        for site in &self.trace {
            write!(f, "\n  called from: {site}")?;
        }
        Ok(())
    }
}

/// Format a list of diagnostics, one per line, for the `Compilation` error.
fn format_diagnostics(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Errors that can occur during document compilation and rendering.
#[derive(Error, Debug)]
pub enum Error {
    /// Entry file was not found in the embedded templates.
    #[error("entry file not found: {0}")]
    EntryNotFound(&'static str),

    /// Entry file content is not valid UTF-8.
    #[error("entry file is not valid UTF-8")]
    InvalidUtf8,

    /// Typst compilation failed.
    #[error("compilation failed:\n{}", format_diagnostics(.0))]
    Compilation(Vec<Diagnostic>),

    /// PDF generation failed.
    #[error("PDF generation failed: {0}")]
    PdfGeneration(String),

    /// PNG encoding failed.
    #[error("PNG encoding failed: {0}")]
    PngEncoding(String),

    /// Invalid file path provided for runtime file injection.
    #[error("invalid file path: {0}")]
    InvalidFilePath(String),

    /// Invalid page selection (empty or out of range).
    #[error("invalid page selection: {0}")]
    InvalidPageSelection(String),

    /// Invalid PDF configuration (e.g. a standard/tagging conflict or bad timestamp).
    #[error("invalid PDF config: {0}")]
    InvalidPdfConfig(String),

    /// Decompression of embedded content failed.
    #[error("decompression failed")]
    Decompression(#[from] std::io::Error),
}

/// A specialized Result type for typst-bake operations.
pub type Result<T> = std::result::Result<T, Error>;
