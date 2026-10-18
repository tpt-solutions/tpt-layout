# tpt-layout

Deterministic box, flow and page layout engine for the TPT ecosystem. Pure Rust, `no_std` + `alloc`,
no `unsafe`. Design contract: [spec.txt](spec.txt). Work plan and status: [TODO.md](TODO.md).

## Architecture

```
BoxTree (arena) ──► layout_document / layout_flow ──► ComputedLayout per node (f32)
   │                    │
   │                    ├─ block + inline flow, margin collapsing, floats  (flow.rs, inline.rs)
   │                    ├─ flexbox                                          (flex.rs)
   │                    ├─ grid                                             (grid.rs)
   │                    ├─ tables and multi-column                          (table.rs, pagination.rs)
   │                    └─ absolute / relative positioning                  (position.rs, flow.rs)
   │
   ├─ relayout()  incremental re-layout of dirty layout boundaries           (engine.rs)
   ├─ paginate()  page breaks, orphans/widows, repeated table headers        (pagination.rs)
   ├─ paint_order()  z-index order and clip regions                          (paint.rs)
   └─ TextMeasurer trait (text.rs)  ◄── implemented by tpt-text (not yet available)
```

- **`tree`**: `BoxTree` owns every node in a slot arena addressed by `NodeId`. Slots are never reused,
  so a removed id resolves to `None` instead of aliasing a new node. Appends reject cycles and double
  parenting. Mutable access through `get_mut` marks a node dirty.
- **`style`**: `BoxStyle` holds the inputs for one box. It is `Copy`, so layout can read a style while
  it mutates the tree. CSS parsing and cascade are out of scope; the application fills these in.
- **`px`**: `Px` is the internal length, an `i32` in 1/64 px. Arithmetic saturates. Halves round away
  from zero. Floats appear only where values leave the crate (`ComputedLayout`, `Page`) or enter it
  (`Px::from_f32`, `Dimension::percent`).
- **`flow`**: single-pass, top-down block layout. Margin collapsing is decided by looking ahead
  through styles, so each box's position is known before its content is placed. Nothing is
  re-laid-out. `layout_detached` lays out a box at a position chosen by its caller (floats, flex and
  grid items, table cells, inline-blocks, absolutely positioned boxes).
- **`engine`**: `layout_document` lays out a whole tree. `relayout` re-runs only the layout
  boundaries that contain dirty nodes, and falls back to a full pass when a change cannot be confined.
- **`canvas`**: stub trait for the drawing boundary. tpt-layout never draws.

## Layout boundaries

Incremental layout re-runs a *boundary* instead of the whole tree. A box is a boundary when all of
the following hold:

- it is positioned (`relative` or `absolute`) and not an inline element;
- its width and height are px lengths;
- its padding, border, insets, and min/max sizes contain no percentages;
- its parent is a block-flow container (block, inline, or inline-block), so it is not resized by a
  flex, grid or table parent;
- it establishes a formatting context (`overflow` other than `visible`, `absolute`, `flex`, `grid`,
  `table`, `inline-block`), or it has top padding or border, so no margin escapes it.

A change inside a boundary cannot move anything outside it, except through its baseline. If a
re-run changes the baseline, `relayout` runs a full pass. The randomized tests check that the
result always equals a full pass.

## Conventions

- Layout is single-pass. No step iterates to convergence (spec, Architect's Note).
- Coordinates are absolute document coordinates. `x`/`y` are the border-box origin.
- Output `baseline` is measured from the top of the border box.
- Determinism: integer math in every layout path, and no reliance on platform float behaviour.
  `scripts/check.sh` and CI run the same tests on each platform. The pagination hash test and the
  golden layout files pin the output.

## Status

Phases 0 to 5 are implemented with tests. Version 1.0.0 is set locally and not published. Known
limits are listed in [TODO.md](TODO.md).

## Checks

```sh
sh scripts/check.sh
```

Runs the no_std build, the `std`-feature build, the test suite (including golden files, the
pagination hash, the adapted official-suite tests, and 300 randomized documents; `FUZZ_CASES=50000`
runs more), clippy, the examples,
the unsafe-code audit, and the wasm32 build. The wasm step needs
`rustup target add wasm32-unknown-unknown`.

To regenerate the golden layouts after an intended change: `UPDATE_GOLDEN=1 cargo test`.

## Documentation

- [docs/quickstart.md](docs/quickstart.md): build, lay out and paginate a tree.
- [docs/text-measurer.md](docs/text-measurer.md): writing a `TextMeasurer` for real fonts.
- [docs/css-differences.md](docs/css-differences.md): where the layout differs from CSS.

## Examples

- `cargo run --example quickstart`: the smallest program, from tree to positions.
- `cargo run --example document`: block, flex and grid content, paginated to A4.
- `cargo run --release --example incremental_bench`: full layout against a single-box relayout.
