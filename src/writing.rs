//! Writing modes.
//!
//! A box's effective mode is its own mode, or its parent's when its mode is `Inherit`; the top of a
//! tree is `HorizontalTb`. A box whose effective mode differs from its caller's is a *root* of
//! that mode. Its subtree is laid out in the root's logical space, where the inline axis is the
//! root's inline direction and the block axis its block direction:
//!
//! 1. Every style the root owns is converted to logical terms (width and height swapped, and the
//!    edges re-mapped). A nested root's own style is first converted back to physical terms, then
//!    to its own logical terms. Descendants of a nested root are left for that root to convert.
//! 2. The root is laid out at the logical origin with the ordinary passes.
//! 3. Each frame, fragment and deferred static position in the subtree is mapped into the caller's
//!    frame. The mapping is a signed 2x2 matrix (the root's logical-to-physical rotation, composed
//!    with the inverse of the caller's), followed by a shift that lands the root's border box on
//!    its origin. A nested root's frames are therefore mapped once by each root that encloses them.
//! 4. The styles of the members are restored.
//!
//! Limits, documented in TODO.md:
//! - A nested root has no containing block from its caller, so percentage sizes of a nested root
//!   resolve against nothing, and its sizes come from its own style. Flex and grid items that are
//!   nested roots do not stretch.
//! - Intrinsic sizes (`intrinsic::max_content`) of a vertical box are computed as if it were
//!   horizontal, so shrink-to-fit placement of a vertical box in a horizontal context is only
//!   approximate.
//! - Lines of a vertical paragraph are not split for pagination across columns.

use alloc::vec::Vec;

use crate::{
    flow::{layout_detached_inner, Pass, Placed, Slot},
    intrinsic,
    math::{Edges, Rect},
    position::Deferred,
    px::Px,
    sizing::{inline_size, Sides},
    style::{BoxStyle, WritingMode},
    tree::{BoxTree, Frame, NodeId},
};

/// Stands in for "unbounded" when a containing size is not known.
const UNBOUNDED: Px = Px::MAX;

/// Largest natural inline size of a vertical box whose physical containing height is unknown. Keeps
/// percentages of an unknown size from growing without bound.
const UNBOUNDED_INLINE_LIMIT: Px = Px(65536 * crate::px::PX_SCALE);

/// The effective writing mode of `id`: its own mode, or the nearest ancestor's that is not
/// `Inherit`. `HorizontalTb` when no ancestor sets one.
pub(crate) fn effective_mode(tree: &BoxTree, id: NodeId) -> WritingMode {
    let mut current = Some(id);
    while let Some(node) = current {
        let Some(n) = tree.get(node) else { break };
        if n.style.writing_mode != WritingMode::Inherit {
            return n.style.writing_mode;
        }
        current = tree.parent(node);
    }
    WritingMode::HorizontalTb
}

/// The mode `id` is laid out in: its parent's effective mode, or `HorizontalTb` at the top.
fn caller_mode(tree: &BoxTree, id: NodeId) -> WritingMode {
    tree.parent(id)
        .map_or(WritingMode::HorizontalTb, |p| effective_mode(tree, p))
}

/// The mode of `id` if it is a root, i.e. its effective mode differs from its caller's.
pub(crate) fn root_mode(tree: &BoxTree, id: NodeId) -> Option<WritingMode> {
    let mode = effective_mode(tree, id);
    (mode != caller_mode(tree, id)).then_some(mode)
}

/// Whether `id` is laid out in a non-horizontal writing mode, itself or through an ancestor.
pub(crate) fn in_vertical_subtree(tree: &BoxTree, id: NodeId) -> bool {
    let mut current = Some(id);
    while let Some(node) = current {
        if effective_mode(tree, node).is_vertical() {
            return true;
        }
        current = tree.parent(node);
    }
    false
}

/// Whether `id`'s caller lays it out in physical (page) terms, which is the case for every root
/// whose caller is horizontal.
pub(crate) fn caller_is_physical(tree: &BoxTree, id: NodeId) -> bool {
    caller_mode(tree, id) == WritingMode::HorizontalTb
}

/// Re-maps edge values from physical to logical order for `mode`.
fn logical_edges<T: Copy>(edges: Edges<T>, mode: WritingMode) -> Edges<T> {
    match mode {
        WritingMode::VerticalRl | WritingMode::SidewaysRl => Edges {
            top: edges.right,
            right: edges.bottom,
            bottom: edges.left,
            left: edges.top,
        },
        WritingMode::VerticalLr => Edges {
            top: edges.left,
            right: edges.bottom,
            bottom: edges.right,
            left: edges.top,
        },
        // Lines run bottom to top, so the inline start is the physical bottom edge.
        WritingMode::SidewaysLr => Edges {
            top: edges.left,
            right: edges.top,
            bottom: edges.right,
            left: edges.bottom,
        },
        WritingMode::Inherit | WritingMode::HorizontalTb => edges,
    }
}

