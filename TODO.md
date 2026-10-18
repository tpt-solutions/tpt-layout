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
- [x] Table layout module (row/column sizing, header repeat across pages, row break rules). Note: no `rowspan`, no `border-collapse` (`colspan` was added later, see Phase 6).
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
- [x] Final Rust API shape for tables and RTL (not in spec). Decided: keep the current names. Tables use `Display::Table`, `TableRow` and `TableCell`; RTL is `BoxStyle::direction`. Recorded in CHANGELOG.md.
- [ ] Real tpt-text / tpt-canvas / tpt-math integration once those crates exist

## Phase 6: Review follow-ups (from the 2026-10 platform review)
Plan: `C:\Users\phill\.claude\plans\review-platform-for-bugs-optimized-allen.md`

### 6A. Bugs and stale items
- [x] `relayout` runs a full pass when root or constraints differ from the last full layout
- [x] Fix stale docs (README writing-mode note, `grid.rs` named-lines note, "TODO Phase N" module headers)
- [x] CI and `scripts/check.sh` run `cargo fmt --check` and `cargo doc -D warnings`
- [x] Merged duplicated Added/Fixed sections under 1.0.0 in CHANGELOG.md
- [x] `tree.rs` `push`: the `u32` overflow `expect` is the only library panic path. Decided to keep: it needs more than 4 billion nodes. Revisit if `insert` ever returns `Result`
- [x] Multi-column (`pagination.rs`): negative column width with a large gap is clamped to zero, and the balanced-height arithmetic saturates. Test: `oversized_column_gap_clamps_instead_of_going_negative`
- [ ] Multi-column: children are still not split across columns (a tall child is balanced whole)
- [x] Slot reuse in `BoxTree::remove`: free list plus per-node generations, so stale ids resolve to `None`. Tests in `tests/tree_edit.rs`
- [x] Tracks beyond `MAX_TRACKS`: the review said these were silently dropped, but `Tracks::new` already panics with a message. Added `Tracks::try_new`, which returns `None` instead
- [x] `BoxTree` move/insert API: `insert_child_at`, `insert_before`, `detach`, `reparent`. Tests in `tests/tree_edit.rs`

