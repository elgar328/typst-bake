#import sys: inputs

#set text(font: "Source Serif 4")
#show heading: set text(weight: "bold")

= Dynamic Input Example

This document demonstrates passing data from Rust to Typst at runtime. The invoice below is generated using values from a Rust struct, accessed via `sys.inputs`.

#v(2em)

#align(center)[
  #text(size: 2em, weight: "bold")[INVOICE]
]

#v(1em)

#line(length: 100%, stroke: 0.5pt)

#v(0.5em)

#grid(
  columns: (1fr, 1fr),
  align: (left, right),
  [
    *Bill To:*\
    #inputs.customer
  ],
  [
    *Invoice \#:* #inputs.number\
    *Date:* #inputs.date
  ],
)

#v(1em)

#table(
  columns: (2fr, auto, auto, auto),
  align: (left, center, right, right),
  stroke: none,
  table.hline(stroke: 1pt),
  table.header(
    [*Description*], [*Qty*], [*Price*], [*Amount*]
  ),
  table.hline(stroke: 0.5pt),
  ..inputs.items.map(item => (
    item.description,
    str(item.quantity),
    [\$#str(item.price)],
    [\$#str(item.amount)],
  )).flatten(),
  table.hline(stroke: 1pt),
  [], [], table.cell(align: right)[*Total*], [*\$#str(inputs.total)*],
)

#v(2em)

#line(length: 100%, stroke: 0.5pt)

#v(0.5em)

#align(center)[
  #text(size: 0.9em, fill: luma(50))[Thank you for your business!]
]
