# typst-bake

[![Crates.io](https://img.shields.io/crates/v/typst-bake.svg)](https://crates.io/crates/typst-bake)
[![Documentation](https://docs.rs/typst-bake/badge.svg)](https://docs.rs/typst-bake)
[![CI](https://github.com/elgar328/typst-bake/actions/workflows/ci.yml/badge.svg)](https://github.com/elgar328/typst-bake/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/typst-bake.svg)](https://github.com/elgar328/typst-bake#license)

Bake Typst templates, fonts, and packages into your Rust binary — use Typst as a self-contained, embedded library.

## Features

- **Multi-Format Output** - Generate PDF, SVG, or PNG from the same template
- **Simple API** - Generate documents with just `document!("main.typ").to_pdf()`
- **Minimal Setup** - Just specify `template-dir` and `fonts-dir` in `Cargo.toml` metadata
- **File Embedding** - All files in `template-dir` are embedded and accessible from templates
- **Font Embedding** - Fonts (TTF, OTF, TTC) in `fonts-dir` are automatically bundled into the binary
- **Automatic Package Bundling** - Scans for package imports, downloads them at compile time, and recursively resolves all dependencies
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

### Cargo Features

| Feature | Description |
|---------|-------------|
| `pdf` (default) | Enable `to_pdf()` |
| `svg` | Enable `to_svg()` |
| `png` | Enable `to_png()` |
| `full` | Enable all output formats |

PDF works out of the box. To disable PDF and use only SVG: `default-features = false, features = ["svg"]`.

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
    let doc = typst_bake::document!("main.typ");

    // Generate PDF
    let pdf = doc.to_pdf()?;
    std::fs::write("output.pdf", &pdf)?;

    // Generate SVG
    let svgs = doc.to_svg()?;
    std::fs::write("page1.svg", &svgs[0])?;

    // Generate PNG at 144 DPI
    let pngs = doc.to_png(144.0)?;
    std::fs::write("page1.png", &pngs[0])?;

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
