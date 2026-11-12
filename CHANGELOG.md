# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Deprecated

- `EmbedStats::compression_ratio`, `EmbedStats::overall_ratio`, and the `HasCompressionRatio` trait, to be removed in 0.4. They return the fraction of size saved, which the name "ratio" does not convey; compute the value you need from the `original_size` and `compressed_size` fields or `total_original()`, `total_compressed()`, and `total_deduplicated()`.

## [0.3.0] - 2026-11-12

### Added

- WebAssembly support: typst-bake builds for WebAssembly without a C toolchain and runs in browsers and other JavaScript runtimes. Embedded resources are decompressed there with the pure-Rust `ruzstd` decoder, several times slower than the C `zstd` library that native targets keep (see the `compression-levels` example). CI builds for `wasm32-unknown-unknown`.
- `js-clock` feature (default): on `wasm32-unknown-unknown`, `datetime.today()` and `PdfTimestamp::now` read the current time from JavaScript `Date`.
- `Document::with_now` fixes the time that `datetime.today()` uses. With a fixed time, `datetime.today()` without an offset returns the UTC date.
- `PdfTimestamp::now` (the current time with the local UTC offset, as the Typst CLI writes it), `PdfTimestamp::from_unix`, and `PdfTimestamp::with_offset_minutes`.
- `IntoValue` and `IntoDict` can be derived for generic structs.

### Changed

- **Breaking:** `PdfTimestamp` is reduced to `now`, `from_unix`, and `with_offset_minutes`, none of which can fail; a timestamp that a PDF date cannot represent is reported by `to_pdf`. Replace `now_utc()` with `now()` (the same instant, written with the local offset), `now_local(m)` with `now().with_offset_minutes(m)`, and `utc(..)`/`local(..)` with `from_unix(..)` (seconds in UTC) plus `with_offset_minutes(..)`: `local(2026, 1, 1, 9, 0, 0, 540)` is `from_unix(1_767_225_600).with_offset_minutes(540)`. A fixed date can also be set in the template with `#set document(date: ..)`.
- **Breaking:** On `wasm32-unknown-unknown`, the current time comes from the `js-clock` feature, which is on by default. Before, `datetime.today()` always read it from JavaScript. Builds with `default-features = false` must add `js-clock` to keep the clock. Without it, typst-bake imports no JavaScript, `datetime.today()` fails unless `Document::with_now` is set, and `PdfTimestamp::now` is not available.
- **Breaking:** `Error`, `Severity`, and `SourceLocation` are `#[non_exhaustive]`, so later releases can extend them.
- **Breaking:** `Error::PdfGeneration` holds `Vec<Diagnostic>` with source locations, like `Error::Compilation`. It held a string of Rust debug output.
- **Breaking:** `Error::Decompression` no longer converts from `std::io::Error`.
- typst-bake no longer depends on `typst-as-lib` and implements Typst's `World` itself.
- Without `Document::with_now`, `datetime.today()` without an offset returns the local date, like the Typst CLI. It was the UTC date.
- Font metadata is computed at build time, and a font is decompressed only when a document first uses it. Previously every embedded font was decompressed and parsed each time a `Document` compiled, which cost tens of milliseconds with large (e.g. CJK) fonts. Decoded fonts are kept for the life of the process and shared by every `Document` from the same `document!` call.
- `typst-bake-macros` now depends on `typst-library`, which is compiled once more for the build host, so clean builds (including `cargo check` and rust-analyzer) take somewhat longer.
- Resources are compressed in a single call with their size known up front, which is faster and uses far less memory for small files (126 package files at level 22: 524 ms and 742 MB peak before, 107 ms and 10 MB now). Each compressed frame records that size, so decompression allocates the output once. Existing compression caches are refreshed on the next build.
- Hidden files in `template-dir` are no longer scanned for package imports; they were not embedded anyway.
- A package found only by scanning another package's sources is skipped with a warning when it cannot be resolved, instead of failing the build: such imports can sit in code the package never runs, like its tests.
- Update dependencies: `zstd` 0.14, `zip` 9, `dirs` 7.

### Fixed

- Package imports inside code blocks, functions, and conditionals, and package `include`s, are found and bundled. Only top-level imports were, so packages imported elsewhere (lilaq imports `komet` inside functions) failed at runtime.
- A panic while rendering no longer leaves the `Document` unusable. Several threads can also render the same `Document` in parallel; before, they took turns.
- `to_png` returns the new `Error::InvalidDpi` for a DPI that is not a positive finite number or makes a page too large to render. It panicked or produced a 1×1 image.
- `add_file` and `has_file` normalize paths like Typst does, so `./img//a.png` names `img/a.png`. Paths with `.` or doubled slashes were stored but never found, and `img/x/../a.png` was rejected.
- A runtime file at the entry's path replaces the entry, as documented for other files.
- An entry file that starts with a byte order mark no longer gains an empty first page.
- `document!` fails at build time when the entry is a directory or a hidden file, differs in letter case from the file on disk, or lies outside `template-dir`. Such entries built and then always failed at runtime. Diagnostics name the entry by its normalized path (`./main.typ` is `main.typ`).
- `IntoValue`/`IntoDict` use `type` as the key of a field named `r#type`. The key was `r#type`.
- Offsets passed to `datetime.today()` that are not whole hours are applied exactly. They were truncated to hours.
- `std.sys.inputs` holds the inputs, like `sys.inputs`. It was an empty dictionary.
- A PDF timestamp with a year outside 0000–9999, or an offset between −1 and −59 minutes, is now an error. Before, a negative year silently dropped the date and such an offset was written as positive.
- Fonts in sub-directories of `fonts-dir` are now available to documents. They were embedded into the binary but never registered. Because they now take part in font selection, output can change for projects that kept fonts in sub-directories.

