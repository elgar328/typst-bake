#import sys: inputs

#let format-size(bytes) = {
  if bytes >= 1024 * 1024 {
    str(calc.round(bytes / 1024 / 1024, digits: 2)) + " MB"
  } else if bytes >= 1024 {
    str(calc.round(bytes / 1024, digits: 1)) + " KB"
  } else {
    str(bytes) + " B"
  }
}

#let ratio(original, compressed) = {
  if original == 0 { "0%" }
  else {
    str(calc.round((1 - compressed / original) * 100, digits: 1)) + "%"
  }
}

`typst-bake` embeds all resources (templates, fonts, and packages) directly into the binary at compile time. To minimize binary size, resources are compressed using zstd. Decompression is performed lazily at runtime---only when each resource is actually accessed.

The table below shows the resources embedded to generate this document.

#block(fill: luma(245), radius: 4pt, inset: 12pt)[
  *Embedded Resources*
  #table(
    columns: (1.5fr, 1fr, 1fr, 1fr),
    align: (left, right, right, right),
    stroke: none,
    inset: (x: 8pt, y: 4pt),
    [*Category*], [*Original*], [*Compressed*], [*Ratio*],
    table.hline(stroke: 0.5pt + gray),
    [Templates (#inputs.templates_count files)],
    [#format-size(inputs.templates_original)],
    [#format-size(inputs.templates_compressed)],
    [#ratio(inputs.templates_original, inputs.templates_compressed)],
    [Fonts (#inputs.fonts_count files)],
    [#format-size(inputs.fonts_original)],
    [#format-size(inputs.fonts_compressed)],
    [#ratio(inputs.fonts_original, inputs.fonts_compressed)],
    [Packages (#inputs.packages_count)],
    [#format-size(inputs.packages_original)],
    [#format-size(inputs.packages_compressed)],
    [#ratio(inputs.packages_original, inputs.packages_compressed)],
    table.hline(stroke: 0.5pt + gray),
    [*Total*],
    [*#format-size(inputs.total_original)*],
    [*#format-size(inputs.total_compressed)*],
    [*#ratio(inputs.total_original, inputs.total_compressed)*],
  )
]
