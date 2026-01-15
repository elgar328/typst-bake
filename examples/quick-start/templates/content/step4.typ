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
      Using packages requires no manual setup. Just use `#import "@preview/..."` as you normally would in Typst, and `typst-bake` handles the rest automatically.
    ]
  ]
)
