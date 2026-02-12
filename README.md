# typst-bake

<img src="https://github.com/user-attachments/assets/dbb1d712-c319-47b5-9724-21f3cd82bfea" alt="Banner" width="100%">

[![Crates.io](https://img.shields.io/crates/v/typst-bake.svg)](https://crates.io/crates/typst-bake)
[![Documentation](https://docs.rs/typst-bake/badge.svg)](https://docs.rs/typst-bake)
[![CI](https://github.com/elgar328/typst-bake/actions/workflows/ci.yml/badge.svg)](https://github.com/elgar328/typst-bake/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/typst-bake.svg)](https://github.com/elgar328/typst-bake#license)

Bake Typst templates, fonts, and packages into your Rust binary — use [Typst](https://typst.app) as a self-contained, embedded library.

## Features

- **Multi-Format Output** - Generate PDF, SVG, or PNG from the same template
- **Simple API** - Generate documents with just `document!("main.typ").to_pdf()`
- **Minimal Setup** - Just specify `template-dir` and `fonts-dir` in `Cargo.toml` metadata
- **File Embedding** - All files in `template-dir` are embedded and accessible from `.typ` files
- **Font Embedding** - Fonts (TTF, OTF, TTC) in `fonts-dir` are automatically bundled into the binary
- **Automatic Package Bundling** - Scans for package imports, downloads them at compile time, and recursively resolves all dependencies
- **Package Caching** - Downloaded packages are cached in system cache directory for faster compilation. Use `TYPST_BAKE_PKG_NOCACHE=1` to force re-download
- **Runtime Inputs** - Pass dynamic data from Rust structs to Typst via `IntoValue` / `IntoDict` derive macros
- **Optimized Binary Size** - Resources are deduplicated and compressed with zstd, then decompressed lazily at runtime
- **Compression Caching** - Compressed data is cached on disk to speed up incremental builds
- **Custom Compression Level** - Zstd compression level (1–22, default 19) is configurable via `compression-level` in Cargo.toml metadata
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

For a complete walkthrough, see the [Quick Start Guide (PDF)](https://elgar328.github.io/typst-bake/quick-start.pdf). Also check out the PDF outputs from other examples below—each document explains its usage in detail.

## How it Works

<a href="https://elgar328.github.io/typst-bake/architecture.pdf">
  <img src="https://github.com/user-attachments/assets/82f6e114-eaa0-4259-a073-ac84621074e1" alt="Architecture diagram">
</a>

## Examples

| Example | Description | Command | Output |
|---------|-------------|---------|--------|
| quick-start | Generates the Quick Start PDF using all features | `cargo run -p example-quick-start` | [PDF](https://elgar328.github.io/typst-bake/quick-start.pdf) |
| font-guide | Guide to font setup and usage | `cargo run -p example-font-guide` | [PDF](https://elgar328.github.io/typst-bake/font-guide.pdf) |
| with-inputs | Pass dynamic data from Rust to Typst | `cargo run -p example-with-inputs` | [PDF](https://elgar328.github.io/typst-bake/with-inputs.pdf) |
| with-files | Embed images and various data files | `cargo run -p example-with-files` | [PDF](https://elgar328.github.io/typst-bake/with-files.pdf) |
| with-package | Automatic package bundling | `cargo run -p example-with-package` | [PDF](https://elgar328.github.io/typst-bake/with-package.pdf) |
| compression-levels | Custom compression level with zstd benchmark | `cargo run -p example-compression-levels` | [PDF](https://elgar328.github.io/typst-bake/compression-levels.pdf) |
| output-formats | Multi-format output with rendering test patterns | `cargo run -p example-output-formats` | See below |

### output-formats Example Outputs

| Format | Files |
|--------|-------|
| PDF | [output.pdf](https://elgar328.github.io/typst-bake/output-formats.pdf) (47KB) |
| SVG | [output_1.svg](https://elgar328.github.io/typst-bake/output-formats_1.svg) (276KB), [output_2.svg](https://elgar328.github.io/typst-bake/output-formats_2.svg) (700KB) |
| PNG | [output_1.png](https://elgar328.github.io/typst-bake/output-formats_1.png) (397KB), [output_2.png](https://elgar328.github.io/typst-bake/output-formats_2.png) (695KB) |

## Comparison with typst-as-lib

**[typst-as-lib](https://github.com/Relacibo/typst-as-lib)** is a lightweight and flexible wrapper that makes it easy to use the Typst compiler as a Rust library. It supports various combinations of runtime filesystem access, package downloads from the internet, caching, and more.

**typst-bake** uses typst-as-lib internally, adding a layer focused on creating self-contained binaries. This focused scope enables a simple, easy-to-use API. It embeds all resources (templates, fonts, packages) into the binary at compile time, so the resulting executable works anywhere without external files or network access. For packages, the entire process—scanning, downloading, compressing, and embedding—is fully automatic.

### Key Differences

| Aspect | typst-as-lib | typst-bake |
|--------|--------------|------------|
| **Resources** | Runtime filesystem access or compile-time individual file embedding | Embeds entire folders at compile time |
| **Packages** | Runtime download (with caching) or local filesystem | Automatic scan, download, compress, and embed at compile time |
| **Fonts** | Typst default fonts, embedded fonts, system fonts, etc. | Embedded fonts only |
| **Configuration** | Flexible setup via builder pattern in code | Cargo.toml metadata only |
| **API** | Flexible with fine-grained control | Simple (`document!("main.typ").to_pdf()`) |

### Which should you use?

- If you want all resources embedded in your binary for a fully self-contained executable → use **typst-bake**
- If you prefer runtime flexibility (e.g., downloading packages on demand to reduce binary size) → use **typst-as-lib** directly

## License

MIT OR Apache-2.0
