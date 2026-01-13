#import "@preview/cetz:0.4.2": canvas, draw

#set text(font: "Source Serif 4")
#show math.equation: set text(font: "STIX Two Math")

= Package Bundling Example

This document demonstrates automatic package bundling with typst-bake.

The `cetz` package was automatically detected from the import statement and downloaded at compile time. No manual package installation is required.

== CetZ Drawing

The following 3D geometric shape is rendered using the CetZ package:

#align(center)[
  #canvas(length: 2cm, {
    import draw: *
    let phi = (1 + calc.sqrt(5)) / 2

    ortho({
      hide({
        line(
          (-phi, -1, 0), (-phi, 1, 0), (phi, 1, 0), (phi, -1, 0), close: true, name: "xy",
        )
        line(
          (-1, 0, -phi), (1, 0, -phi), (1, 0, phi), (-1, 0, phi), close: true, name: "xz",
        )
        line(
          (0, -phi, -1), (0, -phi, 1), (0, phi, 1), (0, phi, -1), close: true, name: "yz",
        )
      })

      intersections("a", "yz", "xy")
      intersections("b", "xz", "yz")
      intersections("c", "xy", "xz")

      set-style(stroke: (thickness: 0.5pt, cap: "round", join: "round"))
      line((0, 0, 0), "c.1", (phi, 1, 0), (phi, -1, 0), "c.3")
      line("c.0", (-phi, 1, 0), "a.2")
      line((0, 0, 0), "b.1", (1, 0, phi), (-1, 0, phi), "b.3")
      line("b.0", (1, 0, -phi), "c.2")
      line((0, 0, 0), "a.1", (0, phi, 1), (0, phi, -1), "a.3")
      line("a.0", (0, -phi, 1), "b.2")

      anchor("A", (0, phi, 1))
      content("A", [$A$], anchor: "north", padding: .1)
      anchor("B", (-1, 0, phi))
      content("B", [$B$], anchor: "south", padding: .1)
      anchor("C", (1, 0, phi))
      content("C", [$C$], anchor: "south", padding: .1)
      line("A", "B", stroke: (dash: "dashed"))
      line("A", "C", stroke: (dash: "dashed"))
    })
  })
]

If you can see the shape above, the package bundling is working correctly.
