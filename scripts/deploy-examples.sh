#!/bin/bash
set -euo pipefail

ASSETS="
quick-start.pdf=examples/quick-start/output.pdf
font-guide.pdf=examples/font-guide/output.pdf
with-inputs.pdf=examples/with-inputs/output.pdf
with-files.pdf=examples/with-files/output.pdf
with-package.pdf=examples/with-package/output.pdf
compression-levels.pdf=examples/compression-levels/output.pdf
output-formats.pdf=examples/output-formats/output.pdf
output-formats_1.svg=examples/output-formats/output_1.svg
output-formats_2.svg=examples/output-formats/output_2.svg
output-formats_1.png=examples/output-formats/output_1.png
output-formats_2.png=examples/output-formats/output_2.png
"

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

origin_url="$(git remote get-url origin)"

# Clone existing gh-pages branch (preserves manually deployed files),
# or create a new one if it doesn't exist yet.
if ! git clone --branch gh-pages --single-branch --depth 1 "$origin_url" "$tmpdir" 2>/dev/null; then
  rm -rf "$tmpdir"
  tmpdir=$(mktemp -d)
  git -C "$tmpdir" init -b gh-pages
  git -C "$tmpdir" remote add origin "$origin_url"
fi

count=0
for entry in $ASSETS; do
  name="${entry%%=*}"
  src="${entry#*=}"
  if [[ ! -f "$src" ]]; then
    echo "Missing: $src (run 'cargo test --workspace' first)"
    exit 1
  fi
  cp "$src" "$tmpdir/$name"
  count=$((count + 1))
done

touch "$tmpdir/.nojekyll"

git -C "$tmpdir" add .
git -C "$tmpdir" commit -m "deploy example outputs" --allow-empty
git -C "$tmpdir" push -f origin gh-pages

echo "Deployed $count files to gh-pages"
