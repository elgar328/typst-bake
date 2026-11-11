//! Embedded fonts with build-time metadata, decompressed lazily on first use.
//!
//! The macro embeds every font file compressed, together with the `FontInfo` of each
//! face as JSON. The font book is built from that metadata alone; a font file is only
//! decompressed and parsed when typst first asks for one of its faces. The decoded
//! state lives in a `static` emitted by the macro, so it is shared by every
//! [`Document`](crate::Document) created from the same `document!` call site.

use crate::error::Error;
use crate::util::decompress;
use std::sync::OnceLock;
use typst::foundations::Bytes;
use typst::text::{Font, FontBook, FontInfo};
use typst::utils::LazyHash;

/// A compressed font file and the metadata of its faces. Built by the macro.
#[doc(hidden)]
pub struct EmbeddedFont {
    /// zstd-compressed font file.
    data: &'static [u8],
    /// Metadata of every face typst can load, or `None` if it could not be
    /// precomputed (it is then read from the font at runtime).
    faces: Option<&'static [EmbeddedFace]>,
}

impl EmbeddedFont {
    /// A font file with precomputed face metadata.
    pub const fn new(data: &'static [u8], faces: &'static [EmbeddedFace]) -> Self {
        Self {
            data,
            faces: Some(faces),
        }
    }

    /// A font file whose metadata is read at runtime.
    pub const fn without_metadata(data: &'static [u8]) -> Self {
        Self { data, faces: None }
    }
}

/// One face of an [`EmbeddedFont`]. Built by the macro.
#[doc(hidden)]
pub struct EmbeddedFace {
    /// Index of the face within its font file.
    pub index: u32,
    /// The face's `FontInfo`, serialized as JSON.
    pub info: &'static str,
}

/// The embedded fonts of one `document!` call site, decoded on demand.
#[doc(hidden)]
pub struct FontSet {
    fonts: &'static [EmbeddedFont],
    store: OnceLock<FontStore>,
}

impl FontSet {
    /// Internal constructor used by the macro.
    pub const fn new(fonts: &'static [EmbeddedFont]) -> Self {
        Self {
            fonts,
            store: OnceLock::new(),
        }
    }

    pub(crate) fn store(&self) -> &FontStore {
        self.store.get_or_init(|| FontStore::new(self.fonts))
    }
}

/// Font book plus lazily decoded fonts, indexed like the book.
pub(crate) struct FontStore {
    book: LazyHash<FontBook>,
    faces: Vec<FaceSlot>,
    files: Vec<FileSlot>,
}

struct FileSlot {
    data: &'static [u8],
    decoded: OnceLock<Result<Bytes, std::io::Error>>,
}

struct FaceSlot {
    file: usize,
    index: u32,
    font: OnceLock<Option<Font>>,
}

impl FileSlot {
    /// Decompress the file once; faces of the same file share the result.
    fn decode(&self) -> Result<&Bytes, &std::io::Error> {
        self.decoded
            .get_or_init(|| decompress(self.data).map(Bytes::new))
            .as_ref()
    }
}

impl FontStore {
    fn new(fonts: &'static [EmbeddedFont]) -> Self {
        let mut book = FontBook::new();
        let mut faces = Vec::new();
        let mut files = Vec::with_capacity(fonts.len());

        for (file, font) in fonts.iter().enumerate() {
            let slot = FileSlot {
                data: font.data,
                decoded: OnceLock::new(),
            };

            // Metadata that does not parse (only possible if the macros used another
            // typst-library, which a test guards against) is treated as missing.
            let infos: Option<Vec<(u32, FontInfo)>> = font.faces.and_then(|embedded| {
                embedded
                    .iter()
                    .map(|face| Some((face.index, serde_json::from_str(face.info).ok()?)))
                    .collect()
            });

            match infos {
                Some(infos) => {
                    for (index, info) in infos {
                        book.push(info);
                        faces.push(FaceSlot {
                            file,
                            index,
                            font: OnceLock::new(),
                        });
                    }
                }
                None => {
                    // No usable metadata: decode now. A failure is kept in the slot
                    // and reported by `decode_error`.
                    if let Ok(bytes) = slot.decode() {
                        for font in Font::iter(bytes.clone()) {
                            book.push(font.info().clone());
                            faces.push(FaceSlot {
                                file,
                                index: font.index(),
                                font: OnceLock::from(Some(font)),
                            });
                        }
                    }
                }
            }

            files.push(slot);
        }

        Self {
            book: LazyHash::new(book),
            faces,
            files,
        }
    }

    pub(crate) fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    /// The font at book index `index`, decoding its file on first use.
    ///
    /// Out-of-range indices yield `None`; typst may pass indices from an outdated
    /// book during incremental compilation.
    pub(crate) fn font(&self, index: usize) -> Option<Font> {
        let face = self.faces.get(index)?;
        face.font
            .get_or_init(|| {
                let bytes = self.files[face.file].decode().ok()?;
                Font::new(bytes.clone(), face.index)
            })
            .clone()
    }

