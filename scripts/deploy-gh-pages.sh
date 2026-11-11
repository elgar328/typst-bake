#!/bin/bash
set -euo pipefail

usage() {
  echo "Usage: $0 [--diagram-svg PATH] [--diagram-pdf PATH]" >&2
  echo "Pass a path only for a newly rendered architecture diagram;" >&2
  echo "otherwise the one currently on gh-pages is kept." >&2
  exit 1
}

diagram_svg=""
diagram_pdf=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --diagram-svg) [[ $# -ge 2 ]] || usage; diagram_svg="$2"; shift 2 ;;
    --diagram-pdf) [[ $# -ge 2 ]] || usage; diagram_pdf="$2"; shift 2 ;;
    *) usage ;;
  esac
done

check_diagram() {
  local src="$1" ext="$2"
  [[ -z "$src" ]] && return
  if [[ ! -f "$src" ]]; then
    echo "Missing: $src"
    exit 1
  fi
  if [[ "$src" != *."$ext" ]]; then
    echo "Not a .$ext file: $src"
    exit 1
  fi
}
check_diagram "$diagram_svg" svg
check_diagram "$diagram_pdf" pdf

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
output-formats_3.svg=examples/output-formats/output_3.svg
output-formats_1.png=examples/output-formats/output_1.png
output-formats_2.png=examples/output-formats/output_2.png
output-formats_3.png=examples/output-formats/output_3.png
with-runtime-files.pdf=examples/with-runtime-files/output.pdf
"

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

origin_url="$(git remote get-url origin)"

git -C "$tmpdir" init -b gh-pages
git -C "$tmpdir" remote add origin "$origin_url"

# The architecture diagram is rendered by hand from docs/architecture.mmd. Unless a new
# rendering is given, carry over the one from the current gh-pages before it is replaced.
if [[ -z "$diagram_svg" || -z "$diagram_pdf" ]]; then
  git -C "$tmpdir" fetch --depth 1 origin gh-pages
fi

place_diagram() {
  local name="$1" src="$2"
  if [[ -n "$src" ]]; then
    cp "$src" "$tmpdir/$name"
  elif ! git -C "$tmpdir" show "FETCH_HEAD:$name" > "$tmpdir/$name"; then
    echo "Missing: $name on gh-pages (pass it with --diagram-${name##*.})"
    exit 1
  fi
}
place_diagram architecture.svg "$diagram_svg"
place_diagram architecture.pdf "$diagram_pdf"

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

echo "Deployed $((count + 2)) files to gh-pages"
