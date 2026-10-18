//! Layout fingerprints and geometry diffs.
//!
//! Both work on the fixed-point frames the engine computes, so their results do not depend on
//! floating-point output and are the same on every platform.
//!
//! A snapshot is just a clone of the [`BoxTree`] taken after a layout pass. Compare it with the
//! tree after an edit and a relayout with [`diff`].

use crate::{
    px::Px,
    tree::{BoxTree, NodeId},
};

/// Border-box geometry of one node, in fixed-point pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub x: Px,
    pub y: Px,
    pub width: Px,
    pub height: Px,
}

/// One node whose geometry differs between two trees. `None` means the node is absent on that
/// side (added or removed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change {
    pub node: NodeId,
    pub before: Option<Geometry>,
    pub after: Option<Geometry>,
}

fn geometry(tree: &BoxTree, id: NodeId) -> Option<Geometry> {
    let frame = tree.frame(id)?;
    Some(Geometry {
        x: frame.x,
        y: frame.y,
        width: frame.width,
        height: frame.height,
    })
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn mix(hash: &mut u64, value: i32) {
    for byte in value.to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

/// A 64-bit hash of the geometry of every node under `root`, in document order. Equal layouts
/// give equal fingerprints on every platform, so this works as a regression value.
pub fn fingerprint(tree: &BoxTree, root: NodeId) -> u64 {
    let mut hash = FNV_OFFSET;
    for id in tree.subtree_ids(root) {
        mix(&mut hash, id.index() as i32);
        if let Some(g) = geometry(tree, id) {
            mix(&mut hash, g.x.0);
            mix(&mut hash, g.y.0);
            mix(&mut hash, g.width.0);
            mix(&mut hash, g.height.0);
        }
    }
    hash
}

/// Nodes under `root` whose geometry differs between `before` and `after`, in `before`'s document
/// order, followed by nodes that only exist in `after`. Identity is by [`NodeId`], so compare a
/// snapshot with the tree it was taken from.
pub fn diff(before: &BoxTree, after: &BoxTree, root: NodeId) -> alloc::vec::Vec<Change> {
    let mut changes = alloc::vec::Vec::new();
    let before_ids = before.subtree_ids(root);
    for id in &before_ids {
        let b = geometry(before, *id);
        let a = geometry(after, *id);
        if b != a {
            changes.push(Change {
                node: *id,
                before: b,
                after: a,
            });
        }
    }
    for id in after.subtree_ids(root) {
        if !before_ids.contains(&id) {
            changes.push(Change {
                node: id,
                before: None,
                after: geometry(after, id),
            });
        }
    }
    changes
}
