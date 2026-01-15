# typst-bake

[![Crates.io](https://img.shields.io/crates/v/typst-bake.svg)](https://crates.io/crates/typst-bake)
[![Documentation](https://docs.rs/typst-bake/badge.svg)](https://docs.rs/typst-bake)
[![License](https://img.shields.io/crates/l/typst-bake.svg)](https://github.com/elgar328/typst-bake#license)

Bake Typst templates, fonts, and packages into your Rust binary to create a fully self-contained PDF generation engine with zero runtime dependencies on the filesystem or network.

## Features

- **Simple API** - Generate PDFs with just `document!("main.typ").to_pdf()`
- **Minimal Setup** - Just specify `template-dir` and `fonts-dir` in `Cargo.toml` metadata
- **File Embedding** - All files in `template-dir` are embedded and accessible from templates
- **Font Embedding** - Fonts (TTF, OTF, TTC) in `fonts-dir` are automatically bundled into the binary
- **Automatic Package Bundling** - Scans templates for package imports, downloads them at compile time, and recursively resolves all dependencies
- **Package Caching** - Downloaded packages are cached in system cache directory for faster compilation. Use `TYPST_BAKE_REFRESH=1` to force re-download
- **Runtime Inputs** - Pass dynamic data from Rust structs to Typst via `IntoValue` / `IntoDict` derive macros
- **Optimized Binary Size** - Resources are compressed with zstd and decompressed lazily at runtime
- **Smart Recompilation** - File changes trigger recompilation automatically, with optional build script for complete coverage

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
typst-bake = "0.1"

[package.metadata.typst-bake]
template-dir = "./templates"  # Path to your .typ files and assets
fonts-dir = "./fonts"         # Path to your font files
```

### Optional: Complete File Change Detection

By default, `typst-bake` detects template or font file **modifications** and triggers recompilation when you run `cargo build`. File **additions and deletions** are not detected directly, but this is rarely an issue—adding a new file usually requires modifying an existing file (like `main.typ`) to use it, which triggers recompilation anyway.

If you want to fully detect even the rare case where you only add or remove files without modifying existing ones, add a build script:

```toml
# Cargo.toml
[package]
build = "build.rs"

[build-dependencies]
typst-bake = "0.1"
```

```rust
// build.rs
fn main() {
    typst_bake::rebuild_if_changed();
}
```

## Quick Start

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf = typst_bake::document!("main.typ").to_pdf()?;
    std::fs::write("output.pdf", &pdf)?;
    Ok(())
}
```

For a complete walkthrough, see the [Quick Start Guide (PDF)](https://github.com/elgar328/typst-bake/blob/main/examples/quick-start/output.pdf).

## Examples

| Example | Description | Command | Output |
|---------|-------------|---------|--------|
| quick-start | Generates the Quick Start PDF using all features | `cargo run -p example-quick-start` | [PDF](https://github.com/elgar328/typst-bake/blob/main/examples/quick-start/output.pdf) |
| font-guide | Guide to font setup and usage | `cargo run -p example-font-guide` | [PDF](https://github.com/elgar328/typst-bake/blob/main/examples/font-guide/output.pdf) |
| with-inputs | Pass dynamic data from Rust to Typst | `cargo run -p example-with-inputs` | [PDF](https://github.com/elgar328/typst-bake/blob/main/examples/with-inputs/output.pdf) |
| with-files | Embed images and various data files | `cargo run -p example-with-files` | [PDF](https://github.com/elgar328/typst-bake/blob/main/examples/with-files/output.pdf) |
| with-package | Automatic package bundling | `cargo run -p example-with-package` | [PDF](https://github.com/elgar328/typst-bake/blob/main/examples/with-package/output.pdf) |

## License

MIT OR Apache-2.0
