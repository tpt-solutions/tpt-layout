//! `text-indent` on the first line of an inline run that starts its block.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, Px,
};

/// Lays out a 100 px block with `text` and the given indent, and returns the fragment x values.
fn fragment_xs(indent: i32, text: &str) -> Vec<f32> {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let block = tree.insert(BoxStyle {
        width: Dimension::Length(Px::from_px(100)),
        text_indent: Dimension::Length(Px::from_px(indent)),
        ..BoxStyle::default()
    });
    tree.append_child(root, block).unwrap();
    let leaf = tree.insert_text(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        text,
    );
    tree.append_child(block, leaf).unwrap();
    layout_document(
        &mut tree,
        root,
        LayoutConstraints {
            width: Px::from_px(100),
            height: None,
        },
        &ctx,
    )
    .unwrap();
    tree.get(leaf)
        .unwrap()
        .fragments
        .iter()
        .map(|f| f.x.to_f32())
        .collect()
}

#[test]
fn first_line_starts_at_the_indent() {
    assert_eq!(fragment_xs(24, "ab cd"), vec![24.0, 48.0]);
}

#[test]
fn indent_shortens_the_first_line_only() {
    // With a 24 px indent the first line has 76 px: "ab cd ef" (64 px) fits and "gh" does not.
    // The second line starts back at the left edge.
    assert_eq!(
        fragment_xs(24, "ab cd ef gh ij"),
        vec![24.0, 48.0, 72.0, 0.0, 24.0]
    );
}

#[test]
fn zero_indent_changes_nothing() {
    assert_eq!(fragment_xs(0, "ab cd"), vec![0.0, 24.0]);
}
