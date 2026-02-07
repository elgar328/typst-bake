High compression levels produce smaller binaries with little impact on decompression speed, but they take significantly longer to compress. Recompressing unchanged files on every build would be wasteful, so `typst-bake` *caches compressed outputs on disk*. The cache key is derived from:

- The *BLAKE3 hash* of the file contents
- The *compression level* in use

If both match, the previously compressed result is reused — no recompression occurs.

```
target/typst-bake-cache/{package-name}/
├── a1b2c3d4e5f6...zst   (cached compressed blob)
├── f7e8d9c0b1a2...zst
└── ...
```

This makes *incremental builds fast* even at high compression levels.
