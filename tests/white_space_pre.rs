//! `white-space: pre` keeps spaces, starts a new line at each newline, and never wraps.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Px, WhiteSpace,
};

/// Lays out `text` in a block of `width` px with the given white-space, and returns the tree and
/// the inline text leaf.
fn layout(text: &str, white_space: WhiteSpace, width: i32) -> (BoxTree, NodeId) {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let block = tree.insert(BoxStyle {
        width: Dimension::Length(Px::from_px(width)),
        white_space,
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
            width: Px::from_px(width),
            height: None,
        },
        &ctx,
    )
    .unwrap();
    (tree, leaf)
}

#[test]
fn runs_of_spaces_are_kept() {
    // "ab" (16) + two spaces (16) puts "cd" at x = 32.
    let (tree, leaf) = layout("ab  cd", WhiteSpace::Pre, 100);
    let xs: Vec<f32> = tree
        .get(leaf)
        .unwrap()
        .fragments
        .iter()
        .map(|f| f.x.to_f32())
        .collect();
    assert_eq!(xs, vec![0.0, 32.0]);
}

#[test]
fn newline_starts_a_new_line() {
    let (tree, leaf) = layout("ab\ncd", WhiteSpace::Pre, 100);
    let fragments = &tree.get(leaf).unwrap().fragments;
    assert_eq!(fragments.len(), 2);
    assert_eq!((fragments[0].x, fragments[0].y), (Px::ZERO, Px::ZERO));
    assert_eq!(
        (fragments[1].x, fragments[1].y),
        (Px::ZERO, Px::from_px(16))
    );
}

#[test]
fn pre_never_wraps_at_spaces() {
    // "ab cd ef gh" is 88 px in a 40 px block. With `pre` it stays on one line.
    let (tree, leaf) = layout("ab cd ef gh", WhiteSpace::Pre, 40);
    let ys: Vec<f32> = tree
        .get(leaf)
        .unwrap()
        .fragments
        .iter()
        .map(|f| f.y.to_f32())
        .collect();
    assert_eq!(ys.len(), 4);
    assert!(ys.iter().all(|y| *y == 0.0), "{ys:?}");
}

#[test]
fn normal_still_wraps_the_same_text() {
    let (tree, leaf) = layout("ab cd ef gh", WhiteSpace::Normal, 40);
    let ys: Vec<f32> = tree
        .get(leaf)
        .unwrap()
        .fragments
        .iter()
        .map(|f| f.y.to_f32())
        .collect();
    assert!(ys.iter().any(|y| *y > 0.0), "{ys:?}");
}

#[test]
fn pre_wrap_keeps_spaces_but_still_wraps() {
    // "ab  cd ef gh" is 104 px in a 60 px block. The double space is kept (cd at 32), and the
    // line wraps at a space, so some words move to a second line.
    let (tree, leaf) = layout("ab  cd ef gh", WhiteSpace::PreWrap, 60);
    let fragments = &tree.get(leaf).unwrap().fragments;
    assert_eq!(
        (fragments[1].x, fragments[1].y),
        (Px::from_px(32), Px::ZERO)
    );
    assert!(fragments.iter().any(|f| f.y > Px::ZERO), "should wrap");
}
