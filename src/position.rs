//! Absolute positioning. Absolutely positioned boxes leave flow and are placed against their
//! containing block once its size is known. Relative offsets are applied by the detached and
//! in-flow layout in `flow.rs`.
//!
//! Simplifications:
//! - An auto width fills the space between the insets, or shrinks to fit when both insets are
//!   auto (it uses the static position as its left edge).
//! - An auto-inset position falls back to the box's static position: the content origin of its
//!   parent, at the point in flow where it appeared.

use alloc::vec::Vec;

use crate::{
    flow::{layout_detached, Pass, Slot},
    math::Rect,
    px::Px,
    sizing::{clamp_inline, to_content, Sides},
    tree::BoxTree,
    tree::NodeId,
};

/// An absolutely positioned box waiting for its containing block.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Deferred {
    pub id: NodeId,
    /// Where the box would sit if it were in flow.
    pub static_x: Px,
    pub static_y: Px,
}

pub(crate) fn layout_deferred(
    pass: &Pass,
    tree: &mut BoxTree,
    list: Vec<Deferred>,
    containing: Rect,
) {
    for item in list {
        layout_absolute(pass, tree, item, containing);
    }
}

fn layout_absolute(pass: &Pass, tree: &mut BoxTree, item: Deferred, cb: Rect) {
    let Some(node) = tree.get(item.id) else {
        return;
    };
    let style = node.style;
    let sides = Sides::of(&style, cb.width);
    let extra_h = sides.horizontal_extra();
    let left = style.inset.left.resolve(Some(cb.width));
    let right = style.inset.right.resolve(Some(cb.width));
    let top = style.inset.top.resolve(Some(cb.height));
    let bottom = style.inset.bottom.resolve(Some(cb.height));

    let content_width = match style.width.resolve(Some(cb.width)) {
        Some(width) => clamp_inline(
            &style,
            cb.width,
            extra_h,
            to_content(&style, width, extra_h),
        ),
        None => {
            let left_edge = left.map_or(item.static_x, |l| cb.x + l);
            let right_edge = right.map_or(cb.x + cb.width, |r| cb.x + cb.width - r);
            let available =
                (right_edge - left_edge - extra_h - sides.margin.horizontal()).max(Px::ZERO);
            clamp_inline(&style, cb.width, extra_h, available)
        }
    };
    let border_width = content_width + extra_h;

    let x = match (left, right) {
        (Some(l), _) => cb.x + l + sides.margin.left,
        (None, Some(r)) => cb.x + cb.width - r - sides.margin.right - border_width,
        (None, None) => item.static_x + sides.margin.left,
    };
    // Provisional top. If only `bottom` is set, the final position depends on the height, so the
    // box is laid out here and then moved.
    let provisional_top = match top {
        Some(t) => cb.y + t + sides.margin.top,
        None => item.static_y + sides.margin.top,
    };

    let mut scratch = Vec::new();
    let Some(placed) = layout_detached(
        pass,
        tree,
        item.id,
        Slot {
            x,
            y: provisional_top,
            content_width,
            cb_width: cb.width,
            cb_height: Some(cb.height),
            height: None,
        },
        &mut scratch,
    ) else {
        return;
    };

    let y = match (top, bottom) {
        (None, Some(b)) => cb.y + cb.height - b - sides.margin.bottom - placed.border_height,
        _ => provisional_top,
    };
    let dy = y - provisional_top;
    if !dy.is_zero() {
        tree.shift_subtree(item.id, Px::ZERO, dy);
    }
}
