// Time ruzstd decompression inside WebAssembly, for tests/benchmark.rs.
//
// Usage: node wasm-decode.mjs <decoder.wasm> <blob.zst> <expected-size> <warmup> <iterations>
// Prints the seconds of each timed run, comma-separated.

import fs from 'node:fs';

const [wasmPath, blobPath, expected, warmup, iterations] = process.argv.slice(2);
const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasmPath));
const { memory, bench_alloc, bench_decode } = instance.exports;

const blob = fs.readFileSync(blobPath);
const ptr = bench_alloc(blob.length);
new Uint8Array(memory.buffer, ptr, blob.length).set(blob);

// V8 first runs wasm in its baseline compiler and swaps in optimized code between calls,
// so the warmup runs make the timed runs use the optimized code.
const seconds = [];
for (let i = 0; i < Number(warmup) + Number(iterations); i++) {
  const start = performance.now();
  const size = bench_decode(ptr, blob.length);
  const elapsed = (performance.now() - start) / 1000;
  if (size !== Number(expected)) {
    throw new Error(`decoded ${size} bytes, expected ${expected}`);
  }
  if (i >= Number(warmup)) seconds.push(elapsed);
}
console.log(seconds.join(','));
