//! Compression caching to avoid re-compressing unchanged files.

use std::collections::HashSet;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

/// Caches zstd-compressed results on disk, keyed by content hash and compression level.
pub struct CompressionCache {
    cache_dir: Option<PathBuf>,
    level: i32,
    used_files: HashSet<String>,
    hits: usize,
    misses: usize,
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
            hits: 0,
            misses: 0,
        }
    }

    /// Compress data, using the cache if available.
    pub fn compress(&mut self, data: &[u8]) -> Vec<u8> {
        let Some(cache_dir) = &self.cache_dir else {
            return self.compress_raw(data);
        };

        let hash = blake3::hash(data).to_hex().to_string();
        let cache_filename = format!("{}_{}.zst", hash, self.level);
        let cache_path = cache_dir.join(&cache_filename);

        self.used_files.insert(cache_filename);

        // Cache hit
        if let Ok(cached) = fs::read(&cache_path) {
            self.hits += 1;
            return cached;
        }

        // Cache miss: compress and store
        self.misses += 1;
        let compressed = self.compress_raw(data);

        // Atomic write: write to tmp file then rename
        let tmp_path = cache_dir.join(format!(".tmp_{}", std::process::id()));
        if fs::write(&tmp_path, &compressed).is_ok() {
            let _ = fs::rename(&tmp_path, &cache_path);
        }

        compressed
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

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("zst") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !self.used_files.contains(name) {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        }
    }

    /// Log compression summary with cache hit/miss stats.
    pub fn log_summary(&self) {
        let total = self.hits + self.misses;
        if self.cache_dir.is_some() {
            eprintln!(
                "typst-bake: Compression level {}, {} files ({} cached, {} compressed)",
                self.level, total, self.hits, self.misses
            );
        } else {
            eprintln!(
                "typst-bake: Compression level {}, {} files (cache disabled)",
                self.level, total
            );
        }
    }

    fn compress_raw(&self, data: &[u8]) -> Vec<u8> {
        zstd::encode_all(Cursor::new(data), self.level).expect("zstd compression failed")
    }
}
