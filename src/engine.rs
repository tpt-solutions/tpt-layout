//! Layout orchestrator (TODO Phase 5): full layout, incremental re-layout, and the context that
//! the passes share.
//!
//! Incremental layout relies on *layout boundaries* (see `is_boundary` for the exact rules). A
//! boundary is a positioned box with explicit px width and height, px-only padding, border, insets
//! and min/max, a block-flow parent, and a formatting context or top padding or border. Its size
//! does not depend on its content, its containing block, or its parent, and its top margin cannot
//! escape it. So a change inside a boundary cannot move anything outside it, except through the
//! boundary's baseline, which is checked after each re-run.
//!
//! A dirty node is re-laid-out by re-running only its nearest enclosing boundary at the boundary's
//! existing position. Re-running a boundary is equivalent to the full pass, because the boundary's
//! inputs (position, width, height and style) are unchanged. The equality tests in
//! `tests/incremental.rs` and `tests/paint_and_robustness.rs` check this.
//!
//! The whole tree is laid out again when a dirty node has no boundary above it, when it was never
//! laid out, when it is itself a boundary (its own size or position may change), or when a
//! boundary's baseline moves.

use alloc::vec::Vec;

use crate::{
    flow::{layout_detached, layout_flow, relative_offset, Containing, Pass, Slot},
    px::Px,
    sizing::Sides,
    style::{Dimension, Display, Overflow, Position},
    text::TextMeasurer,
    tree::{BoxTree, NodeId, TreeError},
};

/// Shared inputs for a layout pass.
#[derive(Clone, Copy)]
pub struct LayoutContext<'a> {
    pub measurer: &'a dyn TextMeasurer,
    /// Pixels per inch. Used to convert point sizes with [`LayoutContext::points`].
    pub dpi: u32,
}

impl<'a> LayoutContext<'a> {
    pub fn new(measurer: &'a dyn TextMeasurer) -> Self {
        LayoutContext { measurer, dpi: 96 }
    }

    /// Converts a point size (hundredths of a point) at this context's DPI.
    pub fn points(&self, hundredths: i32) -> Px {
        Px::from_points(hundredths, self.dpi)
    }
}

/// The containing width and optional height the root is laid out in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutConstraints {
    pub width: Px,
    pub height: Option<Px>,
}

/// Summary of a layout pass. Per-box geometry is read from the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutResult {
    pub root: NodeId,
    /// Border-box width and height of the root.
    pub width: Px,
    pub height: Px,
}

/// Lays out the whole tree from scratch.
pub fn layout_document(
    tree: &mut BoxTree,
    root: NodeId,
    constraints: LayoutConstraints,
    ctx: &LayoutContext,
) -> Result<LayoutResult, TreeError> {
    layout_flow(tree, root, constraints.width, ctx.measurer)?;
    tree.clear_dirty();
    Ok(summary(tree, root))
}

/// Re-lays out only what changed since the last pass. Falls back to a full layout when the
/// changes cannot be confined to a layout boundary.
///
/// `constraints` must be the ones used for the last full pass. Changing them requires
/// [`layout_document`].
pub fn relayout(
    tree: &mut BoxTree,
    root: NodeId,
    constraints: LayoutConstraints,
    ctx: &LayoutContext,
) -> Result<LayoutResult, TreeError> {
    if tree.get(root).is_none() {
        return Err(TreeError::NotFound(root));
    }
    let nodes = tree.subtree(root);
    let dirty: Vec<NodeId> = nodes
        .iter()
        .copied()
        .filter(|id| tree.is_dirty(*id))
        .collect();
    if dirty.is_empty() {
        return Ok(summary(tree, root));
    }

    let mut boundaries: Vec<NodeId> = Vec::new();
    for id in dirty {
        // A changed boundary may change its own size or position, which moves its siblings, so it
        // needs the full pass. Changes strictly inside a boundary are confined to it.
        if id == root || !is_laid_out(tree, id) || is_boundary(tree, id) {
            return layout_document(tree, root, constraints, ctx);
        }
        let Some(parent) = tree.parent(id) else {
            return layout_document(tree, root, constraints, ctx);
        };
        match nearest_boundary(tree, root, parent) {
            Some(boundary) => {
                if !boundaries.contains(&boundary) {
                    boundaries.push(boundary);
                }
            }
            None => return layout_document(tree, root, constraints, ctx),
        }
    }

    // Keep only the outermost boundaries: an inner one is covered by its ancestor.
    let outermost: Vec<NodeId> = boundaries
        .iter()
        .copied()
        .filter(|b| {
            !boundaries
                .iter()
                .any(|other| other != b && is_ancestor(tree, *other, *b))
        })
        .collect();

    let pass = Pass::new(ctx.measurer, root);
    for boundary in outermost {
        if !relayout_boundary(&pass, tree, boundary) {
            return layout_document(tree, root, constraints, ctx);
        }
        if pass.too_deep() {
            return Err(TreeError::TooDeep);
        }
    }
    tree.clear_dirty();
    Ok(summary(tree, root))
}

