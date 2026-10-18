# tpt-layout TODO

Source of truth: [spec.txt](spec.txt). Check items off as they land.

Status: `[x]` is done and tested. `[ ]` is not done. Notes after an item say what is partial.

## Decisions (from clarifying Q&A)
- Crate at repo root, edition 2021, stable Rust, `#![no_std]` + `alloc`; `std` feature is optional (adds `std::error::Error` for `TreeError`); `git init`.
- Dependencies (tpt-text / tpt-canvas / tpt-math / tpt-testkit) are **stubbed via traits** in this crate; no external tpt deps yet.
- **Fixed-point** internal math: `Px(i32)` in 1/64 px. `f32` only at the API boundary (`ComputedLayout`, `Page`, `PageFragment`).
- **Arena** tree: `BoxTree` owns slots indexed by `NodeId`; slots are never reused.
- `TextMeasurer` trait returns line boxes (width, height, baseline) for a width constraint; orphan/widow uses line count.
- Extras beyond spec in v1.0: percentages & `auto` sizes, box-sizing, overflow, z-index (ordering metadata), tables, writing modes / RTL.
- Whole roadmap, implemented phase by phase starting at Phase 1.
- Core rule: single-pass, top-down layout. Never iterate to convergence. Incremental layout re-runs one boundary once, and falls back to a full pass; neither iterates.

## Phase 0: Scaffold
- [x] `git init`, `.gitignore`
- [x] `Cargo.toml` (edition 2021, `std` feature, `no_std` + `alloc`)
- [x] `src/lib.rs` with `#![no_std]`, `extern crate alloc`, module skeleton
- [x] `Px` fixed-point newtype (1/64 px): add/sub/mul/div, rounding rules, f32 conversion, saturating ops
- [x] Deterministic integer-only helpers (no float in layout paths, no libm needed). Note: style `flex_grow` and `flex_shrink` are `f32` inputs, converted once per item with IEEE basic operations.
- [x] Stub traits: `TextMeasurer`, `Renderer`-facing output notes (canvas), geometry helpers (math)
- [x] Mock `TextMeasurer` (fixed-advance font) for tests
- [x] CI-style check script: `sh scripts/check.sh` (no_std, std, tests, clippy, examples, unsafe audit, wasm32)
- [x] README with architecture overview

## Phase 1: Box Model & Block Flow (Weeks 1-3)
- [x] `NodeId`, `BoxNode`, `BoxTree` arena (insert, append_child, remove, parent/children access)
- [x] `BoxStyle`: width/height/min/max, margin, border, padding, position, display, box-sizing, overflow, z-index
- [x] `Dimension` enum: `Auto | Length | Percent`
- [x] `ComputedLayout` (x, y, width, height, baseline) with f32 output
- [x] Margin collapsing rules (block siblings, parent/child, empty blocks). Note: a parent does not collapse with an empty first child.
- [x] `layout_flow` block layout: vertical stacking, auto width/height, min/max clamping
- [x] Relative and absolute positioning (containing block resolution). Note: an auto-width absolute box fills the space between its insets rather than shrinking to fit.
- [x] `TextMeasurer` integration for text box sizing and baselines
- [x] Tests: headings + paragraphs document
- [x] **Milestone:** simple document with headings and paragraphs lays out correctly

## Phase 2: Flexbox (Weeks 4-6)
- [x] Flex container/item style properties (direction, wrap, justify-content, align-items/self/content, gap, grow/shrink/basis, order)
- [x] Flex-basis resolution and hypothetical main sizes. Note: `min-width: auto` is treated as `0`.
- [x] Line collection (single-line and `wrap`/`wrap-reverse`)
- [x] Free-space distribution with grow/shrink, min/max clamping and freezing (bounded loop, no convergence iteration)
- [x] Main-axis `justify-content`
- [x] Cross-axis sizing, `align-items`, `align-self`, `align-content`
- [x] Baseline alignment (row containers)
- [x] Row/column and reverse directions, RTL awareness
- [x] Tests adapted from CSS Flexbox suite. Note: 21 adapted cases (19 in `tests/wpt_flexbox.rs`, 2 in `tests/wpt_writing_modes.rs`) from web-platform-tests `css/css-flexbox` (align-items, align-content, align-self, flex-basis, flex-flow, flex-grow, flex-shrink, order, wrap-reverse, and vertical writing modes), each naming its source file. The suite has about 830 files; most are pixel reftests that need a renderer to compare, so this is a checked sample, not the whole suite.
- [x] **Milestone:** toolbars and card grids lay out correctly (covered by the flex and grid tests)

