# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2025-01-16

### Added

- `IntoDict` derive macro now also implements `From<T> for Dict`, allowing cleaner API usage:
  ```rust
  // Before
  .with_inputs(inputs.into_dict())

  // After
  .with_inputs(inputs)
  ```

### Changed

- Updated dependencies: toml 0.9, dirs 6, chrono 0.4.43
- Improved CI with caching, linting (clippy), and formatting checks
- Added Dependabot for automatic dependency updates

## [0.1.0] - 2025-01-15

Initial release of typst-bake - a library to bake Typst templates, fonts, and packages into your Rust binary for fully self-contained PDF generation.

### Features

- **Simple API** - Generate PDFs with `document!("main.typ").to_pdf()`
- **File & Font Embedding** - All templates and fonts are embedded at compile time
- **Automatic Package Bundling** - Packages are automatically detected, downloaded, and embedded
- **Package Caching** - Downloaded packages are cached for faster compilation
- **Runtime Inputs** - Pass data from Rust to Typst via `IntoValue` / `IntoDict` derive macros
- **Optimized Binary Size** - Resources compressed with zstd, decompressed lazily at runtime
- **Smart Recompilation** - File changes detected automatically by Cargo

### Links

- [Quick Start Guide (PDF)](https://github.com/elgar328/typst-bake/blob/main/examples/quick-start/output.pdf)


[0.1.1]: https://github.com/elgar328/typst-bake/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/elgar328/typst-bake/releases/tag/v0.1.0


