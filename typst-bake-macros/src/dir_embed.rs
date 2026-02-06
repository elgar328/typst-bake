//! Directory embedding with zstd compression

use crate::compression_cache::CompressionCache;
use proc_macro2::TokenStream;
use quote::quote;
use std::fs;
use std::path::Path;

/// Result of embedding a directory, containing entries and statistics.
pub struct DirEmbedResult {
    /// DirEntry tokens for each item in the directory
    pub entries: Vec<TokenStream>,
    /// Original uncompressed size in bytes
    pub original_size: usize,
    /// Compressed size in bytes
    pub compressed_size: usize,
    /// Number of files embedded
    pub file_count: usize,
}

impl DirEmbedResult {
    /// Wrap entries in a Dir::new(...) expression
    pub fn to_dir_code(&self, name: &str) -> TokenStream {
        let entries = &self.entries;
        quote! {
            ::typst_bake::__internal::include_dir::Dir::new(#name, &[#(#entries),*])
        }
    }
}

/// Generate code that creates a Dir struct from a directory path.
/// Files are compressed with zstd using the configured compression level and cache.
pub fn embed_dir(dir_path: &Path, cache: &mut CompressionCache) -> DirEmbedResult {
    if !dir_path.exists() {
        return DirEmbedResult {
            entries: Vec::new(),
            original_size: 0,
            compressed_size: 0,
            file_count: 0,
        };
    }

    let mut original_size = 0;
    let mut compressed_size = 0;
    let mut file_count = 0;

    let entries = scan_entries(
        dir_path,
        dir_path,
        |_| true,
        &mut original_size,
        &mut compressed_size,
        &mut file_count,
        cache,
    );

    DirEmbedResult {
        entries,
        original_size,
        compressed_size,
        file_count,
    }
}

/// Recursively scan directory and generate DirEntry code for each item.
/// Uses file_filter to determine which files to include.
fn scan_entries<F>(
    base: &Path,
    current: &Path,
    file_filter: F,
    original_size: &mut usize,
    compressed_size: &mut usize,
    file_count: &mut usize,
    cache: &mut CompressionCache,
) -> Vec<TokenStream>
where
    F: Fn(&Path) -> bool + Copy,
{
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

        // Use just the file/dir name (not full relative path) for proper nesting
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&rel_path_str)
            .to_string();

        if path.is_file() {
            // Apply file filter
            if !file_filter(&path) {
                continue;
            }

            // Read file and compress
            let file_bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };

            let original_len = file_bytes.len();
            let compressed = cache.compress(&file_bytes);
            let compressed_len = compressed.len();

            *original_size += original_len;
            *compressed_size += compressed_len;
            *file_count += 1;

            // Create byte string literal (single token, not token explosion)
            let bytes_literal = syn::LitByteStr::new(&compressed, proc_macro2::Span::call_site());

            // Get absolute path for Cargo file tracking
            let abs_path = path
                .canonicalize()
                .expect("Failed to get absolute path")
                .to_string_lossy()
                .replace('\\', "/");

            entries.push(quote! {
                ::typst_bake::__internal::include_dir::DirEntry::File(
                    ::typst_bake::__internal::include_dir::File::new(
                        #name,
                        {
                            // Cargo file tracking (not used at runtime)
                            const _: &[u8] = include_bytes!(#abs_path);
                            // Actual compressed data
                            #bytes_literal
                        }
                    )
                )
            });
        } else if path.is_dir() {
            let sub_entries = scan_entries(
                base,
                &path,
                file_filter,
                original_size,
                compressed_size,
                file_count,
                cache,
            );
            entries.push(quote! {
                ::typst_bake::__internal::include_dir::DirEntry::Dir(
                    ::typst_bake::__internal::include_dir::Dir::new(
                        #name,
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
pub fn embed_fonts_dir(dir_path: &Path, cache: &mut CompressionCache) -> DirEmbedResult {
    if !dir_path.exists() {
        return DirEmbedResult {
            entries: Vec::new(),
            original_size: 0,
            compressed_size: 0,
            file_count: 0,
        };
    }

    let mut original_size = 0;
    let mut compressed_size = 0;
    let mut file_count = 0;

    let entries = scan_entries(
        dir_path,
        dir_path,
        is_font_file,
        &mut original_size,
        &mut compressed_size,
        &mut file_count,
        cache,
    );

    DirEmbedResult {
        entries,
        original_size,
        compressed_size,
        file_count,
    }
}

/// Check if a file is a supported font file.
fn is_font_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext.to_lowercase().as_str(), "ttf" | "otf" | "ttc")
}
