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
mod stats;

pub use document::Document;
pub use stats::{CategoryStats, EmbedStats, PackageInfo, PackageStats};
pub use typst_bake_macros::document;

/// Derive macro for implementing `IntoValue` trait.
pub use typst_bake_macros::IntoValue;

/// Derive macro for adding `into_dict()` method.
pub use typst_bake_macros::IntoDict;

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
    // Re-export typst crate for derive macros
    pub use typst;
}
