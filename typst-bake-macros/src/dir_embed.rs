//! Directory embedding with zstd compression

use proc_macro2::TokenStream;
use quote::quote;
use std::fs;
use std::io::Cursor;
use std::path::Path;

/// Result of embedding a directory, containing both generated code and statistics.
pub struct DirEmbedResult {
    /// Generated code that constructs the Dir struct
    pub code: TokenStream,
    /// Original uncompressed size in bytes
    pub original_size: usize,
    /// Compressed size in bytes
    pub compressed_size: usize,
    /// Number of files embedded
    pub file_count: usize,
}

/// Generate code that creates a Dir struct from a directory path.
/// Files are compressed with zstd at level 19 (maximum compression).
pub fn embed_dir(dir_path: &Path) -> DirEmbedResult {
    if !dir_path.exists() {
        // Return empty Dir for non-existent directories (e.g., empty cache)
        return DirEmbedResult {
            code: quote! {
                ::typst_bake::__internal::include_dir::Dir::new("", &[])
            },
            original_size: 0,
            compressed_size: 0,
            file_count: 0,
        };
    }

    let mut original_size = 0;
    let mut compressed_size = 0;
    let mut file_count = 0;

    let entries = scan_dir_entries(
        dir_path,
        dir_path,
        &mut original_size,
        &mut compressed_size,
        &mut file_count,
    );

    let code = quote! {
        ::typst_bake::__internal::include_dir::Dir::new("", &[
            #(#entries),*
        ])
    };

    DirEmbedResult {
        code,
        original_size,
        compressed_size,
        file_count,
    }
}

/// Recursively scan directory and generate DirEntry code for each item.
fn scan_dir_entries(
    base: &Path,
    current: &Path,
    original_size: &mut usize,
    compressed_size: &mut usize,
    file_count: &mut usize,
) -> Vec<TokenStream> {
    let mut entries = Vec::new();

    let read_dir = match fs::read_dir(current) {
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
            // Read file and compress
            let file_bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };

            let original_len = file_bytes.len();
            let compressed = compress_bytes(&file_bytes);
            let compressed_len = compressed.len();

            *original_size += original_len;
            *compressed_size += compressed_len;
            *file_count += 1;

            // Create byte string literal (single token, not token explosion)
            let bytes_literal = syn::LitByteStr::new(&compressed, proc_macro2::Span::call_site());

            entries.push(quote! {
                ::typst_bake::__internal::include_dir::DirEntry::File(
                    ::typst_bake::__internal::include_dir::File::new(
                        #rel_path_str,
                        #bytes_literal
                    )
                )
            });
        } else if path.is_dir() {
            let sub_entries =
                scan_dir_entries(base, &path, original_size, compressed_size, file_count);
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
pub fn embed_fonts_dir(dir_path: &Path) -> DirEmbedResult {
    if !dir_path.exists() {
        return DirEmbedResult {
            code: quote! {
                ::typst_bake::__internal::include_dir::Dir::new("", &[])
            },
            original_size: 0,
            compressed_size: 0,
            file_count: 0,
        };
    }

    let mut original_size = 0;
    let mut compressed_size = 0;
    let mut file_count = 0;

    let entries = scan_font_entries(
        dir_path,
        dir_path,
        &mut original_size,
        &mut compressed_size,
        &mut file_count,
    );

    let code = quote! {
        ::typst_bake::__internal::include_dir::Dir::new("", &[
            #(#entries),*
        ])
    };

    DirEmbedResult {
        code,
        original_size,
        compressed_size,
        file_count,
    }
}

/// Recursively scan directory and generate DirEntry code for font files only.
fn scan_font_entries(
    base: &Path,
    current: &Path,
    original_size: &mut usize,
    compressed_size: &mut usize,
    file_count: &mut usize,
) -> Vec<TokenStream> {
    let mut entries = Vec::new();

    let read_dir = match fs::read_dir(current) {
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

            // Read file and compress
            let file_bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };

            let original_len = file_bytes.len();
            let compressed = compress_bytes(&file_bytes);
            let compressed_len = compressed.len();

            *original_size += original_len;
            *compressed_size += compressed_len;
            *file_count += 1;

            // Create byte string literal (single token)
            let bytes_literal = syn::LitByteStr::new(&compressed, proc_macro2::Span::call_site());

            entries.push(quote! {
                ::typst_bake::__internal::include_dir::DirEntry::File(
                    ::typst_bake::__internal::include_dir::File::new(
                        #rel_path_str,
                        #bytes_literal
                    )
                )
            });
        } else if path.is_dir() {
            let sub_entries =
                scan_font_entries(base, &path, original_size, compressed_size, file_count);
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

/// Compress bytes using zstd at maximum compression level (19).
fn compress_bytes(data: &[u8]) -> Vec<u8> {
    zstd::encode_all(Cursor::new(data), 19).expect("zstd compression failed")
}
