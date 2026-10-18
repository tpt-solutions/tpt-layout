//! Paint metadata (TODO Phase 5): paint order and clip regions. tpt-layout still does not draw.
//!
//! [`paint_order`] lists every laid-out box in the order a renderer should paint it:
//! - by `z-index`, lowest first (boxes without one count as `0`), and
//! - in document order within the same `z-index`.
//!
//! Each item also carries its clip region: the intersection of the border boxes of its ancestors
//! that have `overflow` other than `visible`. Stacking contexts are not modelled. A positioned box
//! with a `z-index` is sorted against every other box in the same tree.

use alloc::vec::Vec;

use crate::{
    math::Rect,
    px::Px,
    style::Overflow,
    tree::{BoxTree, NodeId},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintItem {
    pub node: NodeId,
    pub z_index: i32,
    /// Region the box is clipped to, or `None` when no ancestor clips it.
    pub clip: Option<Rect>,
}

/// Paint items for the laid-out tree under `root`, in paint order.
pub fn paint_order(tree: &BoxTree, root: NodeId) -> Vec<PaintItem> {
    let mut items = Vec::new();
    collect(tree, root, None, &mut items);
    // Stable sort: equal z-index keeps document order.
    items.sort_by_key(|item| item.z_index);
    items
}

fn collect(tree: &BoxTree, id: NodeId, clip: Option<Rect>, out: &mut Vec<PaintItem>) {
    let Some(node) = tree.get(id) else { return };
    out.push(PaintItem {
        node: id,
        z_index: node.style.z_index.unwrap_or(0),
        clip,
    });
    let child_clip = if node.style.overflow != Overflow::Visible {
        let own = border_box(node.layout);
        Some(match clip {
            Some(outer) => intersect(outer, own),
            None => own,
        })
    } else {
        clip
    };
    for child in tree.children(id).to_vec() {
        collect(tree, child, child_clip, out);
    }
}

fn border_box(layout: crate::layout::ComputedLayout) -> Rect {
    Rect {
        x: Px::from_f32(layout.x),
        y: Px::from_f32(layout.y),
        width: Px::from_f32(layout.width),
        height: Px::from_f32(layout.height),
    }
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    Rect {
        x: left,
        y: top,
        width: (right - left).max(Px::ZERO),
        height: (bottom - top).max(Px::ZERO),
    }
}
