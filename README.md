# typst-bake

Bake Typst templates, fonts, and packages into your Rust binary to create a fully self-contained PDF generation engine with zero runtime dependencies on the filesystem or network.

## Features

- **Simple API** - Generate PDFs with just `document!("main.typ").to_pdf()`
- **Minimal Setup** - Just specify `template-dir` and `fonts-dir` in `Cargo.toml` metadata
- **File Embedding** - All files in `template-dir` are embedded and accessible from templates
- **Font Embedding** - Fonts (TTF, OTF, TTC) in `fonts-dir` are automatically bundled into the binary
- **Runtime Inputs** - Pass dynamic data from Rust structs to Typst via `IntoValue` / `IntoDict` derive macros
- **Automatic Package Bundling** - Scans templates for package imports, downloads them at compile time, and recursively resolves all dependencies
- **Package Caching** - Downloaded packages are cached in system cache directory for faster compilation. Use `TYPST_BAKE_REFRESH=1` to force re-download
- **Optimized Binary Size** - Resources are compressed with zstd and decompressed lazily at runtime

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
typst-bake = "0.1.0"

[package.metadata.typst-bake]
template-dir = "./templates"  # Path to your .typ files and assets
fonts-dir = "./fonts"         # Path to your font files
```

## Quick Start

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf = typst_bake::document!("main.typ").to_pdf()?;
    std::fs::write("output.pdf", &pdf)?;
    Ok(())
}
```

For detailed setup guide, see the [Quick Start Guide (PDF)](examples/quick-start/output.pdf).

## Examples

| Example | Description | Command | Output |
|---------|-------------|---------|--------|
| basic | Simplest font embedding example | `cargo run -p example-basic` | [PDF](examples/basic/output.pdf) |
| with-inputs | Pass dynamic data from Rust to Typst | `cargo run -p example-with-inputs` | [PDF](examples/with-inputs/output.pdf) |
| with-files | Embed images and various data files | `cargo run -p example-with-files` | [PDF](examples/with-files/output.pdf) |
| with-package | Automatic package bundling | `cargo run -p example-with-package` | [PDF](examples/with-package/output.pdf) |
| quick-start | Generates the Quick Start PDF using all features | `cargo run -p example-quick-start` | [PDF](examples/quick-start/output.pdf) |

## License

MIT OR Apache-2.0
