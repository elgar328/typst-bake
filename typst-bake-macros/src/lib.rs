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

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

use compression_cache::CompressionCache;
use dir_embed::DirEmbedResult;

use scanner::PackageSpec;

#[derive(Debug)]
struct MacroPackageInfo {
    name: String,
    original_size: usize,
    compressed_size: usize,
    file_count: usize,
}

type ResolvedPackages = (Vec<PackageSpec>, PathBuf);

/// Collected results from embedding all packages.
struct EmbeddedPackages {
    infos: Vec<MacroPackageInfo>,
    total_original: usize,
    total_compressed: usize,
    namespace_entries: Vec<proc_macro2::TokenStream>,
}

/// Resolve template_dir, fonts_dir and validate the entry file exists.
fn resolve_config(
    entry: &LitStr,
    entry_value: &str,
) -> Result<(PathBuf, PathBuf), proc_macro2::TokenStream> {
    let template_dir = config::get_template_dir()
        .map_err(|e| syn::Error::new_spanned(entry, e).to_compile_error())?;

    let entry_path = template_dir.join(entry_value);
    if !entry_path.exists() {
        return Err(syn::Error::new_spanned(
            entry,
            format!("Entry file not found: {}", entry_path.display()),
        )
        .to_compile_error());
    }

    let fonts_dir = config::get_fonts_dir()
        .map_err(|e| syn::Error::new_spanned(entry, e).to_compile_error())?;

    Ok((template_dir, fonts_dir))
}

/// Scan template directory for package imports and download them.
fn resolve_and_download_packages(
    entry: &LitStr,
    template_dir: &Path,
) -> Result<ResolvedPackages, proc_macro2::TokenStream> {
    eprintln!("typst-bake: Scanning for package imports...");
    let packages = scanner::extract_packages(template_dir);

    let cache_dir = downloader::get_cache_dir()
        .map_err(|e| syn::Error::new_spanned(entry, e).to_compile_error())?;

    let resolved_packages = if !packages.is_empty() {
        eprintln!("typst-bake: Found {} package(s) to bundle", packages.len());

        let refresh = config::should_refresh_cache();
        downloader::download_packages(&packages, &cache_dir, refresh)
            .map_err(|e| syn::Error::new_spanned(entry, e).to_compile_error())?
    } else {
        eprintln!("typst-bake: No packages found");
        Vec::new()
    };

    Ok((resolved_packages, cache_dir))
}