## Phase 3: Grid & Inline Flow (Weeks 7-9)
- [x] Grid style properties (template rows/cols/areas, gaps, auto-flow, auto tracks)
- [x] Track sizing: `px`, `%`, `fr`, `minmax()`, `min-content`, `max-content`, `auto`. Note: min-content and max-content are approximated by max-content.
- [x] Explicit placement (`grid-row`, `grid-column`, `grid-area`, spans, named areas and named lines)
- [x] Auto-placement (row/column, dense)
- [x] Item alignment (`justify-*`, `align-*`) within cells
- [x] Inline flow: line boxes, wrapping, vertical-align/baseline. Note: `text-align` is start only; `vertical-align` is baseline only.
- [x] Inline-block
- [x] Inline images (replaced elements): `BoxTree::insert_replaced` gives a leaf an intrinsic size that fills `auto` sizes; inline, it sits on the baseline. Note: no image decoding, object-fit, or aspect-ratio rules.
- [x] Float left/right and `clear`, text wrapping around floats. Note: floats do not escape their container; line widths use floats that overlap the line's top edge.
- [x] Tests adapted from CSS Grid suite. Note: 5 adapted cases (`tests/wpt_grid.rs`) from web-platform-tests `css/css-grid` (fr tracks, free-space distribution, fr sums below one, overflowing items, named areas). Cases needing `max-content` keywords, scrolling, or fonts are excluded.
- [x] **Milestone:** 2D grids and documents with inline images

## Phase 4: Pagination (Weeks 10-12)
- [x] `PageConstraints`: sizes (A4, Letter, custom), margins, header/footer regions. Note: header and footer space is reserved; no header or footer content is placed.
- [x] `PageLayout` / `Page` structs. Note: boxes are not moved. `PageFragment`s give each box's page-local position, and tree frames stay in document coordinates.
- [x] Break rules: `break-before/after/inside` (auto, avoid, always, left, right)
- [x] Valid break point discovery (text lines, whole boxes, table rows)
- [x] Orphan/widow protection (using line counts from `TextMeasurer`). Note: applies to block-level text leaves, not inline paragraphs.
- [x] `paginate()`: splits the continuous layout into pages
- [x] Oversized box handling (taller than a page)
- [x] Multi-column layouts (column-count/gap, balancing). Note: children are balanced whole; they are not split across columns.
- [x] Table layout module (row/column sizing, header repeat across pages, row break rules). Note: no `colspan`/`rowspan`, no `border-collapse`.
- [ ] **Milestone:** tpt-doc can generate paginated PDFs with correct page breaks. Note: this crate produces the page breaks; PDF output belongs to tpt-doc.

## Phase 5: Incremental Layout & Polish (Weeks 13-14)
- [x] Dirty-flag tracking per node, propagation up to layout boundaries
- [x] Cached constraints/results per node to skip clean subtrees. Note: done through layout boundaries (positioned boxes with px sizes that establish a formatting context or have top padding). There is no separate per-node constraint cache.
- [x] Incremental re-layout entry point (`relayout`)
- [x] `layout_document` orchestrator, `LayoutContext` (DPI, metrics), `LayoutConstraints`, `LayoutResult`
- [x] Writing modes / RTL pass across block, flex, grid, inline. Done: vertical-rl, vertical-lr, sideways-rl and sideways-lr; RTL for flex and grid; vertical roots in block, flex, grid, table and inline contexts (including sideways text). Mixed modes: a box whose effective mode differs from its caller's is a root of its own mode, and its frames compose through each enclosing root (`WritingMode::Inherit` is the default, so a horizontal box can sit inside a vertical one). Note: a nested root has no physical containing block, so its percentage sizes resolve against nothing and it does not stretch as a flex or grid item; intrinsic sizes are still computed as horizontal.
- [x] z-index / paint order metadata output (`paint_order`). Note: no stacking contexts.
- [x] Overflow handling metadata (clip regions in `paint_order`)
- [x] Performance optimization and profiling. Done: `examples/profile_scaling.rs` measures per-box cost by layout type and size; `BoxStyle` shrank from about 1 KB to 300 bytes by moving grid templates onto the node; nesting is bounded by `flow::MAX_DEPTH` (128) and reports `TreeError::TooDeep`.
- [x] Docs, examples, changelog (README, `examples/`, CHANGELOG.md)
- [x] Version 1.0.0 set in Cargo.toml and CHANGELOG. Not published: `publish = false`, no tag, no release. Publishing is yours to do.

## Cross-cutting: Testing & Quality
- [x] Golden-file JSON layout tests (`tests/golden/*.json`; regenerate with `UPDATE_GOLDEN=1`)
- [x] Deterministic pagination tests (hash of output). Note: the CI matrix in `.github/workflows/ci.yml` runs them on Linux, macOS and Windows, but the workflow has not been run yet.
- [x] Flexbox/Grid conformance suites adapted to the Rust API (samples; see the two items above).
- [x] Incremental vs full re-layout equality test (identical output), including 400k randomized documents
- [x] Incremental benchmark: single-box change at least 10x faster than full re-layout. Measured 40x with `examples/incremental_bench.rs` (2000 sections, release, one machine). Other documents will differ.
- [x] Property/fuzz tests: termination, no panics, no overflow on huge/degenerate input. Note: a seeded random generator, not a coverage-guided fuzzer. `FUZZ_CASES` raises the count.
- [x] `no_std` + wasm32 build verified locally (`scripts/check.sh`). The CI workflow includes it but has not been run.
- [x] Zero C-FFI / no unsafe audit (`#![forbid(unsafe_code)]`, plus a grep in `scripts/check.sh` and CI)

## Open items to confirm later
- [ ] Final Rust API shape for tables and RTL (not in spec). Current choices: tables use `Display::Table`, `TableRow` and `TableCell`; RTL is `BoxStyle::direction`.
- [ ] Real tpt-text / tpt-canvas / tpt-math integration once those crates exist
