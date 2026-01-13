= Image Embedding Example

This document demonstrates embedding images with typst-bake.

The image below is bundled into the binary at compile time:

#align(center)[
  #image("images/sample.jpeg", width: 60%)
]

Images in the template directory are automatically included and can be referenced using relative paths.
