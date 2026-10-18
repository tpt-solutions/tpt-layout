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
- `scripts/check.sh` and a CI workflow (`.github/workflows/ci.yml`).

### Fixed during development
- Negative margins no longer give negative auto heights.
- Absolutely positioned children of boxes that look empty are laid out.
- Inline elements that contain block-level boxes lay those boxes out as atomic boxes.
- Deferred absolute boxes are shifted only by their own layout, not by later siblings.
- Bounding boxes of text with negative coordinates.
- `wrap-reverse` line positions.

### Added in 1.0.0
- Vertical writing modes (`WritingMode::VerticalRl`, `VerticalLr`, `SidewaysRl`, `SidewaysLr`) for
  subtrees that start from a vertical box, including flex, grid, tables and inline text.
  A box whose mode differs from its parent's (for example a horizontal box inside a vertical one)
  is laid out in its own mode and mapped back. `WritingMode::Inherit` is the default.
- Named grid lines (`GridLine::Named`, `Tracks::with_line_name`).
- Replaced elements (`BoxTree::insert_replaced`).
- `TreeError::TooDeep` and `flow::MAX_DEPTH` (128): deeper trees are rejected instead of
  overflowing the stack.
- Adapted official-suite tests (`tests/wpt_*.rs`), each naming its web-platform-tests source.
- `examples/profile_scaling.rs`.

### Changed
- Grid templates (`grid_template_columns`, `grid_template_rows`, `grid_template_areas`) moved from
  `BoxStyle` to the node, set with `BoxTree::set_grid_template` (`GridTemplate`). `BoxStyle` is now
  about 300 bytes instead of 1 KB.

### Fixed
- `fr` tracks whose sum is below one take that fraction of the free space, as the spec requires.
- A vertical box whose containing height is unknown caps its natural inline size, so percentages
  of an unknown size cannot grow without bound.
- Boundaries exclude vertical subtrees and boxes whose margins could escape them.
