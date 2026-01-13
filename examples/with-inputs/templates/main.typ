#import sys: inputs

= #inputs.title

Date: #inputs.date

#table(
  columns: (auto, 1fr, auto),
  align: (center, left, left),
  table.header(
    [*Qty*], [*Item*], [*Category*]
  ),
  ..inputs.items.map(item => (
    str(item.quantity),
    item.name,
    item.category,
  )).flatten()
)