### 6B. Missing features for documents
- [x] Orphans/widows for inline paragraphs. Done for blocks whose children are all inline: their line boxes are split with orphan and widow control, like text leaves. Blocks that mix inline and block children still do not split. Tests: `tests/paragraph_split.rs`
- [x] `text-align`: start, center, end and justify (`BoxStyle::text_align`, `TextAlign`). RTL start aligns right, trailing spaces are ignored, and justify leaves the last line at start. Tests: `tests/text_align.rs`
- [x] `text-indent` on the first line of an inline run that starts its block. Block-level text leaves are not indented. Tests: `tests/text_indent.rs`
- [x] `line-height` (`BoxStyle::line_height`): a length adds half-leading above and below each inline line. `auto` keeps the natural height. Percentages and numbers are not supported. Tests: `tests/inline_leading.rs`
- [x] `white-space: nowrap` (`WhiteSpace::NoWrap`): inline content stays on one line and may overflow. `pre` is listed below.
- [x] `white-space: pre` (`WhiteSpace::Pre`): no wrapping, runs of spaces kept, and each newline starts a new line. Blank lines (two newlines in a row) are dropped and tabs are not expanded. Tests: `tests/white_space_pre.rs`
- [x] `white-space: pre-wrap` (`WhiteSpace::PreWrap`): as `pre`, but lines wrap at spaces. Tests: `tests/white_space_pre.rs`
- [x] Inline padding, border and margin on spans and links: their horizontal space is added as gaps around the children, and the element's frame covers it. Vertical padding does not change the line box, as in CSS. Tests: `tests/inline_padding.rs`
- [x] Parent top-margin collapse with an empty first child (`flow.rs`). The margins collapse to the larger value, and the empty child's top margin is not added twice. Tests: `tests/margin_empty_child.rs`
- [x] Flex `min-width: auto` as min-content. Approximate: widest word for text, sum for flex rows, max otherwise, capped by a fixed width. Only for row main axes; column items keep a zero minimum. Tests: `tests/flex_min_width.rs`
- [x] Table `colspan` (`BoxStyle::column_span`): a spanning cell widens its columns evenly when they are too narrow, and is laid out across them. Tests: `tests/table_colspan.rs`
- [x] `table-layout: fixed` (`BoxStyle::table_layout`): the first row's declared cell widths set their columns, and undeclared columns share the rest equally. Tests: `tests/table_fixed.rs`
- [x] Table `rowspan` (`BoxStyle::row_span`): cells are placed on a grid, so later rows skip columns held from above. A spanning cell must fit its rows, and any shortfall goes to its last row. Pagination keeps a spanning cell with the row it starts in. Tests: `tests/table_rowspan.rs`
- [x] Grid `justify-content` and `align-content`: leftover space is shared out for fixed tracks. `fr` tracks already fill the container. Tests: `tests/grid_alignment.rs`
- [x] Grid `repeat()`: `Tracks::repeat(count, pattern)` writes a fixed pattern out (up to `MAX_TRACKS`). `repeat(auto-fill, <px>)` is `GridTemplate::auto_fill_columns`, which fits as many fixed px columns as the container allows, after any explicit columns. `auto-fit` is `GridTemplate::auto_fit`: the same columns, with the ones no item occupies collapsed to zero. Auto repeats with fr, and non-px widths, are not supported. Tests: `tests/grid_repeat.rs`
- [x] `display: none`: the box takes no space, paints nothing, and its children are skipped. `BoxTree::children` still returns it, and layout uses the crate-private `laid_children`. Toggling it stays incremental. Tests: `tests/display_none.rs`. Note: it does not break an inline run.
- [x] `aspect-ratio` (`BoxStyle::aspect_ratio`): an auto height comes from the content width in block flow. Not applied in flex, grid or intrinsic sizing. Tests: `tests/aspect_and_positioning.rs`
- [x] `position: fixed` is laid out as absolute. Deviation: the containing block is the nearest positioned ancestor, not the viewport, since there is no viewport.
- [x] `position: sticky` offsets like relative and never sticks, since there is no scroll model.
- [x] Header/footer placement: `PageLayout::place_running` adds the same header and footer nodes to every page, in the space reserved by `header_height`/`footer_height`. Per-page content such as page numbers is not supported. Tests: `tests/running_content.rs`
- [ ] Richer `TextMeasurer` input (font, size, language, direction). **Decision: defer until `tpt-text` exists.** Its real API decides what the engine needs, and guessing now would mean changing the trait again.
- [x] API-deviations note vs `spec.txt`: `docs/spec-deviations.md`

### 6C. Tests and CI
- [x] Test: `relayout` with changed constraints
- [x] Test: `remove` then `relayout` matches a full layout
- [ ] Tests still missing: inline-paragraph pagination, RTL plus vertical modes, percentages in nested roots, tables across pages, triple writing-mode nesting
- [x] Property tests: layout idempotence, and pagination gives each text box contiguous, disjoint line ranges from 0 (`tests/properties.rs`, seeded generator)
- [x] Unit tests in `src/`: grid track spacing, aspect-ratio and length helpers, flex justify and gap, and `is_boundary` in the engine. Still missing for table and the rest of flex
- [x] MSRV: `rust-version = "1.74"` in Cargo.toml. Verified locally by running the full test suite on 1.74; the `msrv` CI job does the same (it deletes the tracked Cargo.lock, which newer cargo writes in a format 1.74 cannot read)
- [x] Cross-OS determinism: the pagination hash and golden JSON are pinned constants, and the test matrix runs them on Linux, macOS and Windows, so every OS is compared to the same values.
- [x] Cargo cache in the main CI job (`Swatinem/rust-cache`). Not yet run on GitHub.
- [ ] `rust-toolchain.toml`: **Decision: do not add.** CI uses `stable` and the MSRV job pins 1.74, so a file would only pin local development. Add it if the team wants one toolchain everywhere.
- [ ] Run wasm tests: **Decision: not now.** The wasm build is checked; running tests needs a wasm runner, which is a new tool to maintain.
- [ ] `cargo-fuzz`: optional. The seeded robustness tests cover the same ground for now.
- [ ] Push and confirm CI is green on GitHub (never run yet)

