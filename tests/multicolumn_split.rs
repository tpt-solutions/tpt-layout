//! Multi-column layout splits a block of inline text between its lines when it crosses a column
//! boundary. Other children move whole.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Px,
};

/// A two-column root of width 100 holding one paragraph of six 8-character words. Each word is
/// its own 16 px line in a 50 px column.
fn build(paragraph_style: BoxStyle) -> (BoxTree, NodeId, NodeId) {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        column_count: 2,
        column_gap: Dimension::Length(Px::ZERO),
        ..BoxStyle::default()
    });
    let paragraph = tree.insert(paragraph_style);
    tree.append_child(root, paragraph).unwrap();
    let text = tree.insert_text(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        "aaaaaaaa bbbbbbbb cccccccc dddddddd eeeeeeee ffffffff",
    );
    tree.append_child(paragraph, text).unwrap();
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
    (tree, root, text)
}

#[test]
fn paragraph_splits_across_two_columns_at_a_line_boundary() {
    let (tree, root, text) = build(BoxStyle::default());
    let fragments = &tree.get(text).unwrap().fragments;
    let positions: Vec<(f32, f32)> = fragments
        .iter()
        .map(|f| (f.x.to_f32(), f.y.to_f32()))
        .collect();
    // Three lines in the first column, then three in the second, starting back at the top.
    assert_eq!(
        positions,
        vec![
            (0.0, 0.0),
            (0.0, 16.0),
            (0.0, 32.0),
            (50.0, 0.0),
            (50.0, 16.0),
            (50.0, 32.0)
        ]
    );
    // Balanced: 96 px of text over two columns is 48 px each.
    assert_eq!(tree.get(root).unwrap().layout.height, 48.0);
}

#[test]
fn widows_keep_enough_lines_in_the_second_column() {
    // With `widows: 4` only two lines may stay in the first column, so the paragraph splits 2 and 4.
    let (tree, _, text) = build(BoxStyle {
        widows: 4,
        ..BoxStyle::default()
    });
    let fragments = &tree.get(text).unwrap().fragments;
    let first_column = fragments.iter().filter(|f| f.x == Px::ZERO).count();
    assert_eq!(first_column, 2);
    assert_eq!(fragments.len(), 6);
}
