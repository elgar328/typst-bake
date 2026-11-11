//! Compression caching to avoid re-compressing unchanged files.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// Summary of deduplication statistics.
#[derive(Debug, Clone, Copy)]
pub struct DedupSummary {
    pub total_files: usize,
    pub unique_blobs: usize,
    pub duplicate_count: usize,
    pub saved_bytes: usize,
}

/// Information about a compressed blob, used for deduplication.
#[derive(Debug, Clone)]
pub struct BlobInfo {
    /// BLAKE3 hex hash of the original data (64 chars).
    pub hash: String,
    /// Size of the compressed data in bytes.
    pub compressed_len: usize,
}

/// Caches zstd-compressed results on disk, keyed by content hash and compression level.
/// Also deduplicates identical content in-memory so each unique blob is stored once.
pub struct CompressionCache {
    cache_dir: Option<PathBuf>,
    level: i32,
    used_files: HashSet<String>,
    cache_hits: usize,
    misses: usize,
    dedup_hits: usize,
    dedup_saved_bytes: usize,
    /// hash → compressed bytes (unique blobs only, BTreeMap for deterministic ordering).
    blobs: BTreeMap<String, Vec<u8>>,
}

impl CompressionCache {
    /// Create a new cache instance.
    /// If `cache_dir` is `None`, caching is disabled and compression is performed directly.
    pub fn new(cache_dir: Option<PathBuf>, level: i32) -> Self {
        if let Some(dir) = &cache_dir {
            let _ = fs::create_dir_all(dir);
        }
        Self {
            cache_dir,
            level,
            used_files: HashSet::new(),
            cache_hits: 0,
            misses: 0,
            dedup_hits: 0,
            dedup_saved_bytes: 0,
            blobs: BTreeMap::new(),
        }
    }

    /// Compress data, using in-memory dedup and disk cache if available.
    /// Returns a `BlobInfo` with the content hash and compressed size.
    pub fn compress(&mut self, data: &[u8]) -> BlobInfo {
        let hash = blake3::hash(data).to_hex().to_string();

        // 1. In-memory dedup hit
        if let Some(existing) = self.blobs.get(&hash) {
            self.dedup_hits += 1;
            self.dedup_saved_bytes += existing.len();
            return BlobInfo {
                compressed_len: existing.len(),
                hash,
            };
        }

        // 2. Load from disk cache or compress fresh
        let compressed = self.load_or_compress(data, &hash);
        let compressed_len = compressed.len();
        self.blobs.insert(hash.clone(), compressed);
        BlobInfo {
            compressed_len,
            hash,
        }
    }

    /// Try to load compressed data from disk cache, or compress fresh.
    fn load_or_compress(&mut self, data: &[u8], hash: &str) -> Vec<u8> {
        if let Some(cache_dir) = &self.cache_dir {
            let cache_filename = format!("{hash}_{}.zst", self.level);
            let cache_path = cache_dir.join(&cache_filename);
            self.used_files.insert(cache_filename);

            if let Ok(cached) = fs::read(&cache_path)
                && is_current_blob(&cached, data.len())
            {
                self.cache_hits += 1;
                return cached;
            }

            self.misses += 1;
            let compressed = self.compress_raw(data);

            // Atomic write: write to tmp file then rename
            let tmp_path = cache_dir.join(format!(".tmp_{}", std::process::id()));
            if fs::write(&tmp_path, &compressed).is_ok() {
                let _ = fs::rename(&tmp_path, &cache_path);
            }

            compressed
        } else {
            self.misses += 1;
            self.compress_raw(data)
        }
    }

    /// Generate static declarations for all unique blobs.
    /// Each blob becomes: `static BLOB_{hash}: [u8; N] = *b"...";`
    /// BTreeMap ordering guarantees reproducible builds.
    pub fn dedup_statics(&self) -> Vec<proc_macro2::TokenStream> {
        self.blobs
            .iter()
            .map(|(hash, data)| {
                let ident = quote::format_ident!("BLOB_{}", hash);
                let len = data.len();
                let bytes_literal = syn::LitByteStr::new(data, proc_macro2::Span::call_site());
                quote::quote! {
                    static #ident: [u8; #len] = *#bytes_literal;
                }
            })
            .collect()
    }

