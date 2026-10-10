# tpt-layout TODO

Source of truth: [spec.txt](spec.txt). Check items off as they land.

## Decisions (from clarifying Q&A)
- Crate at repo root, edition 2021, stable Rust, `#![no_std]` + `alloc`; `std` feature for tests only; `git init`.
- Dependencies (tpt-text / tpt-canvas / tpt-math / tpt-testkit) are **stubbed via traits** in this crate; no external tpt deps yet.
- **Fixed-point** internal math: `Px(i32)` in 1/64 px. `f32` only at the API boundary (`ComputedLayout`, `BoxStyle` conversions).
- **Arena** tree: `BoxTree` owns `Vec<BoxNode>` indexed by `NodeId`; layout fns take `&mut BoxTree` + `NodeId` (spec's slice signatures adapted).
- `TextMeasurer` trait returns line boxes (width, height, baseline) for a width constraint; orphan/widow uses line count.
- Extras beyond spec in v1.0: percentages & `auto` sizes, box-sizing, overflow, z-index (ordering metadata), tables, writing modes / RTL.
- Whole roadmap, implemented phase by phase starting at Phase 1.
- Core rule: single-pass, top-down layout. Never iterate to convergence.

## Phase 0: Scaffold
- [ ] `git init`, `.gitignore`
- [ ] `Cargo.toml` (edition 2021, `std` feature, `no_std` + `alloc`)
- [ ] `src/lib.rs` with `#![no_std]`, `extern crate alloc`, module skeleton
- [ ] `Px` fixed-point newtype (1/64 px): add/sub/mul/div, rounding rules, f32 conversion, saturating ops
- [ ] Deterministic integer-only helpers (no float in layout paths, no libm needed)
- [ ] Stub traits: `TextMeasurer`, `Renderer`-facing output notes (canvas), geometry helpers (math)
- [ ] Mock `TextMeasurer` (fixed-advance font) for tests
- [ ] CI-style check script: `cargo build --no-default-features`, `cargo test`, wasm32 build check
- [ ] README with architecture overview

## Phase 1: Box Model & Block Flow (Weeks 1-3)
- [ ] `NodeId`, `BoxNode`, `BoxTree` arena (insert, append_child, remove, parent/children access)
- [ ] `BoxStyle`: width/height/min/max, margin, border, padding, position, display, box-sizing, overflow, z-index
- [ ] `Dimension` enum: `Auto | Length | Percent`
- [ ] `ComputedLayout` (x, y, width, height, baseline) with f32 output
- [ ] Margin collapsing rules (block siblings, parent/child, empty blocks)
- [ ] `layout_flow` block layout: vertical stacking, auto width/height, min/max clamping
- [ ] Relative and absolute positioning (containing block resolution)
- [ ] `TextMeasurer` integration for text box sizing and baselines
- [ ] Tests: headings + paragraphs document
- [ ] **Milestone:** simple document with headings and paragraphs lays out correctly

## Phase 2: Flexbox (Weeks 4-6)
- [ ] Flex container/item style properties (direction, wrap, justify-content, align-items/self/content, gap, grow/shrink/basis, order)
- [ ] Flex-basis resolution and hypothetical main sizes
- [ ] Line collection (single-line and `wrap`/`wrap-reverse`)
- [ ] Free-space distribution with grow/shrink, min/max clamping and freezing (bounded loop, no convergence iteration)
- [ ] Main-axis `justify-content`
- [ ] Cross-axis sizing, `align-items`, `align-self`, `align-content`
- [ ] Baseline alignment
- [ ] Row/column and reverse directions, RTL awareness
- [ ] Tests adapted from CSS Flexbox suite
- [ ] **Milestone:** toolbars and card grids lay out correctly

## Phase 3: Grid & Inline Flow (Weeks 7-9)
- [ ] Grid style properties (template rows/cols/areas, gaps, auto-flow, auto tracks)
- [ ] Track sizing: `px`, `%`, `fr`, `minmax()`, `min-content`, `max-content`, `auto`
- [ ] Explicit placement (`grid-row`, `grid-column`, `grid-area`, named lines/areas, spans)
- [ ] Auto-placement (row/column, dense)
- [ ] Item alignment (`justify-*`, `align-*`) within cells
- [ ] Inline flow: line boxes, wrapping, vertical-align/baseline
- [ ] Inline-block and inline images
- [ ] Float left/right and `clear`, text wrapping around floats
- [ ] Tests adapted from CSS Grid suite
- [ ] **Milestone:** 2D grids and documents with inline images

## Phase 4: Pagination (Weeks 10-12)
- [ ] `PageConstraints`: sizes (A4, Letter, custom), margins, header/footer regions
- [ ] `PageLayout` / `Page` structs
- [ ] Break rules: `break-before/after/inside` (auto, avoid, always, left, right)
- [ ] Valid break point discovery
- [ ] Orphan/widow protection (using line counts from `TextMeasurer`)
- [ ] `paginate()`: split continuous layout, adjust y offsets per page
- [ ] Oversized box handling (taller than a page)
- [ ] Multi-column layouts (column-count/width/gap, balancing)
- [ ] Table layout module (row/column sizing, header repeat across pages, row break rules)
- [ ] **Milestone:** tpt-doc can generate paginated PDFs with correct page breaks

## Phase 5: Incremental Layout & Polish (Weeks 13-14)
- [ ] Dirty-flag tracking per node, propagation up to layout boundaries
- [ ] Cached constraints/results per node to skip clean subtrees
- [ ] Incremental re-layout entry point
- [ ] `layout_document` orchestrator, `LayoutContext` (DPI, metrics), `LayoutConstraints`, `LayoutResult`
- [ ] Writing modes / RTL pass across block, flex, grid, inline
- [ ] z-index / paint order metadata output
- [ ] Overflow handling metadata
- [ ] Performance optimization and profiling
- [ ] Docs, examples, changelog
- [ ] Publish v1.0.0

## Cross-cutting: Testing & Quality
- [ ] Golden-file JSON layout tests (stubbed testkit golden harness)
- [ ] Deterministic pagination tests (hash of output; CI on Linux/macOS/Windows)
- [ ] Flexbox/Grid conformance suites adapted to the Rust API
- [ ] Incremental vs full re-layout equality test (identical output)
- [ ] Incremental benchmark: single-box change at least 10x faster than full re-layout
- [ ] Property/fuzz tests: termination, no panics, no overflow on huge/degenerate input
- [ ] `no_std` + wasm32 build verified in CI
- [ ] Zero C-FFI / no unsafe audit

## Open items to confirm later
- [ ] Final Rust API shape for tables and RTL (not in spec)
- [ ] Real tpt-text / tpt-canvas / tpt-math integration once those crates exist
