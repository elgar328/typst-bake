//! Font embedding with metadata precomputed at build time.
//!
//! Each font file is compressed like any other embedded file. In addition, the
//! `FontInfo` of every face is computed here and embedded as JSON, so the runtime
//! can build its font book without decompressing any font, and only decompress the
//! fonts a document actually uses.

use crate::compression_cache::CompressionCache;
use crate::config::{is_font_file, is_hidden};
use crate::dir_embed::tracking_path;
use proc_macro2::TokenStream;
use quote::quote;
use std::fs;
use std::path::{Path, PathBuf};
use typst_library::foundations::Bytes;
use typst_library::text::{Font, FontInfo};

/// Result of embedding the fonts directory.
pub struct FontEmbedResult {
    /// One `EmbeddedFont` expression per font file, in font book order.
    pub fonts: Vec<TokenStream>,
    /// Original uncompressed size in bytes.
    pub original_size: usize,
    /// Compressed size in bytes.
    pub compressed_size: usize,
    /// Number of font files embedded.
    pub file_count: usize,
}

/// Embed every font file under `dir` together with its face metadata.
pub fn embed_fonts(dir: &Path, cache: &mut CompressionCache) -> FontEmbedResult {
    let mut result = FontEmbedResult {
        fonts: Vec::new(),
        original_size: 0,
        compressed_size: 0,
        file_count: 0,
    };

    for path in collect_font_paths(dir) {
        let bytes = fs::read(&path).unwrap_or_else(|e| {
            panic!("Failed to read file {}: {e}", path.display());
        });

        let blob_info = cache.compress(&bytes);
        result.original_size += bytes.len();
        result.compressed_size += blob_info.compressed_len;
        result.file_count += 1;

        let blob_ident = quote::format_ident!("BLOB_{}", blob_info.hash);
        let abs_path = tracking_path(&path);
        let data = quote! {
            {
                // Cargo file tracking (not used at runtime)
                const _: &[u8] = include_bytes!(#abs_path);
                &#blob_ident
            }
        };

        result.fonts.push(match face_metadata(bytes) {
            Some(faces) => {
                let faces = faces.iter().map(|(index, info)| {
                    quote! {
                        ::typst_bake::__internal::EmbeddedFace { index: #index, info: #info }
                    }
                });
                quote! { ::typst_bake::__internal::EmbeddedFont::new(#data, &[#(#faces),*]) }
            }
            None => {
                eprintln!(
                    "typst-bake: Font metadata for {} cannot be precomputed; it will be read at runtime",
                    path.display()
                );
                quote! { ::typst_bake::__internal::EmbeddedFont::without_metadata(#data) }
            }
        });
    }

    result
}

/// List font files under `dir` in font book order.
///
/// Entries are sorted by path; within each directory, files come before
/// sub-directories, so a flat fonts directory keeps the order it always had.
/// Hidden entries are skipped. Like the other embedded directories, symlinks are
/// followed.
fn collect_font_paths(dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    visit(dir, &mut paths);
    paths
}

fn visit(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };

    let mut entries: Vec<PathBuf> = read_dir
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| !is_hidden(p))
        .collect();
    entries.sort();

    let (files, dirs): (Vec<_>, Vec<_>) = entries.into_iter().partition(|p| p.is_file());
    out.extend(files.into_iter().filter(|p| is_font_file(p)));
    for sub in dirs.into_iter().filter(|p| p.is_dir()) {
        visit(&sub, out);
    }
}

/// Compute `(face index, FontInfo JSON)` for every face typst can load from `data`.
///
/// Returns `None` if any face's metadata does not survive a JSON round trip
/// unchanged; the runtime then reads that file's metadata itself.
fn face_metadata(data: Vec<u8>) -> Option<Vec<(u32, String)>> {
    // `Font::iter` skips faces it cannot parse, so keep each font's own index.
    Font::iter(Bytes::new(data))
        .map(|font| {
            let json = serde_json::to_string(font.info()).ok()?;
            let back: FontInfo = serde_json::from_str(&json).ok()?;
            (back == *font.info()).then_some((font.index(), json))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_fonts_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/fonts")
    }

    #[test]
    fn collects_nested_fonts_after_top_level_files() {
        let root = std::env::temp_dir().join(format!("typst-bake-fonts-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("a-nested")).unwrap();
        fs::create_dir_all(root.join(".hidden-dir")).unwrap();

        let font = fs::read(example_fonts_dir().join("JetBrainsMono-Regular.otf")).unwrap();
        fs::write(root.join("z-top.otf"), &font).unwrap();
        fs::write(root.join("a-nested/inner.otf"), &font).unwrap();
        fs::write(root.join(".hidden.otf"), &font).unwrap();
        fs::write(root.join(".hidden-dir/skipped.otf"), &font).unwrap();
        fs::write(root.join("notes.txt"), b"not a font").unwrap();

        let found = collect_font_paths(&root);
        let _ = fs::remove_dir_all(&root);

        // The top-level file sorts after the directory name but still comes first.
        assert_eq!(
            found,
            vec![root.join("z-top.otf"), root.join("a-nested/inner.otf")]
        );
    }

    #[test]
    fn example_fonts_have_lossless_metadata() {
        let paths = collect_font_paths(&example_fonts_dir());
        assert!(!paths.is_empty());
        for path in paths {
            let faces = face_metadata(fs::read(&path).unwrap())
                .unwrap_or_else(|| panic!("{} fell back to runtime metadata", path.display()));
            assert!(!faces.is_empty(), "{} has no faces", path.display());
        }
    }
}
