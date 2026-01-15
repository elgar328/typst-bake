#import "@preview/gentle-clues:1.2.0": info

Define structs with derive macros to pass data to templates:

#grid(
  columns: (3fr, 2fr),
  gutter: 1em,
  [
    ```rust
    use typst_bake::{IntoValue, IntoDict};

    #[derive(IntoValue, IntoDict)]
    struct Inputs {
        title: String,
        discount: f64,
        products: Vec<Product>,
    }

    #[derive(IntoValue)]
    struct Product {
        name: String,
        price: f64,
    }
    ```
  ],
  [
    #info[
      *Top-level struct:* \
      `IntoValue`, `IntoDict`

      *Nested structs:* \
      `IntoValue`
    ]
  ]
)

Create input data:

```rust
let inputs = Inputs {
    title: "Sale".to_string(),
    discount: 20.0,
    products: vec![
        Product { name: "Apple".to_string(), price: 2.0 },
        Product { name: "Banana".to_string(), price: 1.0 },
    ],
};
```