/// Inverse of `logical_edges`.
fn physical_edges<T: Copy>(edges: Edges<T>, mode: WritingMode) -> Edges<T> {
    match mode {
        WritingMode::VerticalRl | WritingMode::SidewaysRl => Edges {
            top: edges.left,
            right: edges.top,
            bottom: edges.right,
            left: edges.bottom,
        },
        // This mapping is its own inverse.
        WritingMode::VerticalLr => Edges {
            top: edges.left,
            right: edges.bottom,
            bottom: edges.right,
            left: edges.top,
        },
        WritingMode::SidewaysLr => Edges {
            top: edges.right,
            right: edges.bottom,
            bottom: edges.left,
            left: edges.top,
        },
        WritingMode::Inherit | WritingMode::HorizontalTb => edges,
    }
}

fn is_rotated(mode: WritingMode) -> bool {
    mode.is_vertical()
}

/// Converts a physical style to logical terms for `mode`.
fn to_logical(style: &BoxStyle, mode: WritingMode) -> BoxStyle {
    if !is_rotated(mode) {
        return *style;
    }
    BoxStyle {
        width: style.height,
        height: style.width,
        min_width: style.min_height,
        min_height: style.min_width,
        max_width: style.max_height,
        max_height: style.max_width,
        margin: logical_edges(style.margin, mode),
        padding: logical_edges(style.padding, mode),
        border: logical_edges(style.border, mode),
        inset: logical_edges(style.inset, mode),
        ..*style
    }
}

/// Converts a style in `mode`'s logical terms back to physical terms. Inverse of `to_logical`.
fn to_physical(style: &BoxStyle, mode: WritingMode) -> BoxStyle {
    if !is_rotated(mode) {
        return *style;
    }
    BoxStyle {
        width: style.height,
        height: style.width,
        min_width: style.min_height,
        min_height: style.min_width,
        max_width: style.max_height,
        max_height: style.max_width,
        margin: physical_edges(style.margin, mode),
        padding: physical_edges(style.padding, mode),
        border: physical_edges(style.border, mode),
        inset: physical_edges(style.inset, mode),
        ..*style
    }
}

/// A 2x2 matrix of signed unit entries: a signed permutation of the axes.
type Matrix = [[i32; 2]; 2];

/// Maps a logical vector of `mode` to a physical (page) vector.
fn to_page(mode: WritingMode) -> Matrix {
    match mode {
        WritingMode::VerticalRl | WritingMode::SidewaysRl => [[0, -1], [1, 0]],
        WritingMode::VerticalLr => [[0, 1], [1, 0]],
        WritingMode::SidewaysLr => [[0, 1], [-1, 0]],
        WritingMode::Inherit | WritingMode::HorizontalTb => [[1, 0], [0, 1]],
    }
}

fn mul(a: Matrix, b: Matrix) -> Matrix {
    [
        [
            a[0][0] * b[0][0] + a[0][1] * b[1][0],
            a[0][0] * b[0][1] + a[0][1] * b[1][1],
        ],
        [
            a[1][0] * b[0][0] + a[1][1] * b[1][0],
            a[1][0] * b[0][1] + a[1][1] * b[1][1],
        ],
    ]
}

/// The inverse of a signed permutation matrix, which is its transpose.
fn transpose(m: Matrix) -> Matrix {
    [[m[0][0], m[1][0]], [m[0][1], m[1][1]]]
}

/// A signed-permutation map followed by a shift.
#[derive(Clone, Copy)]
struct Affine {
    m: Matrix,
    shift: (Px, Px),
}

impl Affine {
    fn point(&self, (x, y): (Px, Px)) -> (Px, Px) {
        let m = self.m;
        (
            x * m[0][0] + y * m[0][1] + self.shift.0,
            x * m[1][0] + y * m[1][1] + self.shift.1,
        )
    }

    fn rect(&self, rect: Rect) -> Rect {
        let a = self.point((rect.x, rect.y));
        let b = self.point((rect.x + rect.width, rect.y + rect.height));
        let (x0, x1) = if a.0 <= b.0 { (a.0, b.0) } else { (b.0, a.0) };
        let (y0, y1) = if a.1 <= b.1 { (a.1, b.1) } else { (b.1, a.1) };
        Rect {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        }
    }

    /// Places a root whose logical border box is `logical_box` so that the mapped box starts at
    /// `target`.
    fn fit(m: Matrix, logical_box: Rect, target: (Px, Px)) -> Affine {
        let raw = Affine {
            m,
            shift: (Px::ZERO, Px::ZERO),
        }
        .rect(logical_box);
        Affine {
            m,
            shift: (target.0 - raw.x, target.1 - raw.y),
        }
    }
}

