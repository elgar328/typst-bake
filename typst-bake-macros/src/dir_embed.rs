//! Directory embedding - generates Dir struct code without using include_dir! macro

use proc_macro2::TokenStream;
use quote::quote;
use std::path::Path;

/// Generate code that creates a Dir struct from a directory path.
///
/// This function scans the directory and generates Rust code that constructs
/// the Dir struct directly, avoiding the need for users to add include_dir
/// as a dependency.
pub fn embed_dir(dir_path: &Path) -> TokenStream {
    if !dir_path.exists() {
        // Return empty Dir for non-existent directories (e.g., empty cache)
        return quote! {
            ::typst_bake::__internal::include_dir::Dir::new("", &[])
        };
    }

    let entries = scan_dir_entries(dir_path, dir_path);

    quote! {
        ::typst_bake::__internal::include_dir::Dir::new("", &[
            #(#entries),*
        ])
    }
}

/// Recursively scan directory and generate DirEntry code for each item.
fn scan_dir_entries(base: &Path, current: &Path) -> Vec<TokenStream> {
    let mut entries = Vec::new();

    let read_dir = match std::fs::read_dir(current) {
        Ok(rd) => rd,
        Err(_) => return entries,
    };

    // Collect and sort entries for consistent ordering
    let mut dir_entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
    dir_entries.sort_by_key(|e| e.path());

    for entry in dir_entries {
        let path = entry.path();

        // Skip hidden files and directories
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }

        let rel_path = match path.strip_prefix(base) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let rel_path_str = rel_path.to_string_lossy().to_string();

        if path.is_file() {
            let abs_path = path.to_string_lossy().to_string();
            entries.push(quote! {
                ::typst_bake::__internal::include_dir::DirEntry::File(
                    ::typst_bake::__internal::include_dir::File::new(
                        #rel_path_str,
                        include_bytes!(#abs_path)
                    )
                )
            });
        } else if path.is_dir() {
            let sub_entries = scan_dir_entries(base, &path);
            entries.push(quote! {
                ::typst_bake::__internal::include_dir::DirEntry::Dir(
                    ::typst_bake::__internal::include_dir::Dir::new(
                        #rel_path_str,
                        &[#(#sub_entries),*]
                    )
                )
            });
        }
        // Skip symlinks and other special files
    }

    entries
}

/// Generate code that embeds only font files from a directory.
/// Supported formats: .ttf, .otf, .ttc
pub fn embed_fonts_dir(dir_path: &Path) -> TokenStream {
    if !dir_path.exists() {
        return quote! {
            ::typst_bake::__internal::include_dir::Dir::new("", &[])
        };
    }

    let entries = scan_font_entries(dir_path, dir_path);

    quote! {
        ::typst_bake::__internal::include_dir::Dir::new("", &[
            #(#entries),*
        ])
    }
}

/// Recursively scan directory and generate DirEntry code for font files only.
fn scan_font_entries(base: &Path, current: &Path) -> Vec<TokenStream> {
    let mut entries = Vec::new();

    let read_dir = match std::fs::read_dir(current) {
        Ok(rd) => rd,
        Err(_) => return entries,
    };

    // Collect and sort entries for consistent ordering
    let mut dir_entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
    dir_entries.sort_by_key(|e| e.path());

    for entry in dir_entries {
        let path = entry.path();

        // Skip hidden files and directories
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }

        let rel_path = match path.strip_prefix(base) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let rel_path_str = rel_path.to_string_lossy().to_string();

        if path.is_file() {
            // Only include font files
            if !is_font_file(&path) {
                continue;
            }
            let abs_path = path.to_string_lossy().to_string();
            entries.push(quote! {
                ::typst_bake::__internal::include_dir::DirEntry::File(
                    ::typst_bake::__internal::include_dir::File::new(
                        #rel_path_str,
                        include_bytes!(#abs_path)
                    )
                )
            });
        } else if path.is_dir() {
            let sub_entries = scan_font_entries(base, &path);
            // Only include directory if it contains font files
            if !sub_entries.is_empty() {
                entries.push(quote! {
                    ::typst_bake::__internal::include_dir::DirEntry::Dir(
                        ::typst_bake::__internal::include_dir::Dir::new(
                            #rel_path_str,
                            &[#(#sub_entries),*]
                        )
                    )
                });
            }
        }
    }

    entries
}

/// Check if a file is a supported font file.
fn is_font_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext.to_lowercase().as_str(), "ttf" | "otf" | "ttc")
}
