#set text(font: "Source Serif 4")

= Nested Entry Path Test

This document tests relative path resolution from various locations.

== main.typ (entry/main.typ)

=== Read ../root.txt (parent directory)
#block(stroke: 0.5pt + luma(180), inset: 1em, width: 100%)[#align(center)[#read("../root.txt")]]

=== Read sub/child.txt (child directory)
#block(stroke: 0.5pt + luma(180), inset: 1em, width: 100%)[#align(center)[#read("sub/child.txt")]]

=== Read ../other/sibling.txt (sibling directory)
#block(stroke: 0.5pt + luma(180), inset: 1em, width: 100%)[#align(center)[#read("../other/sibling.txt")]]

#include "sub/child.typ"

#include "../other/sibling.typ"