    /// Remove cache files that were not used in this build.
    pub fn cleanup(&self) {
        let Some(cache_dir) = &self.cache_dir else {
            return;
        };

        let entries = match fs::read_dir(cache_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("zst")
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|name| !self.used_files.contains(name))
            {
                let _ = fs::remove_file(&path);
            }
        }
    }

    /// Log compression summary with cache hit/miss stats and dedup info.
    pub fn log_summary(&self) {
        let total = self.cache_hits + self.misses + self.dedup_hits;
        let unique = self.blobs.len();
        if self.cache_dir.is_some() {
            eprintln!(
                "typst-bake: Compression level {}, {total} files, {unique} unique blobs ({} cached, {} compressed)",
                self.level, self.cache_hits, self.misses
            );
        } else {
            eprintln!(
                "typst-bake: Compression level {}, {total} files, {unique} unique blobs (cache disabled)",
                self.level
            );
        }
        if self.dedup_hits > 0 {
            eprintln!(
                "typst-bake: Dedup: removed {} duplicates, saved {}",
                self.dedup_hits,
                format_size(self.dedup_saved_bytes)
            );
        }
    }

    /// Return deduplication statistics for this build.
    pub fn dedup_summary(&self) -> DedupSummary {
        DedupSummary {
            total_files: self.cache_hits + self.misses + self.dedup_hits,
            unique_blobs: self.blobs.len(),
            duplicate_count: self.dedup_hits,
            saved_bytes: self.dedup_saved_bytes,
        }
    }

    fn compress_raw(&self, data: &[u8]) -> Vec<u8> {
        // With the size known, zstd scales its window and tables down to the input, which is far
        // faster and lighter for small files than the streaming API (whose window is 8 MB at
        // level 19 and 128 MB at level 22). The size is also recorded in the frame header, so
        // decoders can allocate the output up front.
        let mut blob = zstd::bulk::compress(data, self.level).expect("zstd compression failed");
        // Allocated with `compress_bound(len)` and kept for the whole macro expansion.
        blob.shrink_to_fit();
        blob
    }
}

/// Whether `blob` is one complete zstd frame recording `len` as its content size.
/// Files cached by typst-bake 0.2.0 and earlier lack the size and are recompressed.
fn is_current_blob(blob: &[u8], len: usize) -> bool {
    use zstd::zstd_safe::{find_frame_compressed_size, get_frame_content_size};
    matches!(get_frame_content_size(blob), Ok(Some(n)) if n == len as u64)
        && find_frame_compressed_size(blob) == Ok(blob.len())
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &[u8] = b"#set page(width: 10cm)\nHello, typst-bake!\n";

    fn temp_cache_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "typst-bake-compression-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn blobs_record_their_size() {
        let mut cache = CompressionCache::new(None, 19);
        for data in [DATA, b""] {
            let info = cache.compress(data);
            assert!(is_current_blob(&cache.blobs[&info.hash], data.len()));
        }
    }

    #[test]
    fn outdated_cache_files_are_recompressed() {
        let dir = temp_cache_dir("outdated");
        let hash = blake3::hash(DATA).to_hex().to_string();
        let path = dir.join(format!("{hash}_19.zst"));

        // A frame without the recorded size, as written by earlier versions.
        let mut cache = CompressionCache::new(Some(dir.clone()), 19);
        let outdated = zstd::encode_all(DATA, 19).unwrap();
        assert!(!is_current_blob(&outdated, DATA.len()));
        fs::write(&path, &outdated).unwrap();
        cache.compress(DATA);
        assert_eq!((cache.cache_hits, cache.misses), (0, 1));
        let rewritten = fs::read(&path).unwrap();

        // The rewritten file is used by the next build.
        let mut cache = CompressionCache::new(Some(dir.clone()), 19);
        cache.compress(DATA);
        let _ = fs::remove_dir_all(&dir);
        assert_eq!((cache.cache_hits, cache.misses), (1, 0));
        assert!(is_current_blob(&rewritten, DATA.len()));
    }

    #[test]
    fn truncated_blobs_are_rejected() {
        let blob = zstd::bulk::compress(DATA, 19).unwrap();
        assert!(is_current_blob(&blob, DATA.len()));
        assert!(!is_current_blob(&blob[..blob.len() - 1], DATA.len()));
        assert!(!is_current_blob(&blob, DATA.len() + 1));
    }
}
