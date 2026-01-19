#set page(margin: 2cm)
#set text(font: "Source Serif 4", size: 11pt)
#show heading.where(level: 1): set text(size: 20pt, weight: "bold")
#show heading.where(level: 2): set text(size: 14pt, weight: "bold")
#show heading.where(level: 2): set block(above: 1.5em, below: 0.8em)
#show raw: set text(size: 9pt)

= Output Formats

This example demonstrates multi-format output: PDF, SVG, and PNG.

== Feature Configuration

In your `Cargo.toml`, configure output formats using features:

```toml
[dependencies]
# Default: PDF only
typst-bake = "0.1"

# Add SVG support
typst-bake = { version = "0.1", features = ["svg"] }

# Add PNG support
typst-bake = { version = "0.1", features = ["png"] }

# All formats (pdf + svg + png)
typst-bake = { version = "0.1", features = ["full"] }

# SVG only (disable default PDF)
typst-bake = { version = "0.1", default-features = false, features = ["svg"] }
```

== Output Behavior

*PDF* outputs all pages as a single file:
- `output.pdf` (contains all pages)

*SVG* and *PNG* return a Vec with one item per page:
- Save each page individually (e.g., `output_1.svg`, `output_1.png`, ...)

== PNG Resolution

PNG requires a DPI (dots per inch) setting:

```rust
// 72 DPI (standard)
let pngs = doc.to_png(72.0)?;

// 144 DPI (2x)
let pngs = doc.to_png(144.0)?;

// 300 DPI (print quality)
let pngs = doc.to_png(300.0)?;
```

== Note

Page 2 includes test patterns to verify multi-page behavior and evaluate PDF/SVG/PNG rendering differences.

#pagebreak()

= Rendering Test Patterns

// === Constants ===
#let bar-height = 16pt
#let stroke-thin = 0.25pt
#let n-star = 72
#let n-plate = 64

// === Color bar function ===
#let color-bar(colors, height: 16pt) = box(height: height, width: 100%)[
  #grid(
    columns: (1fr,) * colors.len(),
    ..colors.map(c => box(fill: c, width: 100%, height: height))
  )
]

#v(8pt)

// === Color bars (SMPTE order by luminance, 100% saturation) ===
#let colors = (
  rgb("#ffff00"), rgb("#00ffff"), rgb("#00ff00"),
  rgb("#ff00ff"), rgb("#ff0000"), rgb("#0000ff"),
)
#color-bar(colors, height: bar-height)
#let grays = range(0, 11).map(i => luma(i * 25))
#color-bar(grays, height: bar-height)
#box(height: bar-height, width: 100%)[
  #rect(width: 100%, height: bar-height, fill: gradient.linear(black, white))
]

#v(12pt)

// === Stroke width test (curve area is square, excluding label) ===
#let stroke-test(size: 160pt, label-w: 20pt) = {
  let widths = (0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0)
  let step = size / (widths.len() + 1)
  let total-width = label-w + size

  box(width: total-width, height: size)[
    #for (i, w) in widths.enumerate() {
      let y-top = i * step
      let x-right = label-w + size - i * step
      let y-bottom = size
      let curve-height = y-bottom - y-top

      place(dx: 2pt, dy: y-top - 4pt, text(size: 7pt)[#str(w)])

      place(path(
        stroke: w * 1pt,
        fill: none,
        closed: false,
        ((label-w, y-top), (0pt, 0pt), (x-right - label-w, 0pt)),
        ((x-right, y-bottom), (0pt, -curve-height / 2), (0pt, 0pt)),
      ))
    }
  ]
}

// === Siemens Star (parametric) ===
#let siemens-star(size: 80pt, n: n-star) = {
  let center = size / 2
  let radius = size / 2 - 2pt
  box(width: size, height: size)[
    #for i in range(n) {
      let angle = i * 360deg / n
      let x = center + radius * calc.cos(angle)
      let y = center + radius * calc.sin(angle)
      place(line(start: (center, center), end: (x, y), stroke: stroke-thin))
    }
  ]
}

// === Zone Plate (parametric) ===
#let zone-plate(size: 80pt, n: n-plate) = {
  let center = size / 2
  let radius = size / 2 - 2pt
  box(width: size, height: size)[
    #for i in range(1, n + 1) {
      let r = radius * calc.sqrt(i / n)
      place(
        dx: center - r,
        dy: center - r,
        circle(radius: r, fill: none, stroke: stroke-thin)
      )
    }
  ]
}

// === 2-column layout: curves (left) + star/plate (right) ===
#let curve-size = 300pt
#let label-w = 20pt
#let pattern-gap = 8pt
#let pattern-size = (curve-size - pattern-gap) / 2

#grid(
  columns: (label-w + curve-size, 1fr),
  column-gutter: 16pt,
  align: top,
  // Left: stroke test (curve area is square)
  stroke-test(size: curve-size, label-w: label-w),
  // Right: Siemens Star + Zone Plate (vertical stack, exact height)
  align(center + top)[
    #stack(
      dir: ttb,
      spacing: pattern-gap,
      siemens-star(size: pattern-size),
      zone-plate(size: pattern-size),
    )
  ],
)

#v(12pt)

// === Typography samples ===
#let sizes = (2pt, 3pt, 4pt, 5pt, 6pt, 7pt, 8pt, 9pt, 10pt, 12pt, 14pt, 18pt, 24pt, 36pt)
#for (i, s) in sizes.enumerate() {
  if i > 0 { v(-10pt) }
  box(width: 100%, height: s * 1.2, clip: true)[
    #place(dy: s * 0.15)[#text(size: s)[#str(s.pt())pt: The quick brown fox jumps over the lazy dog]]
  ]
}
