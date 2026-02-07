#import "@preview/lilaq:0.5.0" as lq
#import "benchmark-data.typ": *

#let default-level = 19

// Flatten all measurements into (level, speed) pairs for scatter
#let comp_scatter_x = levels.zip(comp_speed_all).map(((l, speeds)) => speeds.map(_ => l)).flatten()
#let comp_scatter_y = comp_speed_all.flatten()
#let decomp_scatter_x = levels.zip(decomp_speed_all).map(((l, speeds)) => speeds.map(_ => l)).flatten()
#let decomp_scatter_y = decomp_speed_all.flatten()

// Auto-calculate label y-positions from data range
#let ratio-label-y = calc.min(..ratio) + (calc.max(..ratio) - calc.min(..ratio)) * 0.15
#let comp-label-y = calc.max(..comp_scatter_y) - (calc.max(..comp_scatter_y) - calc.min(..comp_scatter_y)) * 0.10
#let decomp-label-y = calc.max(..decomp_scatter_y) - (calc.max(..decomp_scatter_y) - calc.min(..decomp_scatter_y)) * 0.10

=== Compression Ratio vs Level

#align(center)[
  #lq.diagram(
    width: 80%,
    height: 5cm,
    xlabel: [Compression Level],
    ylabel: [Compression Ratio],

    lq.plot(levels, ratio, mark: "o", mark-size: 3pt),
    lq.vlines(default-level, stroke: (dash: "dashed", paint: red, thickness: 0.8pt)),
    lq.place(default-level, ratio-label-y, align: left + top, pad(left: 3pt, text(size: 0.75em, fill: red)[Default])),
  )
]

=== Compression Speed vs Level

#align(center)[
  #lq.diagram(
    width: 80%,
    height: 5cm,
    xlabel: [Compression Level],
    ylabel: [Comp. Speed (MB/s)],
    yaxis: (exponent: none),

    lq.plot(comp_scatter_x, comp_scatter_y, mark: "o", mark-size: 2pt, stroke: none, color: blue.lighten(60%)),
    lq.plot(levels, comp_speed, mark: "o", mark-size: 3pt),
    lq.vlines(default-level, stroke: (dash: "dashed", paint: red, thickness: 0.8pt)),
    lq.place(default-level, comp-label-y, align: left + bottom, pad(left: 3pt, text(size: 0.75em, fill: red)[Default])),
  )
]

=== Decompression Speed vs Level

#align(center)[
  #lq.diagram(
    width: 80%,
    height: 5cm,
    xlabel: [Compression Level],
    ylabel: [Decomp. Speed (MB/s)],

    lq.plot(decomp_scatter_x, decomp_scatter_y, mark: "o", mark-size: 2pt, stroke: none, color: blue.lighten(60%)),
    lq.plot(levels, decomp_speed, mark: "o", mark-size: 3pt),
    lq.vlines(default-level, stroke: (dash: "dashed", paint: red, thickness: 0.8pt)),
    lq.place(default-level, decomp-label-y, align: left + bottom, pad(left: 3pt, text(size: 0.75em, fill: red)[Default])),
  )
]

#v(1em)
#line(length: 100%, stroke: 0.5pt + gray)
#text(size: 0.8em)[#footer-text]
