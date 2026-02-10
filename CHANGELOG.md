# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.5] - 2025-02-10

### Added

- `compression_level` field to `EmbedStats`

### Fixed

- Fix package download race condition using file locks (`fd-lock`) and atomic extraction

### Changed

- **BREAKING:** Unify environment variable prefix to `TYPST_BAKE_` and rename `REFRESH` to `PKG_NOCACHE`
  - `TYPST_TEMPLATE_DIR` → `TYPST_BAKE_TEMPLATE_DIR`
  - `TYPST_FONTS_DIR` → `TYPST_BAKE_FONTS_DIR`
  - `TYPST_BAKE_REFRESH` → `TYPST_BAKE_PKG_NOCACHE`
- Remove `chrono` dependency (only used in example)
- Example outputs (PDF/PNG/SVG) are now hosted on GitHub Pages instead of tracked in the repository
- Consolidate example fonts into a shared `examples/fonts/` directory
- **Note:** Git history has been rewritten to remove example output files (PDF/PNG/SVG) and duplicate font files. If you have a local clone, please re-clone the repository.

## [0.1.4] - 2025-02-08

### Added

- `compression-levels` example with benchmark
- `RELEASING.md` release checklist

### Fixed

- Embed only resolved packages instead of entire cache directory

### Changed

- Improved compression: dedup identical file contents, cache compressed blobs
- Added dedup info to `EmbedStats` and improved `display()` format
- Extensive internal refactoring (let-else, iterator patterns, helper extraction, type safety)
- Improved documentation and doc comment quality

## [0.1.3] - 2025-01-19

### Fixed

- Fixed docs.rs build failure by enabling `doc_cfg` feature

## [0.1.2] - 2025-01-19

### Added

- **Multiple output formats** with feature flags:
  - `pdf` (default) - PDF generation via `to_pdf()`
  - `svg` - SVG generation via `to_svg()`
  - `png` - PNG rasterization via `to_png(dpi)`
  - `full` - Enable all formats
- Custom error types (`Error`, `Result`) for better error handling
- `output-formats` example demonstrating all rendering options
- Unit tests for stats module and integration tests for examples

### Changed

- Improved compilation error messages to show clean diagnostic text
- Improved API documentation with feature flag annotations
- Added comparison with typst-as-lib to README
- Internal code cleanup (shared util module, unified scan functions)

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

[0.1.5]: https://github.com/elgar328/typst-bake/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/elgar328/typst-bake/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/elgar328/typst-bake/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/elgar328/typst-bake/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/elgar328/typst-bake/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/elgar328/typst-bake/releases/tag/v0.1.0


