#import sys: inputs
#import "@preview/codly:1.3.0": *
#import "@preview/codly-languages:0.1.1": *

#set text(font: "Source Serif 4")
#show heading.where(level: 1): set text(size: 1.5em)
#show raw: set text(font: "JetBrains Mono")
#show: codly-init.with()
#codly(languages: codly-languages)

= typst-bake Quick Start

== Step 1: Configure Cargo.toml
#include("content/step1.typ")

== Step 2: Prepare Files
#include("content/step2.typ")

== Step 3: Define Inputs (Rust)
#include("content/step3.typ")

== Step 4: Write Typst Template
#include("content/step4.typ")

== Step 5: Generate PDF
#include("content/step5.typ")

#line(length: 100%, stroke: 0.5pt + gray)
For more examples, see the `examples/` directory in the repository.

_This document was generated with *#calc.round(inputs.fonts_size / 1024, digits: 1) KB* of fonts and *#inputs.files_size B* of templates._