    /// The first decompression failure among the fonts decoded so far.
    ///
    /// A failure stays in the process-wide store, so every later compile from the
    /// same call site reports it too (embedded data cannot change at runtime).
    pub(crate) fn decode_error(&self) -> Option<Error> {
        self.files.iter().find_map(|file| match file.decoded.get() {
            Some(Err(e)) => Some(Error::Decompression(std::io::Error::new(
                e.kind(),
                e.to_string(),
            ))),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use typst::text::{AxisValue, Coverage, FontAxis, FontFlags, FontStyle, FontVariant, Tag};

    fn leak<T>(items: Vec<T>) -> &'static [T] {
        Box::leak(items.into_boxed_slice())
    }

    fn leak_str(s: String) -> &'static str {
        Box::leak(s.into_boxed_str())
    }

    /// Embed the example fonts the way the macro does.
    fn example_fonts() -> &'static [EmbeddedFont] {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/fonts");
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e == "otf" || e == "ttf" || e == "ttc")
            })
            .collect();
        paths.sort();

        let fonts = paths
            .iter()
            .map(|path| {
                let raw = std::fs::read(path).unwrap();
                let faces = Font::iter(Bytes::new(raw.clone()))
                    .map(|font| EmbeddedFace {
                        index: font.index(),
                        info: leak_str(serde_json::to_string(font.info()).unwrap()),
                    })
                    .collect();
                EmbeddedFont::new(leak(zstd::encode_all(&raw[..], 3).unwrap()), leak(faces))
            })
            .collect();
        leak(fonts)
    }

    fn decoded_files(store: &FontStore) -> usize {
        store
            .files
            .iter()
            .filter(|f| f.decoded.get().is_some())
            .count()
    }

    #[test]
    fn metadata_round_trips_through_json() {
        let mut infos: Vec<FontInfo> = example_fonts()
            .iter()
            .flat_map(|f| f.faces.unwrap())
            .map(|face| serde_json::from_str(face.info).unwrap())
            .collect();

        // Exercise the fields the example fonts leave empty: axes and a non-ASCII family.
        infos.push(FontInfo {
            family: "본고딕 Variable".into(),
            variant: FontVariant {
                style: FontStyle::Italic,
                ..FontVariant::default()
            },
            flags: FontFlags::VARIABLE | FontFlags::SERIF,
            axes: vec![FontAxis {
                tag: Tag::from_bytes(b"wght"),
                min: AxisValue(100.0),
                max: AxisValue(900.0),
                default: AxisValue(400.0),
            }],
            coverage: Coverage::from_vec(vec![0x41, 0x42, 0xAC00, 0xD7A3]),
        });

        for info in infos {
            let json = serde_json::to_string(&info).unwrap();
            assert_eq!(serde_json::from_str::<FontInfo>(&json).unwrap(), info);
        }
    }

    #[test]
    fn fonts_are_decoded_on_first_use_only() {
        let fonts = example_fonts();
        let store = FontStore::new(fonts);

        let face_count: usize = fonts.iter().map(|f| f.faces.unwrap().len()).sum();
        assert_eq!(store.faces.len(), face_count);
        assert!(store.book().info(face_count - 1).is_some());
        assert_eq!(decoded_files(&store), 0);

        let font = store.font(0).expect("first face loads");
        assert_eq!(decoded_files(&store), 1);
        assert_eq!(font.info(), store.book().info(0).unwrap());
        assert!(store.font(face_count).is_none());
        assert!(store.decode_error().is_none());
    }

    #[test]
    fn faces_of_one_file_share_its_decoded_data() {
        let example = &example_fonts()[0];
        let face = &example.faces.unwrap()[0];
        let same = || EmbeddedFace {
            index: face.index,
            info: face.info,
        };
        let twice = leak(vec![same(), same()]);
        let store = FontStore::new(leak(vec![EmbeddedFont::new(example.data, twice)]));

        let a = store.font(0).unwrap();
        let b = store.font(1).unwrap();
        assert_eq!(a.data().as_ptr(), b.data().as_ptr());
    }

    #[test]
    fn missing_metadata_is_read_at_runtime() {
        let example = &example_fonts()[0];
        let store = FontStore::new(leak(vec![EmbeddedFont::without_metadata(example.data)]));

        assert_eq!(store.faces.len(), example.faces.unwrap().len());
        assert!(store.font(0).is_some());
    }

    #[test]
    fn unparsable_metadata_is_read_at_runtime() {
        let example = &example_fonts()[0];
        let bogus = leak(vec![EmbeddedFace {
            index: 0,
            info: "{\"not\": \"FontInfo\"}",
        }]);
        let store = FontStore::new(leak(vec![EmbeddedFont::new(example.data, bogus)]));

        assert_eq!(store.faces.len(), example.faces.unwrap().len());
        let font = store.font(0).expect("face decoded from the font itself");
        assert_eq!(font.info(), store.book().info(0).unwrap());
    }

    #[test]
    fn corrupt_font_data_reports_decompression_error() {
        let example = &example_fonts()[0];
        let store = FontStore::new(leak(vec![EmbeddedFont::new(
            b"not zstd",
            example.faces.unwrap(),
        )]));

        assert!(store.decode_error().is_none());
        assert!(store.font(0).is_none());
        assert!(matches!(
            store.decode_error(),
            Some(Error::Decompression(_))
        ));
    }

    /// The macros serialize font metadata with their own `typst-library`; the runtime
    /// reads it with the one `typst` uses. Both must be the same package.
    #[test]
    fn single_typst_library_in_lockfile() {
        let lock = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.lock");
        let Ok(lock) = std::fs::read_to_string(lock) else {
            return; // Not in the workspace (e.g. a published crate).
        };
        let count = lock
            .lines()
            .filter(|l| l.trim() == r#"name = "typst-library""#)
            .count();
        assert_eq!(
            count, 1,
            "Cargo.lock has {count} typst-library versions; keep the workspace \
             `typst` and `typst-library` dependencies on the same minor version"
        );
    }
}
