== child.typ (entry/sub/child.typ)

=== Read child.txt (same directory)
#block(stroke: 0.5pt + luma(180), inset: 1em, width: 100%)[#align(center)[#read("child.txt")]]

=== Read ../../root.txt (two levels up)
#block(stroke: 0.5pt + luma(180), inset: 1em, width: 100%)[#align(center)[#read("../../root.txt")]]

=== Read ../../other/sibling.txt (different branch)
#block(stroke: 0.5pt + luma(180), inset: 1em, width: 100%)[#align(center)[#read("../../other/sibling.txt")]]
