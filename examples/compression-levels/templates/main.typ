#import "@preview/lilaq:0.5.0" as lq
#import "@preview/codly:1.3.0": *
#import "@preview/codly-languages:0.1.1": *

#set text(font: "Source Serif 4")
#show math.equation: set text(font: "STIX Two Math")
#show heading.where(level: 1): set text(size: 1.5em)
#show raw: set text(font: "JetBrains Mono")
#show: codly-init.with()
#codly(languages: codly-languages)

= Compression Levels

`typst-bake` compresses all embedded resources using *zstd* to minimize binary size.

== Compression Caching
#include("content/caching.typ")

== Custom Compression Level
#include("content/configuration.typ")

== Running Benchmarks

The next page shows benchmark results. To run them and generate this PDF on your own machine:

```bash
cargo test -p example-compression-levels --release --test benchmark -- --ignored --nocapture --test-threads=1
cargo run -p example-compression-levels
```

#pagebreak()

== Benchmark: zstd Compression Levels
#include("content/benchmarks.typ")