/// `id` and every descendant whose nearest root is `id`: the nested roots below it are included,
/// but not their descendants.
fn root_members(tree: &BoxTree, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack = alloc::vec![id];
    while let Some(node) = stack.pop() {
        out.push(node);
        for child in tree.laid_children(node).iter().rev() {
            if root_mode(tree, *child).is_none() {
                stack.push(*child);
            } else {
                out.push(*child);
            }
        }
    }
    out
}

/// Where a root goes and how big it is, in its caller's frame.
#[derive(Clone, Copy)]
pub(crate) struct VerticalGeometry {
    /// Border position in the caller's frame.
    pub origin: (Px, Px),
    /// Content width of a root whose caller is physical; this is the root's logical block size.
    pub content_block: Option<Px>,
    /// Content height of a root whose caller is physical; this is the root's logical inline size.
    pub content_inline: Option<Px>,
    /// Containing block of a root whose caller is physical: width, and height when definite.
    pub cb: Option<(Px, Option<Px>)>,
}

/// Lays out root `id` of writing mode `mode` in its own logical space, and maps it into its
/// caller's frame at `geometry.origin`. Returns the root's size in the caller's frame.
pub(crate) fn layout_vertical(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    mode: WritingMode,
    geometry: VerticalGeometry,
    deferred: &mut Vec<Deferred>,
) -> Placed {
    let VerticalGeometry {
        origin,
        content_block,
        content_inline,
        cb,
    } = geometry;
    let caller = caller_mode(tree, id);

    // The root's own style is in its caller's logical terms, and so is each member's. Descendants
    // of a nested root are still physical, and are converted here for the first time.
    let members = root_members(tree, id);
    let saved: Vec<(NodeId, BoxStyle)> = members
        .iter()
        .filter_map(|n| tree.get(*n).map(|node| (*n, node.style)))
        .collect();
    for (node, style) in &saved {
        let physical = if *node == id {
            to_physical(style, caller)
        } else {
            *style
        };
        tree.set_style_silently(*node, to_logical(&physical, mode));
    }

    let logical_cb_width = cb.and_then(|c| c.1).unwrap_or(UNBOUNDED);
    let logical_cb_height = cb.map(|c| c.0);
    let converted = tree.get(id).map(|n| n.style).unwrap_or_default();
    let inline = match content_inline {
        Some(inline) => inline,
        None => match converted.width.resolve(cb.and_then(|c| c.1)) {
            Some(_) => {
                let sides = Sides::of(&converted, logical_cb_width);
                inline_size(&converted, &sides, logical_cb_width).0
            }
            None => {
                let natural = intrinsic::shrink_to_fit(pass, tree, id, logical_cb_width);
                if cb.and_then(|c| c.1).is_none() {
                    natural.min(UNBOUNDED_INLINE_LIMIT)
                } else {
                    natural
                }
            }
        },
    };

    let mut scratch = Vec::new();
    let _ = layout_detached_inner(
        pass,
        tree,
        id,
        Slot {
            x: Px::ZERO,
            y: Px::ZERO,
            content_width: inline,
            cb_width: logical_cb_width,
            cb_height: logical_cb_height,
            height: content_block,
        },
        &mut scratch,
    );

    let logical = tree.frame(id).unwrap_or_default();
    let logical_box = Rect {
        x: logical.x,
        y: logical.y,
        width: logical.width,
        height: logical.height,
    };
    // Logical space to the caller's frame: this root's rotation, relative to the caller's own.
    let m = mul(transpose(to_page(caller)), to_page(mode));
    let affine = Affine::fit(m, logical_box, origin);

    for node in tree.subtree(id) {
        if let Some(frame) = tree.frame(node) {
            let mapped = affine.rect(Rect {
                x: frame.x,
                y: frame.y,
                width: frame.width,
                height: frame.height,
            });
            tree.set_frame(
                node,
                Frame {
                    x: mapped.x,
                    y: mapped.y,
                    width: mapped.width,
                    height: mapped.height,
                    baseline: frame.baseline,
                },
            );
        }
        if let Some(fragments) = tree.fragments_mut(node) {
            for fragment in fragments.iter_mut() {
                let mapped = affine.rect(Rect {
                    x: fragment.x,
                    y: fragment.y,
                    width: fragment.width,
                    height: fragment.height,
                });
                fragment.x = mapped.x;
                fragment.y = mapped.y;
                fragment.width = mapped.width;
                fragment.height = mapped.height;
            }
        }
    }

    // Deferred (absolutely positioned) boxes move with their static positions.
    for entry in scratch {
        let (x, y) = affine.point((entry.static_x, entry.static_y));
        deferred.push(Deferred {
            id: entry.id,
            static_x: x,
            static_y: y,
        });
    }

    // Each member goes back to the style it had on entry.
    for (node, style) in saved {
        tree.set_style_silently(node, style);
    }

    let placed = affine.rect(logical_box);
    Placed {
        border_height: placed.height,
        baseline: Px::ZERO,
    }
}