fn summary(tree: &BoxTree, root: NodeId) -> LayoutResult {
    let frame = tree.get(root).map(|n| n.layout);
    let (width, height) = frame.map_or((Px::ZERO, Px::ZERO), |l| {
        (Px::from_f32(l.width), Px::from_f32(l.height))
    });
    LayoutResult {
        root,
        width,
        height,
    }
}

fn is_laid_out(tree: &BoxTree, id: NodeId) -> bool {
    tree.get(id).is_some_and(|n| n.laid_out)
}

fn is_ancestor(tree: &BoxTree, ancestor: NodeId, node: NodeId) -> bool {
    let mut current = tree.parent(node);
    while let Some(id) = current {
        if id == ancestor {
            return true;
        }
        current = tree.parent(id);
    }
    false
}

/// The nearest boundary at or above `start`, excluding the root (which is always re-laid-out).
fn nearest_boundary(tree: &BoxTree, root: NodeId, start: NodeId) -> Option<NodeId> {
    let mut current = Some(start);
    while let Some(node) = current {
        if node == root {
            return None;
        }
        if is_boundary(tree, node) {
            return Some(node);
        }
        current = tree.parent(node);
    }
    None
}

/// Whether `id` is a layout boundary (see the module docs).
pub(crate) fn is_boundary(tree: &BoxTree, id: NodeId) -> bool {
    let Some(node) = tree.get(id) else {
        return false;
    };
    let style = &node.style;
    // A vertical subtree is converted for layout only inside its root, so it cannot be re-run alone.
    if crate::writing::in_vertical_subtree(tree, id) {
        return false;
    }
    // A flex, grid or table container can resize its items (grow, shrink, stretch, track sizing),
    // so an item's size depends on its parent. Only children of block-flow containers keep their
    // own size.
    let parent_is_block_flow = tree.parent(id).and_then(|p| tree.get(p)).is_some_and(|p| {
        matches!(
            p.style.display,
            Display::Block | Display::Inline | Display::InlineBlock
        )
    });
    if !parent_is_block_flow {
        return false;
    }
    let px_only = |d: &Dimension| !matches!(d, Dimension::Percent(_));
    // A boundary's first child must not collapse its top margin through the boundary, because a
    // detached re-layout does not collapse margins. So the boundary establishes a formatting
    // context, or has top padding or border to separate it.
    let establishes_context = style.overflow != Overflow::Visible
        || style.position == Position::Absolute
        || matches!(
            style.display,
            Display::Flex
                | Display::Grid
                | Display::Table
                | Display::TableCell
                | Display::InlineBlock
        );
    let separated_top = matches!(style.padding.top, Dimension::Length(v) if !v.is_zero())
        || !style.border.top.is_zero();
    // An inline element has no box of its own: its children are placed in the enclosing line.
    style.display != Display::Inline
        && (establishes_context || separated_top)
        && style.position != Position::Static
        && matches!(style.width, Dimension::Length(_))
        && matches!(style.height, Dimension::Length(_))
        && [
            style.min_width,
            style.max_width,
            style.min_height,
            style.max_height,
        ]
        .iter()
        .all(px_only)
        && [
            style.padding.top,
            style.padding.right,
            style.padding.bottom,
            style.padding.left,
        ]
        .iter()
        .all(px_only)
        && [
            style.inset.top,
            style.inset.right,
            style.inset.bottom,
            style.inset.left,
        ]
        .iter()
        .all(px_only)
}

/// Re-runs one boundary at its current position. Its frame's width and position are reused, so
/// its border height is also unchanged.
///
/// Returns `false` if the boundary's baseline moved. A baseline is used by ancestors (for
/// baseline alignment and inline lines), so such a change can move siblings and the caller must
/// run a full pass.
fn relayout_boundary(pass: &Pass, tree: &mut BoxTree, boundary: NodeId) -> bool {
    let Some(node) = tree.get(boundary) else {
        return true;
    };
    let style = node.style;
    let Some(frame) = tree.frame(boundary) else {
        return true;
    };
    // Boundaries have px-only edges, so the containing block size does not matter here.
    let sides = Sides::of(&style, Px::ZERO);
    let (dx, dy) = relative_offset(
        &style,
        Containing {
            x: Px::ZERO,
            width: Px::ZERO,
            height: None,
        },
    );
    let slot = Slot {
        x: frame.x - dx,
        y: frame.y - dy,
        content_width: frame.width - sides.horizontal_extra(),
        cb_width: Px::ZERO,
        cb_height: None,
        height: None,
    };
    let mut scratch = Vec::new();
    let before = frame.baseline;
    let _ = layout_detached(pass, tree, boundary, slot, &mut scratch);
    tree.frame(boundary)
        .is_some_and(|after| after.baseline == before)
}
