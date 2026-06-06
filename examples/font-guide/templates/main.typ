#set text(font: "Source Serif 4")
#show heading.where(level: 1): set text(size: 1.5em)
#show heading.where(level: 2): set block(above: 1.5em)
#show math.equation: set text(font: "STIX Two Math")
#show raw: set text(font: "JetBrains Mono")

= Font Guide

`typst-bake` embeds fonts from `fonts-dir` and requires at least one. Without any font, Typst renders invisible text, so a compile-time error is raised when none are found. Only TTF, OTF, and TTC files are embedded; other files in `fonts-dir` are ignored.

Fonts are selected in your template with `set` and `show` rules. The lines below configure this document. These are just examples—use any fonts you place in your `fonts-dir`.

```typ
#set text(font: "Source Serif 4")
#show math.equation: set text(font: "STIX Two Math")
#show raw: set text(font: "JetBrains Mono")
```

== Body Text

Configured with `text(font: ...)`. This document uses `Source Serif 4`.

== Math

Equations and plots need a math font, or rendering fails. This document uses `STIX Two Math`.

Navier-Stokes equations:
$ nabla dot bold(u) = 0 $
$ rho ((partial bold(u)) / (partial t) + (bold(u) dot nabla) bold(u)) = -nabla p + mu nabla^2 bold(u) + bold(f) $

== Code

Code blocks read best in a monospace font. Without one, code may render with improper spacing. This document uses `JetBrains Mono`.

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf = typst_bake::document!("main.typ").to_pdf()?;
    std::fs::write("output.pdf", &pdf)?;
    Ok(())
}
```
