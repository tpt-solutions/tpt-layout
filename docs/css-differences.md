# Differences from CSS

tpt-layout implements a subset of CSS layout, and it is deterministic: the same tree and measurer
give the same geometry on every platform. Where it differs from browsers, this page lists it. The
differences are deliberate or still open, and are tracked in [TODO.md](../TODO.md).

## Units and math

- Lengths are fixed-point, in 1/64 px. Output is `f32` pixels.
- Only `px` lengths and percentages of the containing block are supported. There are no `em`,
  `rem`, `calc()`, `min()` or `max()`.
- Halves round away from zero, everywhere.

## Inheritance

- `direction` and `text-align` are inherited, as in CSS. Both have an `Inherit` default.
- `text-indent` is not inherited. CSS inherits it.
- `line-height` and `white-space` are not inherited. They are read from the block that holds the
  inline text.
- `writing-mode` is inherited, as in CSS. It is the only other inherited property. Fonts are not
  part of the layout at all: the measurer supplies line metrics.

## Box model

- `box-sizing` is supported (`content-box` and `border-box`).
- `aspect-ratio` applies to an `auto` height in block flow only. It is ignored in flex, grid and
  table cells, and in intrinsic sizing.
- `overflow` supports `visible`, `hidden` and `scroll`. `scroll` is treated like `hidden`, since
  there is no scrolling model.
- `display: none` removes a box from layout. It does not break an inline run.

## Positioning

- `position: relative` offsets the box from its flow position.
- `position: absolute` is positioned against the nearest positioned ancestor. When there is none,
  the containing block is the root's width and its content height, starting at the root's origin.
- `position: fixed` is laid out as `absolute`. CSS positions it against the viewport. There is no
  viewport here, so it uses the nearest positioned ancestor.
- `position: sticky` offsets like `relative`. It never sticks, since there is no scroll model.
- `z-index` orders paint only. There are no stacking contexts.

## Text and inline layout

- Text is measured by a `TextMeasurer` you supply. Line breaking, glyph widths and line height come
  from it, not from a font engine.
- `text-align`: `start`, `center`, `end` and `justify`. `justify` leaves the last line at the start.
- `line-height` takes lengths only. Numbers and percentages are not supported.
- `white-space`: `normal`, `nowrap`, `pre` and `pre-wrap`. `pre` drops blank lines, since a line
  with no content has no height. Tabs are not expanded.
- Inline elements add their horizontal margin, border and padding around their children. Vertical
  padding and margin do not change the line box.
- `vertical-align` is baseline only.
- Orphans and widows apply to blocks whose children are all inline, and to block-level text leaves.
  A block that mixes inline and block children is not split across pages.

## Flexbox

- `min-width: auto` on a row item is an approximate min-content width: the widest word for text,
  the sum of items for a flex row, and the widest child otherwise. CSS uses the true min-content
  size, so results can differ for complex content.
- Column items keep a zero automatic minimum.
- Basis, grow, shrink, wrap, `order` and alignment are supported. Tests in `tests/flex.rs` and the
  adapted conformance suites pin the behavior.
- Aspect ratios and baselines in column containers are not fully supported.

## Grid

- Tracks: `px`, percentages, `fr`, `auto` and `minmax()`.
- `repeat()` supports a fixed count and `auto-fill` / `auto-fit` for fixed-pixel columns. There is
  no `repeat()` with `fr`, and no `subgrid`.
- Named lines and named areas are supported. Items spanning several tracks are approximated.
- `justify-content` and `align-content` only place tracks that leave free space.

## Tables

- `column_span` and `row_span` are supported. Pagination keeps a spanning cell with the row it
  starts in.
- `table-layout: fixed` takes column widths from the first row's declared cell widths.
- `border-collapse` is not supported. Borders are separate.
- `vertical-align` is top only.

## Pagination

- Pages do not move boxes. Each page reports where its boxes and line ranges fall, and you draw
  with those offsets.
- `break-before`, `break-after` and `break-inside` are supported, along with `orphans`, `widows`
  and repeated table headers.
- Running headers and footers repeat the same nodes on every page. Per-page content such as page
  numbers is not supported.

## Multi-column

- `column-count` and `column-gap` balance content into columns.
- A block of inline text is split between its lines at a column boundary, keeping orphans and widows.
  Other children move whole, so a block text leaf, an inline element or an inline-block is never
  split. A tall child can make the columns unbalanced.
- A split paragraph keeps one box whose frame spans the columns it is in. Pagination does not split
  a paragraph that was split across columns.

## Writing modes

- `vertical-rl` and `vertical-lr` are supported, along with `sideways` modes.
- A nested box that has a different writing mode is laid out in its own mode and mapped back. A
  nested root has no physical containing block, so percentages inside it resolve against nothing.
