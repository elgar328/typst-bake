# Releasing

## Checklist

### 1. Remove `-dev` suffix from versions

Update all three locations in `Cargo.toml`:

- [ ] `workspace.package.version`
- [ ] `workspace.dependencies.typst-bake.version`
- [ ] `workspace.dependencies.typst-bake-macros.version`

### 2. Run checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

### 3. Update CHANGELOG.md

- Add a new section for the release version in [Keep a Changelog](https://keepachangelog.com/) format.
- Add a comparison link at the bottom (e.g., `[X.Y.Z]: https://github.com/.../compare/vA.B.C...vX.Y.Z`).

### 4. Create release commit and tag

```sh
git add -A
git commit -m "release: vX.Y.Z"
git tag vX.Y.Z
```

### 5. Push commit and tag

```sh
git push
git push --tags
```

### 6. Publish to crates.io

Publish `typst-bake-macros` first (dependency of `typst-bake`):

```sh
cargo publish -p typst-bake-macros --dry-run
cargo publish -p typst-bake-macros
cargo publish -p typst-bake --dry-run
cargo publish -p typst-bake
```

### 7. Create GitHub Release

```sh
gh release create vX.Y.Z --title "vX.Y.Z" --notes-file <(sed -n '/^## \[X\.Y\.Z\]/,/^## \[/{ /^## \[X\.Y\.Z\]/d; /^## \[/d; p; }' CHANGELOG.md)
```

### 8. Upload example outputs to release

```sh
gh release upload vX.Y.Z \
  examples/quick-start/output.pdf#quick-start.pdf \
  examples/font-guide/output.pdf#font-guide.pdf \
  examples/with-inputs/output.pdf#with-inputs.pdf \
  examples/with-files/output.pdf#with-files.pdf \
  examples/with-package/output.pdf#with-package.pdf \
  examples/compression-levels/output.pdf#compression-levels.pdf \
  examples/output-formats/output.pdf#output-formats.pdf \
  examples/output-formats/output_1.svg#output-formats_1.svg \
  examples/output-formats/output_2.svg#output-formats_2.svg \
  examples/output-formats/output_1.png#output-formats_1.png \
  examples/output-formats/output_2.png#output-formats_2.png
```

### 9. Bump version and restore `-dev` suffix

Update the same three locations in `Cargo.toml` to the next development version (e.g., `X.Y.(Z+1)-dev`):

- [ ] `workspace.package.version`
- [ ] `workspace.dependencies.typst-bake.version`
- [ ] `workspace.dependencies.typst-bake-macros.version`

### 10. Commit and push development version

```sh
git add -A
git commit -m "chore: bump version to X.Y.Z-dev"
git push
```
