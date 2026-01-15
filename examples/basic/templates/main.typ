#set text(font: "Inter 18pt")
#show heading: set text(font: "Source Serif 4", weight: "bold")
#show heading.where(level: 2): set block(above: 2em)
#show math.equation: set text(font: "STIX Two Math")
#show raw: set text(font: "JetBrains Mono")

= Hello from typst-bake!

This document was generated using #emph[typst-bake], a compile-time bundling solution for Typst. All resources including templates and fonts are embedded into the binary at compile time.

Note that `typst-bake` requires at least one font in `fonts-dir`. Without fonts, Typst produces invisible text, so a compile-time error is triggered if no fonts are found.

== Math Example

Navier-Stokes equations:
$ nabla dot bold(u) = 0 $
$ rho ((partial bold(u)) / (partial t) + (bold(u) dot nabla) bold(u)) = -nabla p + mu nabla^2 bold(u) + bold(f) $

== Code Example

#v(0.3em)

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf = typst_bake::document!("main.typ").to_pdf()?;
    std::fs::write("output.pdf", &pdf)?;
    Ok(())
}
```
