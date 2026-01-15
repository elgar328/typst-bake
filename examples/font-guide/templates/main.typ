#set text(font: "Inter 18pt")
#show heading: set text(font: "Source Serif 4", weight: "bold")
#show heading.where(level: 2): set block(above: 2em)
#show math.equation: set text(font: "STIX Two Math")
#show raw: set text(font: "JetBrains Mono")

= Font Guide

`typst-bake` requires at least one font in `fonts-dir`. Without fonts, Typst produces invisible text, so a compile-time error is triggered if no fonts are found.

While all files in `template-dir` are embedded, only supported font formats (TTF, OTF, TTC) are embedded from `fonts-dir`. Other files are ignored.

This document uses `Source Serif 4` for headings and `Inter` for body text.

== Math Example

For equations or graphs, a math font is required. Without one, rendering will fail. This example uses `STIX Two Math`.

Navier-Stokes equations:
$ nabla dot bold(u) = 0 $
$ rho ((partial bold(u)) / (partial t) + (bold(u) dot nabla) bold(u)) = -nabla p + mu nabla^2 bold(u) + bold(f) $

== Code Example

For code blocks, using a monospace font is recommended. Without one, code may render with improper spacing. This example uses `JetBrains Mono`.

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf = typst_bake::document!("main.typ").to_pdf()?;
    std::fs::write("output.pdf", &pdf)?;
    Ok(())
}
```
