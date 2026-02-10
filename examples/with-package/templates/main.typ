#import "@preview/cetz:0.4.2": canvas, draw
#import "@preview/lilaq:0.5.0" as lq

#set text(font: "Source Serif 4")
#show heading.where(level: 1): set text(size: 1.5em)
#show math.equation: set text(font: "STIX Two Math")

= Package Bundling Example

Packages require no manual setup. Just use `#import` as you normally would in Typst, and `typst-bake` handles the rest automatically.

The `cetz` and `lilaq` packages were automatically detected from the import statements and downloaded at compile time. Their internal dependencies (such as `oxifmt`, `zero`, `tiptoe`, `elembic`) are also automatically resolved and embedded. No manual package installation is required.

Downloaded packages are cached in the system cache directory to speed up future compilations. To force a fresh download, run with `TYPST_BAKE_PKG_NOCACHE=1 cargo build`.

#grid(
  columns: (1fr, auto),
  column-gutter: 1em,
  align: (right + top, center + top),
  pad(right: -2.0em)[
    #v(1em)
    == CetZ Drawing

    A 3D shape rendered\
    using the CetZ package.
  ],
  [
    #canvas(length: 2.4cm, {
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

    #text(size: 0.8em)[Example by \@samuelireson]
  ],
)

#let pi = calc.pi
#let ts = lq.linspace(0, 24 * pi, num: 1000)
#let curve(t) = {
  let r = calc.exp(calc.cos(t)) - 2 * calc.cos(4 * t) - calc.pow(calc.sin(t / 12), 5)
  (calc.sin(t) * r, calc.cos(t) * r)
}
#let points = ts.map(t => curve(t))
#let xs = points.map(p => p.at(0))
#let ys = points.map(p => p.at(1))

#v(-8em)
#block[
  == Lilaq Plotting

  The butterfly curve rendered using the\
  Lilaq package:

  #set math.equation(numbering: none)
  #show math.equation.where(block: true): set align(left)
  $ x = sin(t) dot (e^(cos(t)) - 2 cos(4t) - sin^5(t slash 12)) $
  $ y = cos(t) dot (e^(cos(t)) - 2 cos(4t) - sin^5(t slash 12)) $
]

#block[
  #lq.diagram(
    width: 75%,
    height: 7.5cm,
    xaxis: (ticks: none),
    yaxis: (ticks: none),

    lq.plot(xs, ys, mark: none, smooth: true),
  )

  #align(center, block(width: 80%)[
    #text(size: 0.8em)[Butterfly Curve — Temple H. Fay (1989)]
  ])
]
