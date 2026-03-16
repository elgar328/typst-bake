#![cfg_attr(docsrs, feature(doc_cfg))]

//! # typst-bake
//!
//! Bake Typst templates, fonts, and packages into your Rust binary — use Typst
//! as a self-contained, embedded library.
//!
//! ## Cargo Features
//!
//! - **`pdf`** (default) - Enable PDF generation via [`Document::to_pdf`]
//! - **`svg`** - Enable SVG generation via [`Document::to_svg`]
//! - **`png`** - Enable PNG rasterization via [`Document::to_png`]
//! - **`full`** - Enable all output formats
//!
//! PDF is enabled by default. To use only SVG: `default-features = false, features = ["svg"]`.
//!
//! ## Features
//!
//! - **Simple API** - Set `template-dir` and `fonts-dir` in `Cargo.toml`, then generate documents with just `document!("main.typ").to_pdf()`
//! - **Multi-Format Output** - Generate PDF, SVG, or PNG with optional [page selection](`Document::select_pages`)
//! - **Self-Contained Binary** - Templates, fonts, and packages are all embedded into the binary at compile time. No external files or internet connection needed at runtime
//! - **Automatic Package Resolution** - Just use `#import "@preview/..."` as in Typst. Packages are resolved automatically using Typst's own cache and data directories
//! - **Runtime Inputs** - Pass dynamic data from Rust structs to Typst via [`IntoValue`] / [`IntoDict`] derive macros
//! - **Runtime Files** - Inject files at runtime with [`Document::add_file`] for dynamically generated content or downloaded resources
//! - **Optimized Binary Size** - Embedded resources are deduplicated and compressed automatically
//!
//! ## Quick Start
//!
//! Add to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! typst-bake = "0.1"
//!
//! [package.metadata.typst-bake]
//! template-dir = "./templates"
//! fonts-dir = "./fonts"
//! ```
//!
//! Then use the [`document!`] macro:
//! ```rust,ignore
//! let doc = typst_bake::document!("main.typ");
//!
//! let pdf = doc.to_pdf()?;
//! std::fs::write("output.pdf", &pdf)?;
//!
//! let svgs = doc.to_svg()?;
//! std::fs::write("page1.svg", &svgs[0])?;
//!
//! let pngs = doc.to_png(144.0)?; // 144 DPI
//! std::fs::write("page1.png", &pngs[0])?;
//! ```

mod build;
mod document;
mod error;
mod resolver;
mod stats;
mod util;

pub use build::rebuild_if_changed;
pub use document::{Document, Pages};
pub use error::{Error, Result};
pub use stats::{
    CategoryStats, DedupStats, EmbedStats, HasCompressionRatio, PackageInfo, PackageStats,
};
/// Creates a [`Document`] with embedded templates, fonts, and packages.
///
/// # Usage
///
/// ```rust,ignore
/// let doc = typst_bake::document!("main.typ");
///
/// // Output formats
/// let pdf = doc.to_pdf()?;
/// let svgs = doc.to_svg()?;
/// let pngs = doc.to_png(144.0)?; // 144 DPI
///
/// // Page selection (0-indexed)
/// let cover_pdf = doc.select_pages([0]).to_pdf()?;
/// let body_pdf = doc.select_pages(1..5).to_pdf()?;
///
/// // Page count
/// let total = doc.page_count()?;
/// let last_page = doc.select_pages([total - 1]).to_png(72.0)?;
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
/// - **Templates**: All files in `template-dir` are embedded and accessible from `.typ` files.
///   Paths resolve relative to the referring `.typ` file.
/// - **Fonts**: Only supported font formats (TTF, OTF, TTC) are embedded. At least one font
///   is required; without fonts, Typst produces invisible text.
/// - **Packages**: Using packages requires no manual setup. Just use `#import "@preview/..."`
///   or `#import "@local/..."` as you normally would in Typst. The macro scans for package
///   imports and recursively resolves all dependencies at compile time. Shares Typst's own
///   package directories, so locally installed packages are picked up automatically.
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
/// In `.typ` files, nested structs are accessed as dictionaries:
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