/// Generate a DirEntry::Dir token wrapping children under a given name.
fn dir_entry_token(name: &str, children: &[proc_macro2::TokenStream]) -> proc_macro2::TokenStream {
    quote! {
        ::typst_bake::__internal::include_dir::DirEntry::Dir(
            ::typst_bake::__internal::include_dir::Dir::new(#name, &[#(#children),*])
        )
    }
}

/// Embed all resolved packages, collecting stats and directory entry tokens.
fn embed_packages(
    resolved_packages: &[PackageSpec],
    cache_dir: &Path,
    cache: &mut CompressionCache,
) -> EmbeddedPackages {
    let mut package_infos = Vec::new();
    let mut pkg_total_original = 0;
    let mut pkg_total_compressed = 0;
    let mut namespace_entries = Vec::new();

    // Group resolved packages into a sorted tree: namespace -> name -> versions
    let mut pkg_tree: BTreeMap<&str, BTreeMap<&str, BTreeSet<&str>>> = BTreeMap::new();
    for pkg in resolved_packages {
        pkg_tree
            .entry(pkg.namespace.as_str())
            .or_default()
            .entry(pkg.name.as_str())
            .or_default()
            .insert(pkg.version.as_str());
    }

    for (namespace, names) in &pkg_tree {
        let mut name_entries = Vec::new();

        for (name, versions) in names {
            let mut version_entries = Vec::new();

            for version in versions {
                let ver_path = cache_dir.join(namespace).join(name).join(version);

                let pkg_result = dir_embed::embed_dir(&ver_path, cache);
                let pkg_name = format!("@{namespace}/{name}:{version}");

                package_infos.push(MacroPackageInfo {
                    name: pkg_name,
                    original_size: pkg_result.original_size,
                    compressed_size: pkg_result.compressed_size,
                    file_count: pkg_result.file_count,
                });
                pkg_total_original += pkg_result.original_size;
                pkg_total_compressed += pkg_result.compressed_size;

                version_entries.push(dir_entry_token(version, &pkg_result.entries));
            }

            name_entries.push(dir_entry_token(name, &version_entries));
        }

        namespace_entries.push(dir_entry_token(namespace, &name_entries));
    }

    EmbeddedPackages {
        infos: package_infos,
        total_original: pkg_total_original,
        total_compressed: pkg_total_compressed,
        namespace_entries,
    }
}

/// Generate the final output TokenStream from embedded results and stats.
fn generate_output(
    entry_value: &str,
    templates_result: &DirEmbedResult,
    fonts_result: &DirEmbedResult,
    packages: &EmbeddedPackages,
    cache: &mut CompressionCache,
) -> proc_macro2::TokenStream {
    cache.log_summary();
    cache.cleanup();

    let dedup = cache.dedup_summary();
    let dedup_total_files = dedup.total_files;
    let dedup_unique_blobs = dedup.unique_blobs;
    let dedup_duplicate_count = dedup.duplicate_count;
    let dedup_saved_bytes = dedup.saved_bytes;
    let dedup_statics = cache.dedup_statics();

    let templates_code = templates_result.to_dir_code("");
    let fonts_code = fonts_result.to_dir_code("");
    let namespace_entries = &packages.namespace_entries;
    let packages_code = quote! {
        ::typst_bake::__internal::include_dir::Dir::new("", &[#(#namespace_entries),*])
    };

    let template_original = templates_result.original_size;
    let template_compressed = templates_result.compressed_size;
    let template_count = templates_result.file_count;

    let font_original = fonts_result.original_size;
    let font_compressed = fonts_result.compressed_size;
    let font_count = fonts_result.file_count;

    let pkg_total_original = packages.total_original;
    let pkg_total_compressed = packages.total_compressed;

    let pkg_info_tokens: Vec<_> = packages
        .infos
        .iter()
        .map(|info| {
            let name = &info.name;
            let orig = info.original_size;
            let comp = info.compressed_size;
            let count = info.file_count;
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

    quote! {
        {
            use ::typst_bake::__internal::{Dir, Document};

            #(#dedup_statics)*

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
                    original_size: #pkg_total_original,
                    compressed_size: #pkg_total_compressed,
                },
                fonts: ::typst_bake::CategoryStats {
                    original_size: #font_original,
                    compressed_size: #font_compressed,
                    file_count: #font_count,
                },
                dedup: ::typst_bake::DedupStats {
                    total_files: #dedup_total_files,
                    unique_blobs: #dedup_unique_blobs,
                    duplicate_count: #dedup_duplicate_count,
                    saved_bytes: #dedup_saved_bytes,
                },
            };

            Document::__new(&TEMPLATES, &PACKAGES, &FONTS, #entry_value, stats)
        }
    }
}

#[proc_macro]
pub fn document(input: TokenStream) -> TokenStream {
    let entry = parse_macro_input!(input as LitStr);
    let entry_value = entry.value();

    let (template_dir, fonts_dir) = match resolve_config(&entry, &entry_value) {
        Ok(v) => v,
        Err(e) => return e.into(),
    };

    let (resolved_packages, cache_dir) = match resolve_and_download_packages(&entry, &template_dir)
    {
        Ok(v) => v,
        Err(e) => return e.into(),
    };

    // Set up compression cache
    let compression_level = config::get_compression_level();
    let compression_cache_dir = config::get_compression_cache_dir()
        .map_err(|e| eprintln!("typst-bake: Compression cache disabled: {e}"))
        .ok();
    let mut cache = CompressionCache::new(compression_cache_dir, compression_level);

    // Embed templates and fonts
    let templates_result = dir_embed::embed_dir(&template_dir, &mut cache);
    let fonts_result = dir_embed::embed_fonts_dir(&fonts_dir, &mut cache);

    // Embed packages
    let embedded_packages = embed_packages(&resolved_packages, &cache_dir, &mut cache);

    // Generate final output
    generate_output(
        &entry_value,
        &templates_result,
        &fonts_result,
        &embedded_packages,
        &mut cache,
    )
    .into()
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
