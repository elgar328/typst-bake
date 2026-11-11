//! zstd decompression of embedded resources.
//!
//! Native targets use the C `zstd` library. WebAssembly targets use the pure-Rust `ruzstd`
//! decoder, so building for wasm needs no C toolchain for the target.

/// Decompress zstd-compressed data.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn decompress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    zstd::decode_all(data)
}

/// Decompress zstd-compressed data.
#[cfg(target_family = "wasm")]
pub(crate) fn decompress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    decompress_pure_rust(data)
}

#[cfg(any(target_family = "wasm", test))]
fn decompress_pure_rust(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    use std::io::{Error, ErrorKind, Read};

    let mut decoder = ruzstd::decoding::StreamingDecoder::new(data)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
    // Size the output from the frame header, like `zstd::decode_all`. A failed reservation
    // is not fatal; the Vec then grows as usual.
    let mut out = Vec::new();
    let _ = out
        .try_reserve_exact(usize::try_from(decoder.decoder.content_size()).unwrap_or(usize::MAX));
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::path::Path;

    #[test]
    fn decoders_agree() {
        let font = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../examples/fonts/SourceSerif4-Regular.ttf"),
        )
        .unwrap();
        let inputs: [&[u8]; 3] = [b"", b"#set page(width: 10cm)\nHello", &font];

        for input in inputs {
            for level in [1, 19, 22] {
                // Frames with and without a recorded content size.
                let frames = [
                    zstd::bulk::compress(input, level).unwrap(),
                    zstd::encode_all(input, level).unwrap(),
                ];
                for frame in frames {
                    assert_eq!(decompress_pure_rust(&frame).unwrap(), input);
                    assert_eq!(decompress(&frame).unwrap(), input);
                }
            }
        }
    }

    #[test]
    fn invalid_data_is_an_error() {
        let err = decompress_pure_rust(b"not zstd").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert!(decompress(b"not zstd").is_err());
    }
}
