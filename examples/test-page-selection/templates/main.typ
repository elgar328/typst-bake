#set page(margin: 1.5cm)
#set text(font: "Source Serif 4", size: 10pt)
#show heading.where(level: 1): set text(size: 1.5em)
#show heading.where(level: 2): set text(size: 1.1em)
#show heading.where(level: 2): set block(above: 1.2em, below: 0.6em)
#show raw: set text(font: "JetBrains Mono", size: 8pt)

= Page Selection Test

== 1. All Pages (PDF)

```rust
let all_pdf = doc.to_pdf()?;
```

#grid(
  columns: 5,
  column-gutter: 4pt,
  ..range(1, 6).map(i =>
    box(stroke: 0.5pt + luma(140), inset: 4pt, image("outputs/all.pdf", height: 60pt, page: i))
  )
)

== 2. Single Page — Page 1 (PDF)

```rust
let page1_pdf = doc.select_pages([0]).to_pdf()?;
```

#box(stroke: 0.5pt + luma(140), inset: 4pt, image("outputs/page1.pdf", height: 60pt))

#block(
  fill: luma(245),
  inset: 10pt,
  radius: 3pt,
  width: 100%,
)[
  *Note: PDF accessibility tags are disabled when using page selection.*

  Since Typst 0.14, PDFs are _tagged_ by default. A tagged PDF embeds structural
  metadata — headings, paragraphs, tables, and reading order — that allows screen
  readers to make the document accessible to visually impaired users.

  In `typst-pdf` 0.14, the tag tree is constructed over the entire document.
  When a page range is applied, not all nodes in the tree are visited, causing
  the internal traversal check to fail. Because of this limitation,
  `select_pages().to_pdf()` automatically disables tagging.
  Full-document `to_pdf()` continues to produce tagged PDFs as normal.
]

== 3. Single Page — Page 2 (PNG)

```rust
let page2_png = doc.select_pages([1]).to_png(144.0)?;
```

#box(stroke: 0.5pt + luma(140), inset: 4pt, image("outputs/page2.png", height: 60pt))

== 4. Single Page — Page 3 (SVG)

```rust
let page3_svg = doc.select_pages([2]).to_svg()?;
```

#box(stroke: 0.5pt + luma(140), inset: 4pt, image("outputs/page3.svg", height: 60pt))

== 5. Odd Pages — 1, 3, 5 (PDF)

```rust
let odd_pdf = doc.select_pages([0, 2, 4]).to_pdf()?;
```

#grid(
  columns: 3,
  column-gutter: 4pt,
  ..range(1, 4).map(i =>
    box(stroke: 0.5pt + luma(140), inset: 4pt, image("outputs/odd.pdf", height: 60pt, page: i))
  )
)

== 6. Odd Pages — 1, 3, 5 (PNG)

```rust
let odd_pngs = doc.select_pages([0, 2, 4]).to_png(144.0)?;
```

#grid(
  columns: 3,
  column-gutter: 4pt,
  ..("outputs/odd_1.png", "outputs/odd_2.png", "outputs/odd_3.png").map(p =>
    box(stroke: 0.5pt + luma(140), inset: 4pt, image(p, height: 60pt))
  )
)

== 7. Dedup & Sort — \[4, 0, 0, 2\] (PDF)

```rust
let dedup_pdf = doc.select_pages([4, 0, 0, 2]).to_pdf()?;
```

Duplicates are removed and indices are sorted automatically (`BTreeSet`),
so `[4, 0, 0, 2]` produces the same result as `[0, 2, 4]` — pages 1, 3, 5.

#grid(
  columns: 3,
  column-gutter: 4pt,
  ..range(1, 4).map(i =>
    box(stroke: 0.5pt + luma(140), inset: 4pt, image("outputs/dedup.pdf", height: 60pt, page: i))
  )
)

== 8. Range — Pages 2–4 (PDF)

```rust
let range_pdf = doc.select_pages(1..4).to_pdf()?;
```

#grid(
  columns: 3,
  column-gutter: 4pt,
  ..range(1, 4).map(i =>
    box(stroke: 0.5pt + luma(140), inset: 4pt, image("outputs/range.pdf", height: 60pt, page: i))
  )
)

== 9. Error Cases

```rust
// Empty selection → Err
assert!(doc.select_pages([]).to_pdf().is_err());

// Out of range → Err
assert!(doc.select_pages([99]).to_pdf().is_err());
```

Both cases correctly return errors without panicking.