## [0.2.0] - 2026-09-01

### Added

- `Document::warnings()` exposes the warnings Typst produces for a document that compiles successfully. Previously these were collected and silently discarded, so there was no way to learn about problems in a template.
- `Severity` and `Hint` types. A `Hint` carries an optional `location`, so hints that point at a secondary piece of code now report where that code is.
- `PdfConfig::creator` sets the PDF `/Creator` metadata. Defaults to `None`, which keeps typst's own `Typst x.y.z` string.

### Changed

- **Breaking:** `Diagnostic` gains a `severity` field and `Diagnostic::hints` is now `Vec<Hint>` instead of `Vec<String>`. Formatting a diagnostic or a hint is unaffected (`Hint` implements `Display`); code that reads hint strings directly needs `hint.message` instead of `hint`.
- **Breaking:** `Diagnostic` and `Hint` are `#[non_exhaustive]`, so future fields can be added without another breaking release.
- **Breaking:** `PdfConfig` gains a `creator` field. Constructing it with `..Default::default()` (as the documentation recommends) is unaffected.
- **Breaking:** The minimum supported Rust version is now 1.92, and the crates use edition 2024. Both are required by typst 0.15.
- Update `typst` to 0.15 and `typst-as-lib` to 0.16. **Templates are now compiled by typst 0.15**, so language and rendering changes in that release can affect your output. The `path` element was removed in favour of `curve`, and typst 0.15 emits more compact SVG.
- Update dependencies: `syn` 3, `ureq` 3.4

## [0.1.11] - 2026-06-24

### Added

- Compilation errors now expose the source location (file, line, column), hints, and the call/import trace of each diagnostic. Motivated by #13 (thanks @HectorCF16).

### Changed

- **Breaking:** `Error::Compilation` now wraps a `Vec<Diagnostic>` instead of a `String`. The new public `Diagnostic` and `SourceLocation` types carry the resolved location; the `Display` output gains `file:line:column: error: ...` formatting.

## [0.1.10] - 2026-06-07

### Added

- PDF export options via `Document::with_pdf_config()` and the `PdfConfig`, `PdfStandard`, and `PdfTimestamp` types: control PDF tagging, PDF/A or PDF/UA conformance, the document identifier, and the creation timestamp. Motivated by #11 (thanks @awhow).

### Changed

- Update dependencies: `toml` 1.1, `ureq` 3.3
- Improved documentation and examples: font setup guide shows the recommended font configuration, and the `output-formats` example demonstrates `with_pdf_config()`

## [0.1.9] - 2026-03-16

### Added

- Runtime file injection API: `add_file()` and `has_file()` methods for adding files at runtime
- Page selection API: `select_pages()` and `page_count()` methods for selective page output
- `with-runtime-files` example
- `test-page-selection` example

### Fixed

- Recursive font file detection in fonts directory (was only checking top level)
- Panic with error message on file read failure instead of silently skipping
- `TYPST_BAKE_PKG_NOCACHE` now actually triggers re-download (was silently skipped)
- Guard against underflow in page selection validation for zero-page documents
- Remove incorrect `template-dir` hint from fonts-dir error message

## [0.1.8] - 2026-03-13

### Fixed

- Support nested entry paths in `document!` macro (e.g., `document!("dir/main.typ")`)
- File paths in `.typ` files now resolve relative to the referring `.typ` file, enabling correct `#include`, `#import`, `image()`, `read()`, etc. across nested directories

### Added

- `test-nested-entry` example to verify relative path resolution from various directory depths

## [0.1.7] - 2026-03-03

### Fixed

- Fix Windows path separator mismatch in embedded file resolver causing "file not found" errors for package imports

### Changed

- Update `zip` dependency from 7 to 8

## [0.1.6] - 2025-02-14

### Added

- Local data directory lookup: locally installed packages (e.g. `@local`) are found automatically
- `TYPST_PACKAGE_PATH` and `TYPST_PACKAGE_CACHE_PATH` environment variable support (same as Typst)

### Changed

- Align package directories with Typst: use `{cache}/typst/packages/` and `{data}/typst/packages/` instead of `{cache}/typst-bake/packages/`
- Package resolution order: local data dir → cache dir → download from Typst Universe
- Improve error messages for failed package resolution
- Update dependencies: `zip` 7, `toml` 1.0

## [0.1.5] - 2025-02-10

### Added

- `compression_level` field to `EmbedStats`

### Fixed

- Fix package download race condition using file locks (`fd-lock`) and atomic extraction
  - Concurrent builds sharing a package cache could corrupt partially extracted directories
  - Now acquires per-package file locks and extracts to a temp directory before atomic rename

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

[0.3.0]: https://github.com/elgar328/typst-bake/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/elgar328/typst-bake/compare/v0.1.11...v0.2.0
[0.1.11]: https://github.com/elgar328/typst-bake/compare/v0.1.10...v0.1.11
[0.1.10]: https://github.com/elgar328/typst-bake/compare/v0.1.9...v0.1.10
[0.1.9]: https://github.com/elgar328/typst-bake/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/elgar328/typst-bake/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/elgar328/typst-bake/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/elgar328/typst-bake/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/elgar328/typst-bake/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/elgar328/typst-bake/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/elgar328/typst-bake/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/elgar328/typst-bake/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/elgar328/typst-bake/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/elgar328/typst-bake/releases/tag/v0.1.0


