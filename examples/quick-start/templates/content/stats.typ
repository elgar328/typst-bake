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

`typst-bake` embeds all resources (templates, fonts, and packages) directly into the binary at compile time. To minimize binary size, resources are compressed using zstd and identical contents are deduplicated. Decompression is performed lazily at runtime---only when each resource is actually accessed.

The table below shows the resources embedded to generate this document.

#block(fill: luma(245), radius: 4pt, inset: 12pt)[
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
  )

  #grid(
    columns: (auto, 1fr),
    column-gutter: 6pt,
    row-gutter: 0.65em,
    [*Compressed:*], [#format-size(inputs.total_original) #sym.arrow.r #format-size(inputs.total_compressed) (#ratio(inputs.total_original, inputs.total_compressed) reduced)],
    ..if inputs.dedup_duplicate_count > 0 {
      ([*Deduplicated:*], [#inputs.dedup_unique_blobs unique, #inputs.dedup_duplicate_count removed (#sym.minus#format-size(inputs.dedup_saved_bytes))])
    } else { () },
    [*Total:*], [#format-size(inputs.total_original) #sym.arrow.r #format-size(inputs.total_deduplicated) (#ratio(inputs.total_original, inputs.total_deduplicated) reduced)],
  )
]
