# Quickstart

This walks through `examples/quickstart.rs`. Run it with:

```sh
cargo run --example quickstart
```

## 1. Choose a text measurer

The layout engine never reads fonts. It asks a `TextMeasurer` how wide text is and how it breaks
into lines. The crate ships `FixedAdvanceMeasurer`, a monospace-style mock, for tests and demos:

```rust
let measurer = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
let ctx = LayoutContext::new(&measurer);
```

For real output, implement `TextMeasurer` for your font. See [text-measurer.md](text-measurer.md).

## 2. Build a box tree

Boxes are created with a `BoxStyle` and attached to a parent with `append_child`:

```rust
let mut tree = BoxTree::new();
let page = tree.insert(BoxStyle {
    width: Dimension::Length(Px::from_px(200)),
    ..BoxStyle::default()
});
```

- `insert` creates a box.
- `insert_text` creates a text leaf. Inside a block, wrap it in an inline run:
  `Display::Inline` on the text, with the block as its parent.
- `insert_replaced` creates an image-like box with an intrinsic size.

`BoxStyle::default()` is a block-level box that sizes to its parent. Set only what you need.

## 3. Lay it out

```rust
let result = layout_document(
    &mut tree,
    page,
    LayoutConstraints { width: Px::from_px(200), height: None },
    &ctx,
)?;
```

`result` gives the root's size. Each box's position and size are in `tree.get(id)?.layout`, in
document coordinates, as `f32` pixels.

## 4. Change it and relayout

After editing a box with `tree.get_mut(id)`, call `relayout` with the same root and constraints.
It re-runs only the affected layout boundaries, and gives the same answer as a full layout.
If you change the root or the constraints, `relayout` runs a full layout instead.

## 5. Paginate (optional)

```rust
let pages = paginate(&tree, page, &PageConstraints::a4());
```

Each `PageFragment` says which page a box or a range of lines landed on. Pagination does not move
boxes. Use the fragment offsets when you draw.

## 6. Draw (optional)

`svg::render_svg` draws the tree as SVG, which is useful for checking a layout by eye. For your own
output, walk the tree with `paint_order` and read each box's `layout`.

## Next steps

- `examples/document.rs`: block, flex and grid content, paginated to A4.
- `src/templates.rs`: starter layouts (invoice, dashboard, navbar, resume, report, newsletter).
- [README.md](../README.md): architecture and conventions.
