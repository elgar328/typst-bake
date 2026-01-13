//! # typst-bake
//!
//! Bake Typst templates, fonts, and packages into your binary for offline PDF generation.
//!
//! ## Quick Start
//!
//! Add to your `Cargo.toml`:
//! ```toml
//! [package.metadata.typst-bake]
//! template-dir = "./templates"
//! fonts-dir = "./fonts"
//!
//! [dependencies]
//! typst-bake = "0.1"
//! ```
//!
//! Then use the `document!()` macro:
//! ```rust,ignore
//! let pdf = typst_bake::document!("main.typ")
//!     .to_pdf()?;
//!
//! std::fs::write("output.pdf", pdf)?;
//! ```

mod document;
mod resolver;

pub use document::Document;
pub use typst_bake_macros::document;

/// Re-export include_dir for macro-generated code.
#[doc(hidden)]
pub use include_dir;

/// Internal module for macro-generated code.
/// Do not use directly.
#[doc(hidden)]
pub mod __internal {
    pub use super::document::Document;
    pub use include_dir::Dir;
    // Re-export include_dir crate for direct struct construction
    pub use include_dir;
}
