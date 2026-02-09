#!/bin/bash
set -euo pipefail

TAG=${1:?Usage: $0 <tag>}

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

gh release upload "$TAG" "$tmpdir"/*
echo "Uploaded $count assets to $TAG"
