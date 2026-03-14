#import "@preview/codly:1.3.0": *
#import "@preview/codly-languages:0.1.1": *

#set text(font: "Source Serif 4")
#show raw: set text(font: "JetBrains Mono")
#show heading.where(level: 1): set text(size: 1.5em)
#show heading.where(level: 2): set block(above: 1.5em)

#show: codly-init.with()
#codly(languages: codly-languages)

= Runtime File Injection Example

Files that don't exist at compile time can be injected at runtime using the `add_file()` method. This is useful for dynamically generated content, downloaded resources, or any data that is only available when the program runs.

== How It Works

The `Document` struct provides two methods for runtime file injection:

- *`add_file(path, data)`* — Inject a file at the given path. The file becomes available to the Typst template as if it were embedded at compile time.
- *`has_file(path)`* — Check whether a file exists (either embedded or injected at runtime). Useful for conditional rendering.

The following example downloads a PDF from a remote server and injects it into the document:

```rust
// Download a PDF at runtime
let url = "https://example.com/report.pdf";
let pdf_bytes = ureq::get(url).call()?.body_mut().read_to_vec()?;

// Inject it so the template can reference "downloaded.pdf"
let pdf = typst_bake::document!("main.typ")
    .add_file("downloaded.pdf", pdf_bytes)?
    .to_pdf()?;
```

#v(0.5em)

#grid(
  columns: (1fr, 53%),
  column-gutter: 1.5em,
  [
    == Result

    The PDF shown on the right was downloaded at runtime and injected into this document using `add_file()`.
  ],
  box(
    stroke: 0.5pt + luma(140),
    clip: true,
    image("downloaded.pdf", width: 100%),
  ),
)
