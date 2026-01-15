#import "@preview/gentle-clues:1.2.0": tip

Add typst-bake to your dependencies and configure the template and font directories:

#grid(
  columns: (3fr, 2fr),
  gutter: 1em,
  [
    ```toml
    [dependencies]
    typst-bake = "0.1"

    [package.metadata.typst-bake]
    template-dir = "./templates"
    fonts-dir = "./fonts"
    ```
  ],
  [
    #tip[You can also use `TYPST_TEMPLATE_DIR` and `TYPST_FONTS_DIR` environment variables. If both are set, environment variables take priority.]
  ]
)