### 6D. Differentiators
- [x] Stable layout fingerprint: `snapshot::fingerprint`, an FNV-1a hash over fixed-point geometry (`tests/snapshot.rs`)
- [ ] `explain(node)`: why a box has its size. **Decision: defer.** Useful, but it needs a design for recording the constraint chain in every pass, and no one has asked for it yet.
- [x] Layout diff: `snapshot::diff` compares a clone of the tree taken before an edit with the tree after it, reporting changed, added and removed nodes
- [ ] `fit_to_pages(n)` helper. **Decision: do not build.** The meaning is not defined (scale, reflow or trim?), so it would be a guess. Build it when a specific use case says what it should do.
- [ ] Serde for trees and results. **Decision: not in the core.** The core stays dependency-free, so serialization belongs in a separate crate that depends on serde. Not started.

### 6E. Adoption
- [x] SVG debug renderer: `svg::SvgCanvas` implements `Canvas`, and `svg::render_svg` draws it. Text is drawn with its characters, inline and block-level. Block-level lines are also outlined. Vertical text is drawn one character per cell. `LineBox` now carries its byte range (a breaking change, noted in CHANGELOG). Example: `examples/svg_render.rs`
- [ ] CLI `tpt-layout` (separate workspace crate): JSON tree in, layout JSON or SVG out. **Decision: a separate crate, so the core stays dependency-free.** Not started; it needs a JSON parser and so depends on the serde decision above.
- [ ] wasm-bindgen playground (static page, GitHub Pages). **Decision: separate crate, later.** It needs wasm-bindgen, which the core must not depend on.
- [x] Starter templates, golden-tested as SVG (`src/templates.rs`, `tests/templates.rs`, `examples/templates.rs`): invoice (table with a header row), dashboard (grid), navbar (flex), resume (two columns). Their text is inline, so the SVG shows the words.
- [x] Also added: RTL page, vertical page, and a two-column newsletter (`tests/golden/templates/`). Vertical text stacks one character per cell in the SVG, which is exact only for the fixed-advance measurer.
- [x] Report template with orphan and widow control, paginated with `Template::paginate` (`templates::report`; `examples/report.rs`; `tests/templates_report.rs`).
- [x] Invoice header repeats across pages. The paginator read `table_header` only from cells, but the template sets it on the row, so no header ever repeated. It now honours the row or the cell. Tests: `tests/templates_invoice.rs`.
- [x] `direction` and `text-align` are inherited, as in CSS. Both have an `Inherit` default, resolved by walking up the tree (`BoxTree::effective_direction` / `effective_text_align`). Tests: `tests/inheritance.rs`. `line-height` and `white-space` are not inherited, and neither is `text-indent`.
- [ ] `BoxStyle` builder and fluent tree builder
- [ ] Optional CSS-subset parser behind a `css` feature. **Decision: do not build.** `spec.txt` leaves styling input to the downstream apps; a parser would be a second, partial CSS engine to maintain.
- [x] Docs: `docs/quickstart.md` (walkthrough of `examples/quickstart.rs`) and `docs/text-measurer.md` (the trait's rules). Linked from README.
- [x] CSS-differences table: `docs/css-differences.md`, linked from README.
- [x] `#![warn(missing_docs)]` enabled, with every public item documented (it gave 206 warnings before).
- [ ] mdbook and cookbook. Not planned as an mdbook for now: the `docs/` pages cover the same ground in plain markdown.
- [ ] Publish readiness: LICENSE, Cargo.toml metadata, flip `publish`, tag, release workflow, issue template
- [ ] Example with a real font-backed `TextMeasurer` (dev-dependency). **Decision: do not add.** It needs a font-parsing dependency; `docs/text-measurer.md` shows how to write one.
