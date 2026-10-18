//! Incremental re-layout must produce exactly the same tree as a full layout.

mod common;

use common::*;
use tpt_layout::{
    layout_document, relayout, BoxStyle, BoxTree, Dimension, LayoutConstraints, LayoutContext,
    NodeId, Position,
};

/// A document of `count` fixed-size, positioned sections, each holding one text leaf.
/// Returns the tree, the root, and the text leaf of every section.
fn sections(count: usize, text_for: impl Fn(usize) -> String) -> (BoxTree, NodeId, Vec<NodeId>) {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let mut texts = Vec::new();
    for index in 0..count {
        let section = child(
            &mut tree,
            root,
            BoxStyle {
                position: Position::Relative,
                // A formatting context, so the section is a layout boundary (see engine.rs).
                overflow: tpt_layout::Overflow::Hidden,
                ..boxed(300, 40)
            },
        );
        texts.push(text_child(
            &mut tree,
            section,
            BoxStyle::default(),
            &text_for(index),
        ));
    }
    (tree, root, texts)
}

fn constraints() -> LayoutConstraints {
    LayoutConstraints {
        width: px(300),
        height: None,
    }
}

#[test]
fn text_change_inside_a_boundary_matches_full_layout() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);

    let (mut incremental, root, texts) = sections(30, |i| format!("item {i}"));
    layout_document(&mut incremental, root, constraints(), &ctx).unwrap();
    incremental.get_mut(texts[7]).unwrap().text =
        Some("a much longer item text that wraps onto lines".into());
    relayout(&mut incremental, root, constraints(), &ctx).unwrap();

    let (mut full, root2, texts2) = sections(30, |i| {
        if i == 7 {
            "a much longer item text that wraps onto lines".into()
        } else {
            format!("item {i}")
        }
    });
    assert_eq!(texts, texts2);
    layout_document(&mut full, root2, constraints(), &ctx).unwrap();

    assert_eq!(incremental, full);
}

#[test]
fn change_outside_any_boundary_falls_back_to_full_layout() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);

    let (mut incremental, root, _) = sections(5, |i| format!("item {i}"));
    layout_document(&mut incremental, root, constraints(), &ctx).unwrap();
    // The root has no boundary above it, so changing its style forces a full pass.
    incremental.get_mut(root).unwrap().style.padding = tpt_layout::Edges::all(Dimension::px(4));
    relayout(&mut incremental, root, constraints(), &ctx).unwrap();

    let (mut full, root2, _) = sections(5, |i| format!("item {i}"));
    full.get_mut(root2).unwrap().style.padding = tpt_layout::Edges::all(Dimension::px(4));
    layout_document(&mut full, root2, constraints(), &ctx).unwrap();

    assert_eq!(incremental, full);
}

#[test]
fn relayout_with_nothing_dirty_is_a_no_op() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let (mut tree, root, _) = sections(3, |i| format!("item {i}"));
    let first = layout_document(&mut tree, root, constraints(), &ctx).unwrap();
    let before = tree.clone();
    let second = relayout(&mut tree, root, constraints(), &ctx).unwrap();
    assert_eq!(first, second);
    assert_eq!(tree, before);
}

#[test]
fn appended_child_inside_a_boundary_matches_full_layout() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);

    let (mut incremental, root, texts) = sections(4, |i| format!("item {i}"));
    layout_document(&mut incremental, root, constraints(), &ctx).unwrap();
    let section = incremental.parent(texts[2]).unwrap();
    text_child(&mut incremental, section, BoxStyle::default(), "extra line");
    relayout(&mut incremental, root, constraints(), &ctx).unwrap();

    let (mut full, root2, texts2) = sections(4, |i| format!("item {i}"));
    let section2 = full.parent(texts2[2]).unwrap();
    text_child(&mut full, section2, BoxStyle::default(), "extra line");
    layout_document(&mut full, root2, constraints(), &ctx).unwrap();

    assert_eq!(incremental, full);
}

#[test]
fn relayout_with_changed_constraints_does_full_pass() {
    use tpt_layout::{
        layout_document, relayout, BoxStyle, BoxTree, LayoutConstraints, LayoutContext, Px,
    };
    let m = tpt_layout::FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let narrow = LayoutConstraints {
        width: Px::from_px(200),
        height: None,
    };
    let wide = LayoutConstraints {
        width: Px::from_px(400),
        height: None,
    };
    let a = layout_document(&mut tree, root, narrow, &ctx).unwrap();
    let b = relayout(&mut tree, root, wide, &ctx).unwrap();
    assert_ne!(a.width, b.width);
    assert_eq!(b.width, Px::from_px(400));
}
