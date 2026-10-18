//! A block whose children are all inline is divided between its lines, with orphan and widow
//! control, like a block-level text leaf.

use tpt_layout::{
    layout_document, paginate, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, PageConstraints, Px,
};

/// A block of width 80 holding one paragraph of five 8-character words, one word per line (lines
/// are 16 px tall). The paragraph has the given orphan and widow counts.
fn build(orphans: u16, widows: u16) -> (BoxTree, NodeId, NodeId) {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: Dimension::Length(Px::from_px(80)),
        ..BoxStyle::default()
    });
    let paragraph = tree.insert(BoxStyle {
        orphans,
        widows,
        ..BoxStyle::default()
    });
    tree.append_child(root, paragraph).unwrap();
    let text = tree.insert_text(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        "aaaaaaaa bbbbbbbb cccccccc dddddddd eeeeeeee",
    );
    tree.append_child(paragraph, text).unwrap();
    layout_document(
        &mut tree,
        root,
        LayoutConstraints {
            width: Px::from_px(80),
            height: None,
        },
        &ctx,
    )
    .unwrap();
    (tree, root, paragraph)
}

/// A 48 px page: room for three lines.
fn page() -> PageConstraints {
    PageConstraints::new(Px::from_px(80), Px::from_px(48))
}

#[test]
fn paragraph_splits_between_lines_with_widows_respected() {
    let (tree, root, paragraph) = build(2, 2);
    let layout = paginate(&tree, root, &page());
    let ranges: Vec<_> = layout
        .fragments
        .iter()
        .filter(|f| f.node == paragraph)
        .map(|f| (f.page, f.lines.clone()))
        .collect();
    // Three lines fit on the first page; the remaining two are a valid widow count.
    assert_eq!(ranges, vec![(0, Some(0..3)), (1, Some(3..5))]);
}

#[test]
fn widow_control_moves_the_split_earlier() {
    let (tree, root, paragraph) = build(2, 3);
    let layout = paginate(&tree, root, &page());
    let ranges: Vec<_> = layout
        .fragments
        .iter()
        .filter(|f| f.node == paragraph)
        .map(|f| (f.page, f.lines.clone()))
        .collect();
    // Two lines would leave three for the next page, so the split moves up to two lines.
    assert_eq!(ranges, vec![(0, Some(0..2)), (1, Some(2..5))]);
}
