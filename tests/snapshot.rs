//! Layout fingerprints and geometry diffs.

use tpt_layout::{
    layout_document, relayout, snapshot, BoxStyle, BoxTree, Dimension, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Px,
};

fn measurer() -> FixedAdvanceMeasurer {
    FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12))
}

fn constraints() -> LayoutConstraints {
    LayoutConstraints {
        width: Px::from_px(200),
        height: None,
    }
}

/// A root with three fixed-height children. Returns the tree, the root, and the middle child.
fn build(middle_height: i32) -> (BoxTree, NodeId, NodeId) {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let mut middle = None;
    for h in [10, middle_height, 30] {
        let child = tree.insert(BoxStyle {
            height: Dimension::Length(Px::from_px(h)),
            ..BoxStyle::default()
        });
        tree.append_child(root, child).unwrap();
        if h == middle_height {
            middle = Some(child);
        }
    }
    (tree, root, middle.unwrap())
}

#[test]
fn fingerprint_is_stable_for_equal_layouts() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let (mut a, root_a, _) = build(20);
    let (mut b, root_b, _) = build(20);
    layout_document(&mut a, root_a, constraints(), &ctx).unwrap();
    layout_document(&mut b, root_b, constraints(), &ctx).unwrap();
    assert_eq!(
        snapshot::fingerprint(&a, root_a),
        snapshot::fingerprint(&b, root_b)
    );
}

#[test]
fn fingerprint_changes_when_geometry_changes() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let (mut a, root_a, _) = build(20);
    let (mut b, root_b, _) = build(25);
    layout_document(&mut a, root_a, constraints(), &ctx).unwrap();
    layout_document(&mut b, root_b, constraints(), &ctx).unwrap();
    assert_ne!(
        snapshot::fingerprint(&a, root_a),
        snapshot::fingerprint(&b, root_b)
    );
}

#[test]
fn diff_of_unchanged_tree_is_empty() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let (mut tree, root, _) = build(20);
    layout_document(&mut tree, root, constraints(), &ctx).unwrap();
    let snapshot_before = tree.clone();
    assert!(snapshot::diff(&snapshot_before, &tree, root).is_empty());
}

#[test]
fn diff_reports_only_the_nodes_that_moved_or_resized() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let (mut tree, root, middle) = build(20);
    layout_document(&mut tree, root, constraints(), &ctx).unwrap();
    let before = tree.clone();

    tree.get_mut(middle).unwrap().style.height = Dimension::Length(Px::from_px(40));
    relayout(&mut tree, root, constraints(), &ctx).unwrap();

    let changes = snapshot::diff(&before, &tree, root);
    let changed: Vec<NodeId> = changes.iter().map(|c| c.node).collect();
    assert!(changed.contains(&middle), "resized child must be reported");
    // The third child moves down because the middle one grew, so it is reported too.
    let last = *tree.children(root).last().unwrap();
    assert!(
        changed.contains(&last),
        "following sibling moved, must be reported"
    );
    // The first child is above the change and must not be reported.
    let first = tree.children(root)[0];
    assert!(!changed.contains(&first));
}

#[test]
fn diff_marks_added_nodes_with_no_before_geometry() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let (mut tree, root, _) = build(20);
    layout_document(&mut tree, root, constraints(), &ctx).unwrap();
    let before = tree.clone();

    let extra = tree.insert(BoxStyle {
        height: Dimension::Length(Px::from_px(5)),
        ..BoxStyle::default()
    });
    tree.append_child(root, extra).unwrap();
    relayout(&mut tree, root, constraints(), &ctx).unwrap();

    let change = snapshot::diff(&before, &tree, root)
        .into_iter()
        .find(|c| c.node == extra)
        .expect("added node must be reported");
    assert!(change.before.is_none());
    assert!(change.after.is_some());
}
