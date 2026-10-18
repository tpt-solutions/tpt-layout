//! The smallest useful program: build a box tree, lay it out, and print where each box landed.
//!
//! Usage: `cargo run --example quickstart`. See `docs/quickstart.md` for the walkthrough.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, Px,
};

fn main() {
    // 1. A text measurer. This one is a fixed-width mock: 8 px per character, 16 px lines.
    //    Replace it with your own `TextMeasurer` for real fonts.
    let measurer = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&measurer);

    // 2. A tree: a 200 px page with a 40 px box and a paragraph of text.
    let mut tree = BoxTree::new();
    let page = tree.insert(BoxStyle {
        width: Dimension::Length(Px::from_px(200)),
        ..BoxStyle::default()
    });
    let heading = tree.insert(BoxStyle {
        height: Dimension::Length(Px::from_px(40)),
        ..BoxStyle::default()
    });
    tree.append_child(page, heading).expect("child");
    let paragraph = tree.insert(BoxStyle::default());
    tree.append_child(page, paragraph).expect("child");
    let words = tree.insert_text(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        "Layout engines place boxes, then wrap text to fit the width.",
    );
    tree.append_child(paragraph, words).expect("child");

    // 3. Lay out the page at a width of 200 px.
    let result = layout_document(
        &mut tree,
        page,
        LayoutConstraints {
            width: Px::from_px(200),
            height: None,
        },
        &ctx,
    )
    .expect("layout");
    println!(
        "page: {} x {} px",
        result.width.to_f32(),
        result.height.to_f32()
    );

    // 4. Read the geometry back. Every node has a `layout` with x, y, width and height.
    for (name, id) in [("heading", heading), ("paragraph", paragraph)] {
        let layout = tree.get(id).expect("node").layout;
        println!(
            "{name}: x={} y={} width={} height={}",
            layout.x, layout.y, layout.width, layout.height
        );
    }
}
