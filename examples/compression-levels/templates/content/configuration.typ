The default compression level is *19* (zstd supports levels 1 through 22).

You can set a custom level in `Cargo.toml` under `[package.metadata.typst-bake]`:

```toml
[package.metadata.typst-bake]
compression-level = 22
```

Alternatively, override the level at build time with an environment variable:

```bash
TYPST_BAKE_COMPRESSION_LEVEL=3 cargo build
```

The environment variable takes precedence over the `Cargo.toml` setting.

*Higher levels* produce smaller binaries but compress more slowly.
