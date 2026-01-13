#set text(font: "Inter 18pt")
#show heading: set text(font: "Inter 28pt", weight: "bold")
#show math.equation: set text(font: "STIX Two Math")
#show raw: set text(font: "JetBrains Mono")

= Hello from typst-bake!

This document was generated using #emph[typst-bake], a compile-time bundling solution for Typst.

== Math Example

Gaussian integral:
$ integral_(-infinity)^infinity e^(-x^2) dif x = sqrt(pi) $

Euler's identity:
$ e^(i pi) + 1 = 0 $

== Code Example

```rust
fn main() {
    let message = "Hello, typst-bake!";
    println!("{}", message);
}
```

Inline code: `let x = 42;`
