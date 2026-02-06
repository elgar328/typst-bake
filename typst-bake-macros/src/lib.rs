//! Procedural macros for typst-bake
//!
//! This crate provides the [`document!`] macro that embeds templates, fonts,
//! and packages at compile time. All resources are compressed with zstd for
//! optimized binary size.

mod compression_cache;
mod config;
mod derive_intoval;
mod dir_embed;
mod downloader;
mod scanner;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn document(input: TokenStream) -> TokenStream {
    let entry = parse_macro_input!(input as LitStr);
    let entry_value = entry.value();

    // Get template directory
    let template_dir = match config::get_template_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return syn::Error::new_spanned(entry, e).to_compile_error().into();
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
            return syn::Error::new_spanned(entry, e).to_compile_error().into();
        }
    };

    // Scan for packages
    eprintln!("typst-bake: Scanning for package imports...");
    let packages = scanner::extract_packages(&template_dir);

    // Download packages if any
    let cache_dir = match downloader::get_cache_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return syn::Error::new_spanned(entry, e).to_compile_error().into();
        }
    };

    let resolved_packages = if !packages.is_empty() {
        eprintln!("typst-bake: Found {} package(s) to bundle", packages.len());

        let refresh = config::should_refresh_cache();
        match downloader::download_packages(&packages, &cache_dir, refresh) {
            Ok(pkgs) => pkgs,
            Err(e) => {
                return syn::Error::new_spanned(entry, e).to_compile_error().into();
            }
        }
    } else {
        eprintln!("typst-bake: No packages found");
        Vec::new()
    };

    // Set up compression cache
    let compression_level = config::get_compression_level();
    let compression_cache_dir = match config::get_compression_cache_dir() {
        Ok(dir) => Some(dir),
        Err(e) => {
            eprintln!("typst-bake: Compression cache disabled: {}", e);
            None
        }
    };
    let mut cache =
        compression_cache::CompressionCache::new(compression_cache_dir, compression_level);

    // Generate code
    // We directly generate Dir struct code instead of using include_dir! macro
    // This allows users to not need include_dir in their dependencies
    let templates_result = dir_embed::embed_dir(&template_dir, &mut cache);
    let fonts_result = dir_embed::embed_fonts_dir(&fonts_dir, &mut cache);

    // Collect per-package stats and entries in a single pass
    let mut package_infos = Vec::new();
    let mut pkg_total_original = 0usize;
    let mut pkg_total_compressed = 0usize;
    let mut namespace_entries: Vec<proc_macro2::TokenStream> = Vec::new();

    {
        use std::collections::{BTreeMap, BTreeSet};

        // Group resolved packages into a sorted tree: namespace -> name -> versions
        let mut pkg_tree: BTreeMap<&str, BTreeMap<&str, BTreeSet<&str>>> = BTreeMap::new();
        for (ns, name, ver) in &resolved_packages {
            pkg_tree
                .entry(ns.as_str())
                .or_default()
                .entry(name.as_str())
                .or_default()
                .insert(ver.as_str());
        }

        for (namespace, names) in &pkg_tree {
            let mut name_entries: Vec<proc_macro2::TokenStream> = Vec::new();

            for (name, versions) in names {
                let mut version_entries: Vec<proc_macro2::TokenStream> = Vec::new();

                for version in versions {
                    let ver_path = cache_dir.join(namespace).join(name).join(version);

                    // Embed this package
                    let pkg_result = dir_embed::embed_dir(&ver_path, &mut cache);
                    let pkg_name = format!("@{}/{}:{}", namespace, name, version);

                    // Collect stats
                    package_infos.push((
                        pkg_name,
                        pkg_result.original_size,
                        pkg_result.compressed_size,
                        pkg_result.file_count,
                    ));
                    pkg_total_original += pkg_result.original_size;
                    pkg_total_compressed += pkg_result.compressed_size;

                    // Build version directory entry
                    let version_str = *version;
                    let pkg_entries = &pkg_result.entries;
                    version_entries.push(quote! {
                        ::typst_bake::__internal::include_dir::DirEntry::Dir(
                            ::typst_bake::__internal::include_dir::Dir::new(#version_str, &[#(#pkg_entries),*])
                        )
                    });
                }

                // Build name directory entry
                let name_str = *name;
                name_entries.push(quote! {
                    ::typst_bake::__internal::include_dir::DirEntry::Dir(
                        ::typst_bake::__internal::include_dir::Dir::new(#name_str, &[#(#version_entries),*])
                    )
                });
            }

            // Build namespace directory entry
            let ns_str = *namespace;
            namespace_entries.push(quote! {
                ::typst_bake::__internal::include_dir::DirEntry::Dir(
                    ::typst_bake::__internal::include_dir::Dir::new(#ns_str, &[#(#name_entries),*])
                )
            });
        }
    }

    // Log compression stats and clean up unused cache files
    cache.log_summary();
    cache.cleanup();

    // Build final code
    let templates_code = templates_result.to_dir_code("");
    let fonts_code = fonts_result.to_dir_code("");
    let packages_code = quote! {
        ::typst_bake::__internal::include_dir::Dir::new("", &[#(#namespace_entries),*])
    };

    // Generate stats
    let template_original = templates_result.original_size;
    let template_compressed = templates_result.compressed_size;
    let template_count = templates_result.file_count;

    let font_original = fonts_result.original_size;
    let font_compressed = fonts_result.compressed_size;
    let font_count = fonts_result.file_count;

    // Generate package info tokens
    let pkg_info_tokens: Vec<_> = package_infos
        .iter()
        .map(|(name, orig, comp, count)| {
            quote! {
                ::typst_bake::PackageInfo {
                    name: #name.to_string(),
                    original_size: #orig,
                    compressed_size: #comp,
                    file_count: #count,
                }
            }
        })
        .collect();

    let expanded = quote! {
        {
            use ::typst_bake::__internal::{Dir, Document};

            static TEMPLATES: Dir<'static> = #templates_code;
            static PACKAGES: Dir<'static> = #packages_code;
            static FONTS: Dir<'static> = #fonts_code;

            let stats = ::typst_bake::EmbedStats {
                templates: ::typst_bake::CategoryStats {
                    original_size: #template_original,
                    compressed_size: #template_compressed,
                    file_count: #template_count,
                },
                packages: ::typst_bake::PackageStats {
                    packages: vec![#(#pkg_info_tokens),*],
                    total_original: #pkg_total_original,
                    total_compressed: #pkg_total_compressed,
                },
                fonts: ::typst_bake::CategoryStats {
                    original_size: #font_original,
                    compressed_size: #font_compressed,
                    file_count: #font_count,
                },
            };

            Document::__new(&TEMPLATES, &PACKAGES, &FONTS, #entry_value, stats)
        }
    };

    expanded.into()
}

#[proc_macro_derive(IntoValue)]
pub fn derive_into_value(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::DeriveInput);
    derive_intoval::derive_into_value(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_derive(IntoDict)]
pub fn derive_into_dict(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::DeriveInput);
    derive_intoval::derive_into_dict(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
