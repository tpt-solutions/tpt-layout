//! Arena box tree: insert, append, remove, and invariant checks.

use tpt_layout::{BoxStyle, BoxTree, TreeError};

#[test]
fn append_sets_parent_and_children() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let a = tree.insert(BoxStyle::default());
    let b = tree.insert(BoxStyle::default());
    tree.append_child(root, a).unwrap();
    tree.append_child(root, b).unwrap();

    assert_eq!(tree.children(root), &[a, b]);
    assert_eq!(tree.parent(a), Some(root));
    assert_eq!(tree.parent(root), None);
    assert_eq!(tree.len(), 3);
}

#[test]
fn child_cannot_have_two_parents() {
    let mut tree = BoxTree::new();
    let p1 = tree.insert(BoxStyle::default());
    let p2 = tree.insert(BoxStyle::default());
    let c = tree.insert(BoxStyle::default());
    tree.append_child(p1, c).unwrap();
    assert_eq!(
        tree.append_child(p2, c),
        Err(TreeError::AlreadyHasParent(c))
    );
}

#[test]
fn cycles_are_rejected() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let mid = tree.insert(BoxStyle::default());
    tree.append_child(root, mid).unwrap();
    // `root` is detached, but it is an ancestor of `mid`.
    assert_eq!(tree.append_child(mid, root), Err(TreeError::WouldCycle));

    // Two detached nodes: close a loop through a deeper descendant.
    let top = tree.insert(BoxStyle::default());
    let leaf = tree.insert(BoxStyle::default());
    tree.append_child(top, leaf).unwrap();
    assert_eq!(tree.append_child(leaf, top), Err(TreeError::WouldCycle));
}

#[test]
fn missing_nodes_are_reported() {
    let mut tree = BoxTree::new();
    let a = tree.insert(BoxStyle::default());
    // Ids are arena indices, so pick one past the end of `tree` to be certainly absent.
    let mut other = BoxTree::new();
    for _ in 0..4 {
        other.insert(BoxStyle::default());
    }
    let foreign = other.insert(BoxStyle::default());
    assert_eq!(
        tree.append_child(a, foreign),
        Err(TreeError::NotFound(foreign))
    );
    assert_eq!(tree.remove(foreign), Err(TreeError::NotFound(foreign)));
}

#[test]
fn remove_detaches_and_frees_the_subtree() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let mid = tree.insert(BoxStyle::default());
    let leaf = tree.insert_text(BoxStyle::default(), "x");
    tree.append_child(root, mid).unwrap();
    tree.append_child(mid, leaf).unwrap();

    tree.remove(mid).unwrap();
    assert!(tree.children(root).is_empty());
    assert!(tree.get(mid).is_none());
    assert!(tree.get(leaf).is_none());
    assert_eq!(tree.len(), 1);
}

#[test]
fn stale_ids_do_not_alias_new_nodes() {
    let mut tree = BoxTree::new();
    let old = tree.insert(BoxStyle::default());
    tree.remove(old).unwrap();
    let new = tree.insert(BoxStyle::default());
    assert_ne!(old, new);
    assert!(tree.get(old).is_none());
    assert!(tree.get(new).is_some());
}
