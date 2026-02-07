//! Error types for typst-bake.

use thiserror::Error;

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
    #[error("compilation failed:\n{0}")]
    Compilation(String),

    /// PDF generation failed.
    #[error("PDF generation failed: {0}")]
    PdfGeneration(String),

    /// PNG encoding failed.
    #[error("PNG encoding failed: {0}")]
    PngEncoding(String),

    /// Decompression of embedded content failed.
    #[error("decompression failed")]
    Decompression(#[from] std::io::Error),
}

/// A specialized Result type for typst-bake operations.
pub type Result<T> = std::result::Result<T, Error>;
