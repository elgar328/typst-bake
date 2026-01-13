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
//!
//! [dependencies]
//! typst-bake = "0.1"
//! include_dir = "0.7"  # Required for the document!() macro
//! ```
//!
//! Then use the `document!()` macro:
//! ```rust,ignore
//! let pdf = typst_bake::document!("main.typ")
//!     .with_font(include_bytes!("fonts/myfont.ttf"))
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
}
