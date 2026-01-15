//! Procedural macros for typst-bake
//!
//! This crate provides the `document!()` macro that embeds templates
//! and packages at compile time.

mod config;
mod derive_intoval;
mod dir_embed;
mod downloader;
mod scanner;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

/// Creates a Document with embedded templates, fonts, and packages.
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
/// Add to your Cargo.toml:
/// ```toml
/// [package.metadata.typst-bake]
/// template-dir = "./templates"
/// fonts-dir = "./fonts"
/// ```
#[proc_macro]
pub fn document(input: TokenStream) -> TokenStream {
    let entry = parse_macro_input!(input as LitStr);
    let entry_value = entry.value();

    // Get template directory
    let template_dir = match config::get_template_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return syn::Error::new_spanned(entry, e)
                .to_compile_error()
                .into();
        }
    };

    // Check if entry file exists
    let entry_path = template_dir.join(&entry_value);
    if !entry_path.exists() {
        return syn::Error::new_spanned(
            entry,
            format!("Entry file not found: {}", entry_path.display()),
        )
        .to_compile_error()
        .into();
    }

    // Get fonts directory
    let fonts_dir = match config::get_fonts_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return syn::Error::new_spanned(entry, e)
                .to_compile_error()
                .into();
        }
    };

    // Scan for packages
    eprintln!("typst-bake: Scanning templates for package imports...");
    let packages = scanner::extract_packages(&template_dir);

    // Download packages if any
    let cache_dir = match downloader::get_cache_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return syn::Error::new_spanned(entry, e)
                .to_compile_error()
                .into();
        }
    };

    if !packages.is_empty() {
        eprintln!(
            "typst-bake: Found {} package(s) to bundle",
            packages.len()
        );

        let refresh = config::should_refresh_cache();
        if let Err(e) = downloader::download_packages(&packages, &cache_dir, refresh) {
            return syn::Error::new_spanned(entry, e)
                .to_compile_error()
                .into();
        }
    } else {
        eprintln!("typst-bake: No packages found in templates");
    }

    // Generate code
    // We directly generate Dir struct code instead of using include_dir! macro
    // This allows users to not need include_dir in their dependencies
    let templates_code = dir_embed::embed_dir(&template_dir);
    let packages_code = dir_embed::embed_dir(&cache_dir);
    let fonts_code = dir_embed::embed_fonts_dir(&fonts_dir);

    let expanded = quote! {
        {
            use ::typst_bake::__internal::{Dir, Document};

            static TEMPLATES: Dir<'static> = #templates_code;
            static PACKAGES: Dir<'static> = #packages_code;
            static FONTS: Dir<'static> = #fonts_code;

            Document::__new(&TEMPLATES, &PACKAGES, &FONTS, #entry_value)
        }
    };

    expanded.into()
}

/// Derive macro for implementing `IntoValue` trait.
///
/// This allows structs to be converted to Typst values for use with `with_inputs()`.
///
/// # Example
///
/// ```rust,ignore
/// use typst_bake::IntoValue;
///
/// #[derive(IntoValue)]
/// struct Item {
///     name: String,
///     quantity: i32,
/// }
/// ```
#[proc_macro_derive(IntoValue)]
pub fn derive_into_value(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::DeriveInput);
    derive_intoval::derive_into_value(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive macro for adding `into_dict()` method to structs.
///
/// This allows structs to be converted to Typst Dict for use with `with_inputs()`.
///
/// # Example
///
/// ```rust,ignore
/// use typst_bake::{IntoValue, IntoDict};
///
/// #[derive(IntoValue, IntoDict)]
/// struct Data {
///     title: String,
///     count: i32,
/// }
///
/// let pdf = typst_bake::document!("main.typ")
///     .with_inputs(data.into_dict())
///     .to_pdf()?;
/// ```
#[proc_macro_derive(IntoDict)]
pub fn derive_into_dict(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::DeriveInput);
    derive_intoval::derive_into_dict(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
