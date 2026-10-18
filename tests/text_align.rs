//! `text-align` for inline runs: start, center, end, RTL start, and trailing spaces.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Direction, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Px, TextAlign,
};

/// Lays out one block of width 100 px holding `text` as an inline run, and returns the x of the
/// text leaf.
fn text_x(text: &str, align: TextAlign, direction: Direction) -> f32 {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let block = tree.insert(BoxStyle {
        width: Dimension::Length(Px::from_px(100)),
        text_align: align,
        direction,
        ..BoxStyle::default()
    });
    tree.append_child(root, block).unwrap();
    let leaf: NodeId = tree.insert_text(
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
    tree.get(leaf).unwrap().layout.x
}

#[test]
fn start_is_the_left_edge() {
    assert_eq!(text_x("ab", TextAlign::Start, Direction::Ltr), 0.0);
}

#[test]
fn center_splits_the_free_space() {
    // "ab" is 16 px wide, so 84 px are free and half of that is 42 px.
    assert_eq!(text_x("ab", TextAlign::Center, Direction::Ltr), 42.0);
}

#[test]
fn end_pushes_to_the_right_edge() {
    assert_eq!(text_x("ab", TextAlign::End, Direction::Ltr), 84.0);
}

#[test]
fn start_in_rtl_aligns_right() {
    assert_eq!(text_x("ab", TextAlign::Start, Direction::Rtl), 84.0);
}

#[test]
fn trailing_space_does_not_shift_centred_text() {
    // Without the trailing space the result must match the no-space case.
    let plain = text_x("ab", TextAlign::Center, Direction::Ltr);
    let spaced = text_x("ab ", TextAlign::Center, Direction::Ltr);
    assert_eq!(plain, spaced);
}

/// Lays out a 100 px block with `text` justified and returns the x of each text leaf, in order.
fn justified_xs(text: &str) -> Vec<f32> {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let block = tree.insert(BoxStyle {
        width: Dimension::Length(Px::from_px(100)),
        text_align: TextAlign::Justify,
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
    let node = tree.get(leaf).unwrap();
    node.fragments.iter().map(|f| f.x.to_f32()).collect()
}

#[test]
fn justify_leaves_the_last_line_at_start() {
    // One line only: it is the last line, so it stays at the start edge.
    let xs = justified_xs("ab cd");
    assert_eq!(xs[0], 0.0);
}

#[test]
fn justify_spreads_spaces_on_a_wrapped_line() {
    // Words are 16 px and spaces 8 px. "ab cd ef gh" is 88 px, so 12 px are free across its three
    // spaces: each gains 4 px. "ij" does not fit and starts the last line.
    let xs = justified_xs("ab cd ef gh ij");
    assert_eq!(xs, vec![0.0, 28.0, 56.0, 84.0, 0.0], "{xs:?}");
}
