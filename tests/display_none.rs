//! `display: none`: no space, no paint, excluded from layout, and toggling it stays incremental.

use tpt_layout::{
    layout_document, paint_order, relayout, BoxStyle, BoxTree, Dimension, Display,
    FixedAdvanceMeasurer, LayoutConstraints, LayoutContext, NodeId, Px,
};

fn measurer() -> FixedAdvanceMeasurer {
    FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12))
}

fn block(height: i32, display: Display) -> BoxStyle {
    BoxStyle {
        height: Dimension::Length(Px::from_px(height)),
        display,
        ..BoxStyle::default()
    }
}

fn constraints() -> LayoutConstraints {
    LayoutConstraints {
        width: Px::from_px(100),
        height: None,
    }
}

/// Root with children A (10 px), B (50 px, possibly hidden) and C (30 px).
fn build(middle: Display) -> (BoxTree, NodeId, NodeId) {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let a = tree.insert(block(10, Display::Block));
    let b = tree.insert(block(50, middle));
    let c = tree.insert(block(30, Display::Block));
    for id in [a, b, c] {
        tree.append_child(root, id).unwrap();
    }
    (tree, root, b)
}

#[test]
fn hidden_child_takes_no_space() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let (mut tree, root, _) = build(Display::None);
    let result = layout_document(&mut tree, root, constraints(), &ctx).unwrap();
    assert_eq!(result.height, Px::from_px(40));
}

#[test]
fn hidden_child_is_not_painted() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let (mut tree, root, hidden) = build(Display::None);
    layout_document(&mut tree, root, constraints(), &ctx).unwrap();
    let painted: Vec<NodeId> = paint_order(&tree, root).iter().map(|i| i.node).collect();
    assert!(!painted.contains(&hidden));
    assert_eq!(painted.len(), 3, "root plus two visible children");
}

#[test]
fn public_children_still_lists_hidden_child() {
    let (tree, root, hidden) = build(Display::None);
    assert!(tree.children(root).contains(&hidden));
}

#[test]
fn showing_a_hidden_child_relayouts_to_the_full_result() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let (mut tree, root, hidden) = build(Display::None);
    layout_document(&mut tree, root, constraints(), &ctx).unwrap();

    tree.get_mut(hidden).unwrap().style.display = Display::Block;
    let incremental = relayout(&mut tree, root, constraints(), &ctx).unwrap();

    let (mut fresh, froot, _) = build(Display::Block);
    let full = layout_document(&mut fresh, froot, constraints(), &ctx).unwrap();
    assert_eq!(incremental.height, full.height);
    assert_eq!(incremental.height, Px::from_px(90));
}

#[test]
fn hiding_a_child_relayouts_to_the_full_result() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let (mut tree, root, hidden) = build(Display::Block);
    layout_document(&mut tree, root, constraints(), &ctx).unwrap();

    tree.get_mut(hidden).unwrap().style.display = Display::None;
    let incremental = relayout(&mut tree, root, constraints(), &ctx).unwrap();
    assert_eq!(incremental.height, Px::from_px(40));
}
