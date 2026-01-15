//! Compression statistics for embedded files

/// Compression statistics for all embedded content
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
