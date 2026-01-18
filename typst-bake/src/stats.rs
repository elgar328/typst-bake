//! Compression statistics for embedded files.
//!
//! All embedded resources (templates, fonts, packages) are compressed with zstd
//! and decompressed lazily at runtime.

/// Compression statistics for all embedded content.
///
/// Resources are compressed with zstd at compile time and decompressed lazily at runtime.
#[derive(Debug, Clone)]
pub struct EmbedStats {
    /// Template files statistics
    pub templates: CategoryStats,
    /// Package files statistics
    pub packages: PackageStats,
    /// Font files statistics
    pub fonts: CategoryStats,
}

/// Statistics for a category of files (templates, fonts)
#[derive(Debug, Clone)]
pub struct CategoryStats {
    /// Original uncompressed size in bytes
    pub original_size: usize,
    /// Compressed size in bytes
    pub compressed_size: usize,
    /// Number of files
    pub file_count: usize,
}

/// Statistics for all packages
#[derive(Debug, Clone)]
pub struct PackageStats {
    /// Per-package statistics
    pub packages: Vec<PackageInfo>,
    /// Total original size of all packages
    pub total_original: usize,
    /// Total compressed size of all packages
    pub total_compressed: usize,
}

/// Statistics for a single package
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// Package name with version (e.g., "gentle-clues:1.2.0")
    pub name: String,
    /// Original uncompressed size in bytes
    pub original_size: usize,
    /// Compressed size in bytes
    pub compressed_size: usize,
    /// Number of files in this package
    pub file_count: usize,
}

impl EmbedStats {
    /// Calculate total original size across all categories
    pub fn total_original(&self) -> usize {
        self.templates.original_size + self.packages.total_original + self.fonts.original_size
    }

    /// Calculate total compressed size across all categories
    pub fn total_compressed(&self) -> usize {
        self.templates.compressed_size + self.packages.total_compressed + self.fonts.compressed_size
    }

    /// Calculate compression ratio (0.0 to 1.0, where 0.0 means no compression)
    pub fn compression_ratio(&self) -> f64 {
        let original = self.total_original();
        if original == 0 {
            return 0.0;
        }
        1.0 - (self.total_compressed() as f64 / original as f64)
    }

    /// Display compression statistics in a human-readable format
    pub fn display(&self) {
        println!("Compression Statistics:");
        println!("========================");

        // Templates
        if self.templates.file_count > 0 {
            println!(
                "Templates:  {:>9} -> {:>9} ({:>5.1}% reduced, {} files)",
                format_size(self.templates.original_size),
                format_size(self.templates.compressed_size),
                self.templates.compression_ratio() * 100.0,
                self.templates.file_count
            );
        }

        // Fonts
        if self.fonts.file_count > 0 {
            println!(
                "Fonts:      {:>9} -> {:>9} ({:>5.1}% reduced, {} files)",
                format_size(self.fonts.original_size),
                format_size(self.fonts.compressed_size),
                self.fonts.compression_ratio() * 100.0,
                self.fonts.file_count
            );
        }

        // Packages
        if !self.packages.packages.is_empty() {
            println!("Packages:");

            // Calculate column widths for package alignment
            let name_width = self
                .packages
                .packages
                .iter()
                .map(|p| p.name.len())
                .max()
                .unwrap_or(0);
            let orig_width = self
                .packages
                .packages
                .iter()
                .map(|p| format_size(p.original_size).len())
                .max()
                .unwrap_or(0);
            let comp_width = self
                .packages
                .packages
                .iter()
                .map(|p| format_size(p.compressed_size).len())
                .max()
                .unwrap_or(0);

            for pkg in &self.packages.packages {
                println!(
                    "  {:<name_w$}  {:>orig_w$} -> {:>comp_w$}  ({:>5.1}%)",
                    pkg.name,
                    format_size(pkg.original_size),
                    format_size(pkg.compressed_size),
                    pkg.compression_ratio() * 100.0,
                    name_w = name_width,
                    orig_w = orig_width,
                    comp_w = comp_width,
                );
            }
        }

        // Total
        println!("------------------------");
        println!(
            "Total: {} -> {} ({:.1}% reduced)",
            format_size(self.total_original()),
            format_size(self.total_compressed()),
            self.compression_ratio() * 100.0
        );
    }
}

impl CategoryStats {
    /// Calculate compression ratio for this category
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            return 0.0;
        }
        1.0 - (self.compressed_size as f64 / self.original_size as f64)
    }
}

impl PackageInfo {
    /// Calculate compression ratio for this package
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            return 0.0;
        }
        1.0 - (self.compressed_size as f64 / self.original_size as f64)
    }
}

impl PackageStats {
    /// Calculate compression ratio for all packages
    pub fn compression_ratio(&self) -> f64 {
        if self.total_original == 0 {
            return 0.0;
        }
        1.0 - (self.total_compressed as f64 / self.total_original as f64)
    }
}

/// Format bytes into human-readable size
fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;

    if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(10240), "10.0 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1572864), "1.50 MB");
    }

    #[test]
    fn test_compression_ratio_zero_original() {
        let stats = CategoryStats {
            original_size: 0,
            compressed_size: 0,
            file_count: 0,
        };
        assert_eq!(stats.compression_ratio(), 0.0);
    }

    #[test]
    fn test_compression_ratio_75_percent() {
        // Asymmetric values to distinguish from incorrect calculation (original/compressed)
        // Correct: 1 - (250/1000) = 0.75
        // Wrong:   1 - (1000/250) = -3.0
        let stats = CategoryStats {
            original_size: 1000,
            compressed_size: 250,
            file_count: 1,
        };
        assert!((stats.compression_ratio() - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_embed_stats_totals() {
        let stats = EmbedStats {
            templates: CategoryStats {
                original_size: 1000,
                compressed_size: 200, // 80% compression
                file_count: 1,
            },
            fonts: CategoryStats {
                original_size: 2000,
                compressed_size: 600, // 70% compression
                file_count: 2,
            },
            packages: PackageStats {
                packages: vec![],
                total_original: 1000,
                total_compressed: 200, // 80% compression
            },
        };
        // Total: 4000 -> 1000 (75% compression)
        assert_eq!(stats.total_original(), 4000);
        assert_eq!(stats.total_compressed(), 1000);
        assert!((stats.compression_ratio() - 0.75).abs() < 0.001);
    }
}
