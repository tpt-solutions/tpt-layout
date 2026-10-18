//! `line-height` (half-leading on each line) and `white-space: nowrap` for inline runs.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Px, WhiteSpace,
};

/// Lays out a 100 px block with `text` and the given block style tweaks. Returns the tree and
/// the inline text leaf.
fn layout_block(text: &str, block: BoxStyle) -> (BoxTree, NodeId) {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let para = tree.insert(BoxStyle {
        width: Dimension::Length(Px::from_px(100)),
        ..block
    });
    tree.append_child(root, para).unwrap();
    let leaf = tree.insert_text(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        text,
    );
    tree.append_child(para, leaf).unwrap();
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
    (tree, leaf)
}

fn fragment_ys(tree: &BoxTree, leaf: NodeId) -> Vec<f32> {
    tree.get(leaf)
        .unwrap()
        .fragments
        .iter()
        .map(|f| f.y.to_f32())
        .collect()
}

#[test]
fn natural_line_height_is_unchanged_by_default() {
    let (tree, leaf) = layout_block("ab cd ef gh ij", BoxStyle::default());
    let ys = fragment_ys(&tree, leaf);
    // Four words on line one at y=0; "ij" starts line two at the natural 16 px line height.
    assert_eq!(ys, vec![0.0, 0.0, 0.0, 0.0, 16.0]);
}

#[test]
fn line_height_adds_half_leading_above_and_below() {
    // Natural height is 16 (12 ascent + 4 descent). A 30 px line adds 14 px split 7 above and 7
    // below, so each word's top sits 7 px into its line, and the second line starts at 30.
    let (tree, leaf) = layout_block(
        "ab cd ef gh ij",
        BoxStyle {
            line_height: Dimension::Length(Px::from_px(30)),
            ..BoxStyle::default()
        },
    );
    let ys = fragment_ys(&tree, leaf);
    // The first four words are on line one, and "ij" starts line two.
    assert_eq!(ys[0], 7.0, "{ys:?}");
    assert_eq!(*ys.last().unwrap(), 37.0, "{ys:?}");
}

#[test]
fn nowrap_keeps_everything_on_one_line() {
    // 112 px of text in a 100 px block: with wrapping it would take two lines.
    let (tree, leaf) = layout_block(
        "ab cd ef gh ij",
        BoxStyle {
            white_space: WhiteSpace::NoWrap,
            ..BoxStyle::default()
        },
    );
    let ys = fragment_ys(&tree, leaf);
    assert_eq!(ys.len(), 5);
    assert!(ys.iter().all(|y| *y == 0.0), "{ys:?}");
}
