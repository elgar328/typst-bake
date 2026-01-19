#import "@preview/gentle-clues:1.2.0": tip

Generate PDF with `document!` macro:

#grid(
  columns: (3fr, 2fr),
  gutter: 1em,
  [
    ```rust
    let pdf = typst_bake::document!("main.typ")
        .with_inputs(inputs)
        .to_pdf()?;

    std::fs::write("output.pdf", &pdf)?;
    ```
  ],
  [
    #tip[
      When no inputs are needed, call `.to_pdf()` directly without `.with_inputs()`. \ For SVG/PNG output, see the `output-formats` example.
    ]
  ]
)
