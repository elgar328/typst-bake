//! # typst-bake
//!
//! Bake Typst templates, fonts, and packages into your Rust binary to create a **fully
//! self-contained** PDF generation engine with **zero runtime dependencies** on the
//! filesystem or network.
//!
//! ## Features
//!
//! - **File Embedding** - All files in `template-dir` are embedded and accessible from templates
//! - **Font Embedding** - Fonts (TTF, OTF, TTC) in `fonts-dir` are automatically bundled
//! - **Package Bundling** - Scans templates for package imports and recursively resolves all dependencies
//! - **Optimized Binary Size** - Resources are compressed with zstd and decompressed lazily at runtime
//! - **Runtime Inputs** - Pass dynamic data from Rust structs to Typst via [`IntoValue`] / [`IntoDict`] derive macros
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
//! Then use the [`document!`] macro:
//! ```rust,ignore
//! let pdf = typst_bake::document!("main.typ")
//!     .to_pdf()?;
//!
//! std::fs::write("output.pdf", pdf)?;
//! ```

mod build;
mod document;
mod resolver;
mod stats;

pub use build::rebuild_if_changed;
pub use document::Document;
pub use stats::{CategoryStats, EmbedStats, PackageInfo, PackageStats};
/// Creates a [`Document`] with embedded templates, fonts, and packages.
///
/// # Usage
///
/// ```rust,ignore
/// let pdf = typst_bake::document!("main.typ")
///     .to_pdf()?;
/// ```
///
/// # Configuration
///
/// Add to your `Cargo.toml`:
/// ```toml
/// [package.metadata.typst-bake]
/// template-dir = "./templates"
/// fonts-dir = "./fonts"
/// ```
///
/// # What Gets Embedded
///
/// - **Templates**: All files in `template-dir` are embedded and accessible from `.typ` files
/// - **Fonts**: Only supported font formats (TTF, OTF, TTC) are embedded. At least one font
///   is required; without fonts, Typst produces invisible text
/// - **Packages**: Using packages requires no manual setup. Just use `#import "@preview/..."`
///   as you normally would in Typst. The macro scans templates for package imports and
///   recursively resolves all dependencies at compile time
pub use typst_bake_macros::document;

/// Derive macro for converting a struct to a Typst value.
///
/// All structs that will be passed to Typst templates (directly or nested) must derive this.
///
/// - **Top-level struct**: Use both [`IntoValue`] and [`IntoDict`]
/// - **Nested structs**: Use [`IntoValue`] only
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
/// ```
///
/// In Typst templates, nested structs are accessed as dictionaries:
/// ```typ
/// #for product in inputs.products [
///   - #product.name: $#product.price
/// ]
/// ```
pub use typst_bake_macros::IntoValue;

/// Derive macro for converting a struct to a Typst dictionary.
///
/// Only the top-level struct passed to [`Document::with_inputs`] needs this.
/// Nested structs should only derive [`IntoValue`].
///
/// # Example
///
/// ```rust,ignore
/// use typst_bake::{IntoValue, IntoDict};
///
/// #[derive(IntoValue, IntoDict)]  // Top-level: both macros
/// struct Inputs {
///     title: String,
///     items: Vec<Item>,
/// }
///
/// #[derive(IntoValue)]  // Nested: IntoValue only
/// struct Item {
///     name: String,
///     price: f64,
/// }
///
/// let pdf = typst_bake::document!("main.typ")
///     .with_inputs(Inputs { /* ... */ })
///     .to_pdf()?;
/// ```
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
