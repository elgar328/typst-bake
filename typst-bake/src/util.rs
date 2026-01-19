use std::io::Cursor;

/// Decompress zstd compressed data
pub(crate) fn decompress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    zstd::decode_all(Cursor::new(data))
}
