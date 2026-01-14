#set text(font: "Inter 18pt")
#show heading: set text(font: "Source Serif 4", weight: "bold")
#show heading.where(level: 2): set block(above: 2em)
#show math.equation: set text(font: "STIX Two Math")
#show raw: set text(font: "JetBrains Mono")

= Hello from typst-bake!

This document was generated using #emph[typst-bake], a compile-time bundling solution for Typst.

== Math Example

Navier-Stokes equations:
$ nabla dot bold(u) = 0 $
$ rho ((partial bold(u)) / (partial t) + (bold(u) dot nabla) bold(u)) = -nabla p + mu nabla^2 bold(u) + bold(f) $

== Code Example

#v(0.3em)

```rust
fn is_prime(n: u32) -> bool {
    if n < 2 { return false; }
    for i in 2..=(n as f64).sqrt() as u32 {
        if n % i == 0 { return false; }
    }
    true
}

fn main() {
    let primes: Vec<_> = (2..30).filter(|&n| is_prime(n)).collect();
    println!("Primes: {:?}", primes);
}
```
