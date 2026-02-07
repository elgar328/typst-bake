//! Directory embedding with zstd compression.

use crate::compression_cache::CompressionCache;
use crate::config::{is_font_file, is_hidden};
use proc_macro2::TokenStream;
use quote::quote;
use std::fs;
use std::path::Path;

/// Result of embedding a directory, containing entries and statistics.
#[derive(Default)]
pub struct DirEmbedResult {
    /// DirEntry tokens for each item in the directory.
    pub entries: Vec<TokenStream>,
    /// Original uncompressed size in bytes.
    pub original_size: usize,
    /// Compressed size in bytes.
    pub compressed_size: usize,
    /// Number of files embedded.
    pub file_count: usize,
}

impl DirEmbedResult {
    /// Wrap entries in a `Dir::new(...)` expression.
    pub fn to_dir_code(&self, name: &str) -> TokenStream {
        let entries = &self.entries;
        quote! {
            ::typst_bake::__internal::include_dir::Dir::new(#name, &[#(#entries),*])
        }
    }
}

/// Context for recursive directory scanning, bundling mutable state and config.
struct ScanContext<'a, F> {
    base: &'a Path,
    file_filter: F,
    original_size: usize,
    compressed_size: usize,
    file_count: usize,
    cache: &'a mut CompressionCache,
}

impl<'a, F> ScanContext<'a, F>
where
    F: Fn(&Path) -> bool + Copy,
{
    fn new(base: &'a Path, file_filter: F, cache: &'a mut CompressionCache) -> Self {
        Self {
            base,
            file_filter,
            original_size: 0,
            compressed_size: 0,
            file_count: 0,
            cache,
        }
    }

    /// Recursively scan directory and generate DirEntry code for each item.
    fn scan_entries(&mut self, current: &Path) -> Vec<TokenStream> {
        let mut entries = Vec::new();

        let Ok(read_dir) = fs::read_dir(current) else {
            return entries;
        };

        // Collect and sort entries for consistent ordering
        let mut dir_entries: Vec<_> = read_dir.filter_map(Result::ok).collect();
        dir_entries.sort_by_key(|e| e.path());

        for entry in dir_entries {
            let path = entry.path();

            if is_hidden(&path) {
                continue;
            }

            let Ok(rel_path) = path.strip_prefix(self.base) else {
                continue;
            };

            // Use just the file/dir name (not full relative path) for proper nesting
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_owned(),
                None => rel_path.to_string_lossy().into_owned(),
            };

            if path.is_file() {
                if !(self.file_filter)(&path) {
                    continue;
                }

                let Ok(file_bytes) = fs::read(&path) else {
                    continue;
                };

                let original_len = file_bytes.len();
                let blob_info = self.cache.compress(&file_bytes);
                let compressed_len = blob_info.compressed_len;

                self.original_size += original_len;
                self.compressed_size += compressed_len;
                self.file_count += 1;

                let blob_ident = quote::format_ident!("BLOB_{}", blob_info.hash);

                // Get absolute path for Cargo file tracking
                let abs_path = path
                    .canonicalize()
                    .unwrap_or_else(|_| path.to_path_buf())
                    .to_string_lossy()
                    .replace('\\', "/");

                entries.push(quote! {
                    ::typst_bake::__internal::include_dir::DirEntry::File(
                        ::typst_bake::__internal::include_dir::File::new(
                            #name,
                            {
                                // Cargo file tracking (not used at runtime)
                                const _: &[u8] = include_bytes!(#abs_path);
                                &#blob_ident
                            }
                        )
                    )
                });
            } else if path.is_dir() {
                let sub_entries = self.scan_entries(&path);
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

    fn into_result(self, entries: Vec<TokenStream>) -> DirEmbedResult {
        DirEmbedResult {
            entries,
            original_size: self.original_size,
            compressed_size: self.compressed_size,
            file_count: self.file_count,
        }
    }
}

fn embed_with_filter(
    dir_path: &Path,
    filter: impl Fn(&Path) -> bool + Copy,
    cache: &mut CompressionCache,
) -> DirEmbedResult {
    if !dir_path.exists() {
        return DirEmbedResult::default();
    }
    let mut ctx = ScanContext::new(dir_path, filter, cache);
    let entries = ctx.scan_entries(dir_path);
    ctx.into_result(entries)
}

/// Generate code that creates a Dir struct from a directory path.
/// Files are compressed with zstd using the configured compression level and cache.
pub fn embed_dir(dir_path: &Path, cache: &mut CompressionCache) -> DirEmbedResult {
    embed_with_filter(dir_path, |_| true, cache)
}

/// Generate code that embeds only font files from a directory.
/// Supported formats: .ttf, .otf, .ttc.
pub fn embed_fonts_dir(dir_path: &Path, cache: &mut CompressionCache) -> DirEmbedResult {
    embed_with_filter(dir_path, is_font_file, cache)
}
