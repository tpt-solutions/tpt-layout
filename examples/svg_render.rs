//! Lays out a small page and writes it as SVG, for looking at a layout by eye.
//!
//! Usage: `cargo run --example svg_render -- <output.svg>` (defaults to `layout.svg`).

use tpt_layout::{
    layout_document, svg::render_svg, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, Px,
};

fn main() -> std::io::Result<()> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "layout.svg".into());
    let measurer = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&measurer);

    let mut tree = BoxTree::new();
    let page = tree.insert(BoxStyle {
        width: Dimension::Length(Px::from_px(400)),
        ..BoxStyle::default()
    });
    let heading = tree.insert(BoxStyle {
        height: Dimension::Length(Px::from_px(24)),
        ..BoxStyle::default()
    });
    tree.append_child(page, heading).expect("append");
    let paragraph = tree.insert(BoxStyle {
        display: Display::Block,
        ..BoxStyle::default()
    });
    tree.append_child(page, paragraph).expect("append");
    let text = tree.insert_text(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
    );
    tree.append_child(paragraph, text).expect("append");

    let result = layout_document(
        &mut tree,
        page,
        LayoutConstraints {
            width: Px::from_px(400),
            height: None,
        },
        &ctx,
    )
    .expect("layout");
    let svg = render_svg(&tree, page, result.width.to_f32(), result.height.to_f32());
    std::fs::write(&out, svg)?;
    println!("wrote {out}");
    Ok(())
}
