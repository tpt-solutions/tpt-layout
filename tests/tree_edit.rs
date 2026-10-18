//! Structural edits: slot reuse with generational ids, and the move API.

use tpt_layout::{
    layout_document, relayout, BoxStyle, BoxTree, Dimension, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, Px, TrackSize, Tracks, TreeError,
};

fn measurer() -> FixedAdvanceMeasurer {
    FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12))
}

fn constraints(width: i32) -> LayoutConstraints {
    LayoutConstraints {
        width: Px::from_px(width),
        height: None,
    }
}

#[test]
fn removed_id_stays_dead_after_slot_reuse() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let old = tree.insert(BoxStyle::default());
    tree.append_child(root, old).unwrap();
    tree.remove(old).unwrap();

    // The freed slot is reused by the next insert.
    let new = tree.insert(BoxStyle::default());
    assert_eq!(new.index(), old.index());
    assert_ne!(new, old);
    assert!(tree.get(old).is_none());
    assert!(tree.get(new).is_some());
    assert_eq!(tree.remove(old), Err(TreeError::NotFound(old)));
    assert!(tree.get_mut(old).is_none());
}

#[test]
fn slots_are_reused_across_many_edits() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    for _ in 0..1000 {
        let id = tree.insert(BoxStyle::default());
        tree.append_child(root, id).unwrap();
        tree.remove(id).unwrap();
    }
    // Root plus one reusable slot, not one slot per insert.
    assert!(tree.len() <= 2, "len was {}", tree.len());
}

#[test]
fn detach_keeps_subtree_and_reattaches() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let a = tree.insert(BoxStyle::default());
    let b = tree.insert(BoxStyle::default());
    tree.append_child(root, a).unwrap();
    tree.append_child(root, b).unwrap();

    tree.detach(a).unwrap();
    assert_eq!(tree.children(root), &[b]);
    assert!(tree.get(a).is_some(), "detach must not free the node");
    assert_eq!(tree.parent(a), None);

    tree.append_child(root, a).unwrap();
    assert_eq!(tree.children(root), &[b, a]);
}

#[test]
fn insert_before_and_insert_child_at_order_children() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let a = tree.insert(BoxStyle::default());
    let b = tree.insert(BoxStyle::default());
    let c = tree.insert(BoxStyle::default());
    tree.append_child(root, a).unwrap();
    tree.append_child(root, c).unwrap();

    tree.insert_before(c, b).unwrap();
    assert_eq!(tree.children(root), &[a, b, c]);

    let d = tree.insert(BoxStyle::default());
    tree.insert_child_at(root, 0, d).unwrap();
    assert_eq!(tree.children(root), &[d, a, b, c]);

    let e = tree.insert(BoxStyle::default());
    tree.insert_child_at(root, 99, e).unwrap();
    assert_eq!(tree.children(root), &[d, a, b, c, e]);
}

#[test]
fn insert_before_a_root_is_an_error() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let x = tree.insert(BoxStyle::default());
    assert_eq!(tree.insert_before(root, x), Err(TreeError::NotFound(root)));
}

#[test]
fn reparent_moves_subtree_and_rejects_cycles() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let left = tree.insert(BoxStyle::default());
    let right = tree.insert(BoxStyle::default());
    let inner = tree.insert(BoxStyle::default());
    tree.append_child(root, left).unwrap();
    tree.append_child(root, right).unwrap();
    tree.append_child(left, inner).unwrap();

    tree.reparent(inner, right).unwrap();
    assert_eq!(tree.children(left), &[]);
    assert_eq!(tree.children(right), &[inner]);
    assert_eq!(tree.parent(inner), Some(right));

    assert_eq!(tree.reparent(right, inner), Err(TreeError::WouldCycle));
}

#[test]
fn moved_node_relayouts_to_its_new_place() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let first = tree.insert(BoxStyle {
        height: Dimension::Length(Px::from_px(10)),
        ..BoxStyle::default()
    });
    let second = tree.insert(BoxStyle {
        height: Dimension::Length(Px::from_px(30)),
        ..BoxStyle::default()
    });
    tree.append_child(root, first).unwrap();
    tree.append_child(root, second).unwrap();
    layout_document(&mut tree, root, constraints(100), &ctx).unwrap();

    tree.detach(second).unwrap();
    tree.insert_child_at(root, 0, second).unwrap();
    let result = relayout(&mut tree, root, constraints(100), &ctx).unwrap();
    assert_eq!(result.height, Px::from_px(40));
    let second_y = tree.get(second).unwrap().layout.y;
    let first_y = tree.get(first).unwrap().layout.y;
    assert!(second_y < first_y, "second should now be above first");
}

#[test]
fn tracks_try_new_reports_overflow_instead_of_panicking() {
    let many = vec![TrackSize::Auto; 17];
    assert!(Tracks::try_new(&many).is_none());
    assert!(Tracks::try_new(&many[..16]).is_some());
}

#[test]
fn oversized_column_gap_clamps_instead_of_going_negative() {
    use tpt_layout::layout_document;
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        column_count: 5,
        column_gap: Dimension::Length(Px::from_px(500)),
        ..BoxStyle::default()
    });
    for _ in 0..4 {
        let child = tree.insert(BoxStyle {
            height: Dimension::Length(Px::from_px(20)),
            ..BoxStyle::default()
        });
        tree.append_child(root, child).unwrap();
    }
    layout_document(&mut tree, root, constraints(100), &ctx).unwrap();
    for child in tree.children(root).to_vec() {
        let w = tree.get(child).unwrap().layout.width;
        assert!(w >= 0.0, "negative width {w}");
    }
}

#[test]
fn remove_then_relayout_matches_full_layout() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let a = tree.insert(BoxStyle {
        height: Dimension::Length(Px::from_px(10)),
        ..BoxStyle::default()
    });
    let b = tree.insert(BoxStyle {
        height: Dimension::Length(Px::from_px(30)),
        ..BoxStyle::default()
    });
    tree.append_child(root, a).unwrap();
    tree.append_child(root, b).unwrap();
    layout_document(&mut tree, root, constraints(100), &ctx).unwrap();

    tree.remove(a).unwrap();
    let incremental = relayout(&mut tree, root, constraints(100), &ctx).unwrap();

    let mut fresh = BoxTree::new();
    let froot = fresh.insert(BoxStyle::default());
    let fb = fresh.insert(BoxStyle {
        height: Dimension::Length(Px::from_px(30)),
        ..BoxStyle::default()
    });
    fresh.append_child(froot, fb).unwrap();
    let full = layout_document(&mut fresh, froot, constraints(100), &ctx).unwrap();

    assert_eq!(incremental.height, full.height);
    assert_eq!(
        tree.get(b).unwrap().layout.y,
        fresh.get(fb).unwrap().layout.y
    );
}
