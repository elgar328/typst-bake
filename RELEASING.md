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

### 8. Bump version and restore `-dev` suffix

Update the same three locations in `Cargo.toml` to the next development version (e.g., `X.Y.(Z+1)-dev`):

- [ ] `workspace.package.version`
- [ ] `workspace.dependencies.typst-bake.version`
- [ ] `workspace.dependencies.typst-bake-macros.version`

### 9. Commit and push development version

```sh
git add -A
git commit -m "chore: bump version to X.Y.Z-dev"
git push
```
