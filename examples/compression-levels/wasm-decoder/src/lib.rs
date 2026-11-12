//! The ruzstd decoder as typst-bake runs it on WebAssembly, exported for
//! `tests/wasm-decode.mjs`, which the compression-levels benchmark runs in Node.js.
//!
//! Only meaningful on `wasm32`; on other targets the crate is empty.
#![cfg(target_arch = "wasm32")]

use std::io::Read;

/// Allocate `len` bytes for the host to copy the compressed input into. Never freed: the
/// module only lives for one benchmark level.
#[unsafe(no_mangle)]
pub extern "C" fn bench_alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// Decompress the zstd frame at `ptr..ptr + len` and return the decompressed length.
///
/// The same steps as `decompress_pure_rust` in `typst-bake/src/util.rs`.
///
/// # Safety
/// `ptr..ptr + len` must be initialized memory, e.g. from [`bench_alloc`] filled by the host.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bench_decode(ptr: *const u8, len: usize) -> usize {
    // SAFETY: guaranteed by the caller.
    let input = unsafe { std::slice::from_raw_parts(ptr, len) };
    let mut decoder = ruzstd::decoding::StreamingDecoder::new(input).expect("a zstd frame");
    let mut out = Vec::new();
    let _ = out
        .try_reserve_exact(usize::try_from(decoder.decoder.content_size()).unwrap_or(usize::MAX));
    decoder.read_to_end(&mut out).expect("valid zstd data");
    out.len()
}
