#import "@preview/gentle-clues:1.2.0": info

Access inputs via `sys.inputs` and use embedded files:

#grid(
  columns: (3fr, 2fr),
  gutter: 1em,
  [
    ```typ
    #import sys: inputs

    #image("images/logo.png", width: 50%)

    = #inputs.title (#inputs.discount% off)

    #for product in inputs.products [
      - #product.name: $#product.price
    ]
    ```
  ],
  [
    #info[
      Packages imported with `#import` in `.typ` files are automatically detected, downloaded, and embedded at build time.
    ]
  ]
)
