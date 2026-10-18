# Changelog

## 1.0.0 (not published)

Complete implementation of the layout engine. Not published: see TODO.md.

### Added
- Arena box tree with dirty tracking.
- Fixed-point `Px` math with documented rounding.
- Block and inline flow with margin collapsing, floats and `clear`.
- Flexbox: wrap, grow/shrink with a bounded freeze loop, justify, align, baseline, RTL.
- Grid: px/%/fr/minmax tracks, explicit and auto placement, named areas and named lines, dense packing, alignment.
- Tables: column sizing from max-content, row heights, header rows.
- Replaced elements (`BoxTree::insert_replaced`): intrinsic sizes, inline or block.
- Absolute and relative positioning.
- Pagination: page constraints (A4, Letter, custom), break rules, orphans and widows, oversized
  boxes, repeated table headers, left and right page parity.
- Multi-column layout with balanced column heights.
- Incremental re-layout (`relayout`) over layout boundaries, with a full-pass fallback.
- Paint order by z-index, and clip regions from `overflow`.
- Golden layout tests, a pagination hash test, randomized robustness tests, and an
  incremental-equals-full test.
- `examples/document.rs` and `examples/incremental_bench.rs`.
- Vertical writing modes (`WritingMode::VerticalRl`, `VerticalLr`, `SidewaysRl`, `SidewaysLr`) for
  subtrees that start from a vertical box, including flex, grid, tables and inline text.
  A box whose mode differs from its parent's (for example a horizontal box inside a vertical one)
  is laid out in its own mode and mapped back. `WritingMode::Inherit` is the default.
- Named grid lines (`GridLine::Named`, `Tracks::with_line_name`).
- `TreeError::TooDeep` and `flow::MAX_DEPTH` (128): deeper trees are rejected instead of
  overflowing the stack.
- Adapted official-suite tests (`tests/wpt_*.rs`), each naming its web-platform-tests source.
- `examples/profile_scaling.rs`.
- Structural edits: `BoxTree::detach`, `insert_child_at`, `insert_before` and `reparent`.
- `Tracks::try_new`, which reports too many tracks as `None` instead of panicking.

### API decisions
- Tables use `Display::Table`, `TableRow` and `TableCell`, matching the CSS names.
- RTL is set with `BoxStyle::direction`.
- `scripts/check.sh` and a CI workflow (`.github/workflows/ci.yml`).

### Changed
- Grid templates (`grid_template_columns`, `grid_template_rows`, `grid_template_areas`) moved from
  `BoxStyle` to the node, set with `BoxTree::set_grid_template` (`GridTemplate`). `BoxStyle` is now
  about 300 bytes instead of 1 KB.
- `LineBox` has a `text` byte range, so renderers can draw each line's characters. `LineBox` is no
  longer `Copy`.
- `Direction` and `TextAlign` have an `Inherit` default and are inherited from the containing block.
- Pagination: blocks whose children are all inline split between their lines, with orphan and widow
  control, like text leaves.
- Grid: `Tracks::repeat`, `GridTemplate::auto_fill_columns` and `auto_fit` for fixed-pixel columns.
- Tables: `column_span` and `row_span` on cells, and `TableLayout::Fixed` (`table_layout`) are new.
- `Position` has `Fixed` and `Sticky`. `Fixed` is laid out as absolute; `Sticky` offsets like relative.

### Fixed
- Negative margins no longer give negative auto heights.
- Absolutely positioned children of boxes that look empty are laid out.
- Inline elements that contain block-level boxes lay those boxes out as atomic boxes.
- Deferred absolute boxes are shifted only by their own layout, not by later siblings.
- Bounding boxes of text with negative coordinates.
- `wrap-reverse` line positions.

- `fr` tracks whose sum is below one take that fraction of the free space, as the spec requires.
- A vertical box whose containing height is unknown caps its natural inline size, so percentages
  of an unknown size cannot grow without bound.
- Boundaries exclude vertical subtrees and boxes whose margins could escape them.
- `relayout` now runs a full pass when the root or constraints differ from the last full layout, instead of returning stale geometry.
- Removed stale "not supported" and "TODO Phase N" notes in docs; CI and `scripts/check.sh` now also run `cargo fmt --check` and `cargo doc -D warnings`.
- Slots freed by `BoxTree::remove` are reused. A `NodeId` now carries a generation, so stale ids still resolve to `None`.
- Multi-column layout clamps column width at zero when the gap exceeds the content width, and saturates the balanced-height arithmetic.
