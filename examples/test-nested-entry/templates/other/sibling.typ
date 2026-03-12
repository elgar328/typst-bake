== sibling.typ (other/sibling.typ)

=== Read sibling.txt (same directory)
#block(stroke: 0.5pt + luma(180), inset: 1em, width: 100%)[#align(center)[#read("sibling.txt")]]

=== Read ../root.txt (parent directory)
#block(stroke: 0.5pt + luma(180), inset: 1em, width: 100%)[#align(center)[#read("../root.txt")]]

=== Read ../entry/sub/child.txt (different branch)
#block(stroke: 0.5pt + luma(180), inset: 1em, width: 100%)[#align(center)[#read("../entry/sub/child.txt")]]
