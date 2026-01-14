#set text(font: "Source Serif 4")
#show heading: set text(weight: "bold")
#show heading.where(level: 2): set block(above: 2em)

= File Embedding Example

This document demonstrates embedding various file types with typst-bake. All files in the template directory are bundled into the binary at compile time.

== Image File

#align(center)[
  #image("files/svg_file.svg", width: 40%)
]

== CSV File

#let data = csv("files/csv_file.csv")
#align(center)[
  #block(width: 80%)[
    #table(
      columns: (1.5fr, 1fr, 2fr),
      align: (center, center, center),
      table.header([*Language*], [*Year*], [*Paradigm*]),
      ..data.slice(1).flatten()
    )
  ]
]

== JSON File

#let books = json("files/json_file.json")
#align(center)[
  #block(width: 80%)[
    #table(
      columns: (2fr, 2fr, auto, 2fr),
      align: (left, left, center, left),
      table.header([*Title*], [*Author*], [*Year*], [*Genres*]),
      ..books.map(b => (b.title, b.author, str(b.year), b.genres.join(", "))).flatten()
    )
  ]
]

== TOML File

#let config = toml("files/toml_file.toml")
#align(center)[
  #block(width: 80%)[
    #table(
      columns: (1fr, 2fr),
      align: (left, left),
      [*title*], [#config.document.title],
      [*author*], [#config.document.author],
      [*date*], [#config.document.date],
      [*font-size*], [#config.settings.font-size],
      [*line-spacing*], [#config.settings.line-spacing],
    )
  ]
]

== Text File

#align(center)[
  #block(
    width: 80%,
    stroke: 1pt,
    inset: 1em,
    read("files/txt_file.txt")
  )
]
