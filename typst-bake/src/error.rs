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

/// Whether a diagnostic is a fatal error or a non-fatal warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A fatal error; compilation did not produce a document.
    Error,
    /// A non-fatal warning; compilation still succeeded.
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => f.write_str("error"),
            Self::Warning => f.write_str("warning"),
        }
    }
}

/// A hint attached to a [`Diagnostic`].
///
/// Most hints are general advice and carry no location. Some instead point at a
/// *secondary* piece of code related to the diagnostic — for those, `location` is
/// set to where that code lives.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Hint {
    /// The hint message.
    pub message: String,
    /// Where the hint points, when it refers to a secondary piece of code.
    pub location: Option<SourceLocation>,
}

impl fmt::Display for Hint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.location {
            Some(loc) => write!(f, "{loc}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

/// A single Typst compilation diagnostic with resolved source location.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic {
    /// Whether this is an error or a warning.
    pub severity: Severity,
    /// Where the diagnostic occurred, if it points into a source file.
    pub location: Option<SourceLocation>,
    /// The diagnostic message.
    pub message: String,
    /// Additional hints the compiler provided.
    pub hints: Vec<Hint>,
    /// The chain of call/import sites leading to the diagnostic (may be empty).
    pub trace: Vec<SourceLocation>,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = self.severity;
        match &self.location {
            Some(loc) => write!(f, "{loc}: {severity}: {}", self.message)?,
            None => write!(f, "{severity}: {}", self.message)?,
        }
        for hint in &self.hints {
            match &hint.location {
                Some(loc) => write!(f, "\n  hint at {loc}: {}", hint.message)?,
                None => write!(f, "\n  hint: {}", hint.message)?,
            }
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
