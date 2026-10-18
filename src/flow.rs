//! Block and inline flow with CSS margin collapsing. Entry point: [`layout_flow`].
//!
//! Layout is single-pass and top-down. Collapsed margins are decided by looking ahead through
//! styles only (`outer_top_chain`, `collapses_*`), so every box's position is known before its
//! content is placed and nothing is re-laid-out.
//!
//! Margins collapse:
//! - between adjacent block siblings (positive: max; negative: min; mixed: max positive plus min
//!   negative),
//! - through an empty block (no content, no vertical padding or border, height auto or 0), and
//! - between a parent and its first/last in-flow block child when neither has vertical padding or
//!   border and the parent is not an independent formatting context (root, `overflow` other than
//!   `visible`, absolutely positioned, flex/grid/table container, inline-block, table cell).
//!
//! Known simplification: a parent does not collapse its top margin with an empty first child.
//!
//! Floats are placed by the block container that contains them. They shorten the line boxes of
//! inline content in that container but do not move block siblings. Floats do not escape their
//! container.
//!
//! Detached layout (`layout_detached`) lays out a box at a position chosen by the caller without
//! margin collapsing. Floats, absolutely positioned boxes, flex/grid items, table cells, and
//! inline-blocks all use it.

use alloc::vec::Vec;
use core::cell::Cell;

use crate::{
    flex, grid, inline, intrinsic,
    math::Rect,
    pagination,
    position::{layout_deferred, Deferred},
    px::Px,
    sizing::{
        block_size, clamp_block, clamp_inline, inline_size, is_zero_or_auto, to_content, Sides,
    },
    style::{BoxStyle, Clear, Dimension, Display, Float, Overflow, Position},
    table,
    text::TextMeasurer,
    tree::{BoxTree, Frame, NodeId, TreeError},
};

/// Deepest nesting of boxes the layout will descend into. Deeper trees return
/// `TreeError::TooDeep`. The limit keeps the recursion within a default thread stack.
pub const MAX_DEPTH: u32 = 128;

/// Shared state for one layout pass.
pub(crate) struct Pass<'a> {
    pub(crate) measurer: &'a dyn TextMeasurer,
    pub(crate) root: NodeId,
    depth: Cell<u32>,
    too_deep: Cell<bool>,
}

impl<'a> Pass<'a> {
    pub(crate) fn new(measurer: &'a dyn TextMeasurer, root: NodeId) -> Self {
        Pass {
            measurer,
            root,
            depth: Cell::new(0),
            too_deep: Cell::new(false),
        }
    }

    /// Descends one level. Returns `false` (and records the error) past `MAX_DEPTH`.
    fn enter(&self) -> bool {
        let depth = self.depth.get() + 1;
        self.depth.set(depth);
        if depth > MAX_DEPTH {
            self.too_deep.set(true);
            return false;
        }
        true
    }

    fn leave(&self) {
        self.depth.set(self.depth.get().saturating_sub(1));
    }

    pub(crate) fn too_deep(&self) -> bool {
        self.too_deep.get()
    }
}

/// The containing block a child is laid out in. `height` is `None` when indefinite.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Containing {
    pub x: Px,
    pub width: Px,
    pub height: Option<Px>,
}

/// A pair of margins that have not yet been placed. `pos` is the largest positive margin and
/// `neg` the most negative one (`neg <= 0`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Margin {
    pos: Px,
    neg: Px,
}

impl Margin {
    pub(crate) const ZERO: Margin = Margin {
        pos: Px::ZERO,
        neg: Px::ZERO,
    };

    pub(crate) fn single(margin: Px) -> Self {
        if margin.0 >= 0 {
            Margin {
                pos: margin,
                neg: Px::ZERO,
            }
        } else {
            Margin {
                pos: Px::ZERO,
                neg: margin,
            }
        }
    }

    pub(crate) fn collapse(self, other: Margin) -> Self {
        Margin {
            pos: self.pos.max(other.pos),
            neg: self.neg.min(other.neg),
        }
    }

    pub(crate) fn resolve(self) -> Px {
        self.pos + self.neg
    }
}

/// Vertical flow position within a containing block.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cursor {
    /// Bottom edge of the last placed border box (margins not included).
    pub y: Px,
    /// Margins following the last placed box, waiting to be placed or collapsed.
    pub pending: Margin,
}

impl Cursor {
    pub(crate) fn new(y: Px) -> Self {
        Cursor {
            y,
            pending: Margin::ZERO,
        }
    }

    /// Places any pending margins.
    pub(crate) fn flush(&mut self) {
        self.y += self.pending.resolve();
        self.pending = Margin::ZERO;
    }
}

/// Floats placed so far in one block container.
#[derive(Clone, Debug, Default)]
pub(crate) struct Floats {
    entries: Vec<(Float, Rect)>,
}

impl Floats {
    pub(crate) fn push(&mut self, side: Float, rect: Rect) {
        self.entries.push((side, rect));
    }

    /// Bottom edge that a box with `clear` must start below. Returns `Px(i32::MIN)` when no float
    /// matches, so callers take the maximum with the current cursor.
    pub(crate) fn clear_y(&self, clear: Clear) -> Px {
        self.entries
            .iter()
            .filter(|(side, _)| match clear {
                Clear::None => false,
                Clear::Both => true,
                Clear::Left => *side == Float::Left,
                Clear::Right => *side == Float::Right,
            })
            .map(|(_, r)| r.y + r.height)
            .fold(Px(i32::MIN), |acc, bottom| acc.max(bottom))
    }

    /// Right edge of the left floats overlapping `[top, bottom)`, or `min` if none.
    pub(crate) fn left_edge(&self, top: Px, bottom: Px, min: Px) -> Px {
        self.entries
            .iter()
            .filter(|(side, r)| *side == Float::Left && overlaps(r, top, bottom))
            .map(|(_, r)| r.x + r.width)
            .fold(min, |acc, edge| acc.max(edge))
    }

    /// Left edge of the right floats overlapping `[top, bottom)`, or `max` if none.
    pub(crate) fn right_edge(&self, top: Px, bottom: Px, max: Px) -> Px {
        self.entries
            .iter()
            .filter(|(side, r)| *side == Float::Right && overlaps(r, top, bottom))
            .map(|(_, r)| r.x)
            .fold(max, |acc, edge| acc.min(edge))
    }
}

fn overlaps(r: &Rect, top: Px, bottom: Px) -> bool {
    !r.height.is_zero() && r.y < bottom && r.y + r.height > top
}

/// Result of laying out a detached box.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Placed {
    pub border_height: Px,
    /// Baseline, measured from the border-box top.
    pub baseline: Px,
}

/// Where and how wide a detached box goes. `height`, when set, is a content height that
/// overrides the box's own height (used by flex and grid stretching).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Slot {
    pub x: Px,
    pub y: Px,
    pub content_width: Px,
    pub cb_width: Px,
    pub cb_height: Option<Px>,
    pub height: Option<Px>,
}

/// Lays out `root` and its descendants in flow at the origin, within `available_width`.
pub fn layout_flow(
    tree: &mut BoxTree,
    root: NodeId,
    available_width: Px,
    measurer: &dyn TextMeasurer,
) -> Result<(), TreeError> {
    if tree.get(root).is_none() {
        return Err(TreeError::NotFound(root));
    }
    let pass = Pass::new(measurer, root);
    let mut cursor = Cursor::new(Px::ZERO);
    let mut deferred = Vec::new();
    let containing = Containing {
        x: Px::ZERO,
        width: available_width,
        height: None,
    };
    layout_in_flow(
        &pass,
        tree,
        root,
        containing,
        &mut cursor,
        false,
        &mut deferred,
    );
    if pass.too_deep() {
        return Err(TreeError::TooDeep);
    }

    // Absolutely positioned boxes with no positioned ancestor use the root's extent as their
    // containing block (a stand-in for the initial containing block).
    let viewport = Rect {
        x: Px::ZERO,
        y: Px::ZERO,
        width: available_width,
        height: cursor.y,
    };
    layout_deferred(&pass, tree, deferred, viewport);
    Ok(())
}

/// Boxes that establish an independent formatting context. Their children's margins never
/// collapse with them.
fn establishes_bfc(pass: &Pass, style: &BoxStyle, id: NodeId) -> bool {
    id == pass.root
        || style.overflow != Overflow::Visible
        || style.position == Position::Absolute
        || matches!(
            style.display,
            Display::Flex
                | Display::Grid
                | Display::Table
                | Display::TableCell
                | Display::InlineBlock
        )
}

/// Children that take part in block flow at the top level: not absolutely positioned, not
/// floated, not inline-level.
fn is_block_flow_child(child: &BoxStyle) -> bool {
    child.position != Position::Absolute
        && child.float == Float::None
        && !child.display.is_inline_level()
}

/// The first child that is not absolutely positioned or floated, if it is block-level. Margin
/// collapsing only looks through this child.
fn first_in_flow_block_child(tree: &BoxTree, id: NodeId) -> Option<NodeId> {
    let first = tree.children(id).iter().copied().find(|child| {
        tree.get(*child)
            .is_some_and(|n| n.style.position != Position::Absolute && n.style.float == Float::None)
    })?;
    let node = tree.get(first)?;
    is_block_flow_child(&node.style).then_some(first)
}

/// True for a box with no content, no vertical padding or border, and no in-flow children. Its
/// top and bottom margins collapse with each other and with its neighbours.
fn is_empty_block(pass: &Pass, tree: &BoxTree, id: NodeId, cb_width: Px) -> bool {
    let Some(node) = tree.get(id) else {
        return false;
    };
    let style = &node.style;
    if node.text.is_some() || establishes_bfc(pass, style, id) {
        return false;
    }
    let sides = Sides::of(style, cb_width);
    if sides.top_separated() || sides.bottom_separated() {
        return false;
    }
    if !is_zero_or_auto(style.height) || !is_zero_or_auto(style.min_height) {
        return false;
    }
    // Absolutely positioned children do not count as content, but they must still be laid out.
    // The empty-box shortcut skips children entirely, so a box with any child is not empty.
    tree.children(id).is_empty()
}

/// Whether the top margin of `id` collapses with the top margin of its first block child.
fn collapses_top_with_first_child(pass: &Pass, tree: &BoxTree, id: NodeId, cb_width: Px) -> bool {
    let Some(node) = tree.get(id) else {
        return false;
    };
    let style = &node.style;
    if establishes_bfc(pass, style, id) || node.text.is_some() {
        return false;
    }
    let sides = Sides::of(style, cb_width);
    if sides.top_separated() {
        return false;
    }
    let content_width = inline_size(style, &sides, cb_width).0;
    first_in_flow_block_child(tree, id)
        .is_some_and(|child| !is_empty_block(pass, tree, child, content_width))
}

/// Whether the bottom margin of a box collapses with its last in-flow child's bottom margin.
/// Requires an auto height so the parent's extent depends on its content.
fn collapses_bottom(pass: &Pass, id: NodeId, style: &BoxStyle, sides: &Sides) -> bool {
    !establishes_bfc(pass, style, id)
        && !sides.bottom_separated()
        && style.height == Dimension::Auto
        && is_zero_or_auto(style.min_height)
}

/// The margin a box presents above itself: its own top margin, collapsed with its first child's
/// chain when they collapse.
fn outer_top_chain(pass: &Pass, tree: &BoxTree, id: NodeId, cb_width: Px) -> Margin {
    let Some(node) = tree.get(id) else {
        return Margin::ZERO;
    };
    let sides = Sides::of(&node.style, cb_width);
    let own = Margin::single(sides.margin.top);
    if !collapses_top_with_first_child(pass, tree, id, cb_width) {
        return own;
    }
    let content_width = inline_size(&node.style, &sides, cb_width).0;
    match first_in_flow_block_child(tree, id) {
        Some(child) => own.collapse(outer_top_chain(pass, tree, child, content_width)),
        None => own,
    }
}

pub(crate) fn relative_offset(style: &BoxStyle, cb: Containing) -> (Px, Px) {
    if style.position != Position::Relative {
        return (Px::ZERO, Px::ZERO);
    }
    let dx = match (
        style.inset.left.resolve(Some(cb.width)),
        style.inset.right.resolve(Some(cb.width)),
    ) {
        (Some(left), _) => left,
        (None, Some(right)) => -right,
        (None, None) => Px::ZERO,
    };
    let dy = match (
        style.inset.top.resolve(cb.height),
        style.inset.bottom.resolve(cb.height),
    ) {
        (Some(top), _) => top,
        (None, Some(bottom)) => -bottom,
        (None, None) => Px::ZERO,
    };
    (dx, dy)
}

/// Lays out a block-level box in flow, one nesting level down. Past `MAX_DEPTH` the box is left
/// out and the pass reports `TreeError::TooDeep`.
pub(crate) fn layout_in_flow(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    cb: Containing,
    cursor: &mut Cursor,
    inherit_top: bool,
    deferred: &mut Vec<Deferred>,
) -> Option<Px> {
    if !pass.enter() {
        return None;
    }
    let result = layout_in_flow_dispatch(pass, tree, id, cb, cursor, inherit_top, deferred);
    pass.leave();
    result
}

/// Lays out a box at a caller-chosen position, one nesting level down. See `layout_in_flow`.
pub(crate) fn layout_detached(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    slot: Slot,
    deferred: &mut Vec<Deferred>,
) -> Option<Placed> {
    if !pass.enter() {
        return None;
    }
    let result = layout_detached_dispatch(pass, tree, id, slot, deferred);
    pass.leave();
    result
}

fn layout_in_flow_dispatch(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    cb: Containing,
    cursor: &mut Cursor,
    inherit_top: bool,
    deferred: &mut Vec<Deferred>,
) -> Option<Px> {
    let Some(mode) = crate::writing::root_mode(tree, id) else {
        return layout_in_flow_horizontal(pass, tree, id, cb, cursor, inherit_top, deferred);
    };
    let style = tree.get(id)?.style;
    let sides = Sides::of(&style, cb.width);
    if !inherit_top {
        cursor.pending = cursor.pending.collapse(Margin::single(sides.margin.top));
        cursor.flush();
    }
    let (content, margin_left, _) = inline_size(&style, &sides, cb.width);
    let y = cursor.y;
    // A root inside another writing mode has no physical containing block to size against.
    let physical = crate::writing::caller_is_physical(tree, id);
    let placed = crate::writing::layout_vertical(
        pass,
        tree,
        id,
        mode,
        crate::writing::VerticalGeometry {
            origin: (cb.x + margin_left, y),
            content_block: physical.then_some(content),
            content_inline: None,
            cb: physical.then_some((cb.width, cb.height)),
        },
        deferred,
    );
    cursor.y = y + placed.border_height;
    cursor.pending = Margin::single(sides.margin.bottom);
    None
}

/// Lays out a box at a caller-chosen position. Vertical writing modes are handled in `writing.rs`.
/// For a vertical box, `content_width` is the physical content width and `height` the physical
/// content height.
fn layout_detached_dispatch(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    slot: Slot,
    deferred: &mut Vec<Deferred>,
) -> Option<Placed> {
    if let Some(mode) = crate::writing::root_mode(tree, id) {
        let physical = crate::writing::caller_is_physical(tree, id);
        return Some(crate::writing::layout_vertical(
            pass,
            tree,
            id,
            mode,
            crate::writing::VerticalGeometry {
                origin: (slot.x, slot.y),
                content_block: physical.then_some(slot.content_width),
                content_inline: if physical { slot.height } else { None },
                cb: physical.then_some((slot.cb_width, slot.cb_height)),
            },
            deferred,
        ));
    }
    layout_detached_inner(pass, tree, id, slot, deferred)
}

/// Lays out one block-level box in horizontal writing mode, with margin collapsing. Margins collapse as
/// described in the module docs.
/// `inherit_top` is set when the parent already placed this box's top margin chain (the parent
/// collapses with its first child), so the chain must not be placed again.
///
/// Returns the box's absolute baseline, if it has one.
pub(crate) fn layout_in_flow_horizontal(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    cb: Containing,
    cursor: &mut Cursor,
    inherit_top: bool,
    deferred: &mut Vec<Deferred>,
) -> Option<Px> {
    let style = tree.get(id)?.style;
    let sides = Sides::of(&style, cb.width);
    let extra_h = sides.horizontal_extra();
    let extra_v = sides.vertical_extra();

    if is_empty_block(pass, tree, id, cb.width) {
        cursor.pending = cursor
            .pending
            .collapse(Margin::single(sides.margin.top))
            .collapse(Margin::single(sides.margin.bottom));
        let (content_width, margin_left, _) = inline_size(&style, &sides, cb.width);
        tree.set_frame(
            id,
            Frame {
                x: cb.x + margin_left,
                y: cursor.y,
                width: content_width + extra_h,
                height: Px::ZERO,
                baseline: Px::ZERO,
            },
        );
        return None;
    }

    if !inherit_top {
        let chain = outer_top_chain(pass, tree, id, cb.width);
        cursor.pending = cursor.pending.collapse(chain);
        cursor.flush();
    }

    let (content_width, margin_left, _) = inline_size(&style, &sides, cb.width);
    let y_border = cursor.y;
    let x_border = cb.x + margin_left;
    let explicit_height = block_size(&style, cb.height, extra_v);
    let inherit_first = collapses_top_with_first_child(pass, tree, id, cb.width);
    let content = Containing {
        x: x_border + sides.border.left + sides.padding.left,
        width: content_width,
        height: explicit_height,
    };
    let content_top = y_border + sides.border.top + sides.padding.top;
    let positioned = style.position != Position::Static;

    // A positioned box owns the absolutely positioned boxes beneath it; they are placed once its
    // padding box is known.
    let mut own = Vec::new();
    let (children_end, children_pending, child_baseline) = if positioned {
        layout_content(
            pass,
            tree,
            id,
            content,
            content_top,
            inherit_first,
            &mut own,
        )
    } else {
        layout_content(
            pass,
            tree,
            id,
            content,
            content_top,
            inherit_first,
            deferred,
        )
    };

    // Auto heights never go negative, even when negative margins pull the content up.
    let natural = (children_end - content_top).max(Px::ZERO);
    let bottom_margin = Margin::single(sides.margin.bottom);
    let (content_height, trailing) = match explicit_height {
        Some(height) => (
            clamp_block(&style, cb.height, extra_v, height),
            bottom_margin,
        ),
        None => {
            let clamped = clamp_block(&style, cb.height, extra_v, natural);
            if clamped != natural {
                // Min/max clamping moves the bottom edge away from the content, so the trailing
                // margin stays inside the box.
                (clamped, bottom_margin)
            } else if collapses_bottom(pass, id, &style, &sides) {
                (natural, children_pending.collapse(bottom_margin))
            } else {
                (
                    (natural + children_pending.resolve()).max(Px::ZERO),
                    bottom_margin,
                )
            }
        }
    };

    let border_height = content_height + extra_v;
    let baseline = child_baseline.map_or(border_height, |b| b - y_border);
    tree.set_frame(
        id,
        Frame {
            x: x_border,
            y: y_border,
            width: content_width + extra_h,
            height: border_height,
            baseline,
        },
    );

    if positioned {
        let padding_box = Rect {
            x: x_border + sides.border.left,
            y: y_border + sides.border.top,
            width: content_width + sides.padding.horizontal(),
            height: content_height + sides.padding.vertical(),
        };
        layout_deferred(pass, tree, own, padding_box);
    }

    let (dx, dy) = relative_offset(&style, cb);
    if !dx.is_zero() || !dy.is_zero() {
        tree.shift_subtree(id, dx, dy);
    }

    // The cursor advances by the unshifted position: relative offsets never affect siblings.
    cursor.y = y_border + border_height;
    cursor.pending = trailing;
    Some(y_border + dy + baseline)
}

/// Lays out a box at a caller-chosen position, without margin collapsing. The box's own margins
/// are the caller's concern.
///
/// Absolutely positioned descendants are placed once this box's padding box is known. Other
/// deferred descendants are appended to `deferred` at their static positions, which are in this
/// box's coordinates before any relative offset is applied. Callers that move the box afterwards
/// must shift the entries they appended (see `shift_deferred`).
pub(crate) fn layout_detached_inner(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    slot: Slot,
    deferred: &mut Vec<Deferred>,
) -> Option<Placed> {
    let style = tree.get(id)?.style;
    let sides = Sides::of(&style, slot.cb_width);
    let extra_h = sides.horizontal_extra();
    let extra_v = sides.vertical_extra();
    let explicit = slot
        .height
        .or_else(|| block_size(&style, slot.cb_height, extra_v));

    let content = Containing {
        x: slot.x + sides.border.left + sides.padding.left,
        width: slot.content_width,
        height: explicit,
    };
    let top = slot.y + sides.border.top + sides.padding.top;
    let positioned = style.position != Position::Static;

    let mut own = Vec::new();
    let (end, pending, baseline) = if positioned {
        layout_content(pass, tree, id, content, top, false, &mut own)
    } else {
        layout_content(pass, tree, id, content, top, false, deferred)
    };

    let natural = (end + pending.resolve() - top).max(Px::ZERO);
    let content_height = match explicit {
        Some(height) => clamp_block(&style, slot.cb_height, extra_v, height),
        None => clamp_block(&style, slot.cb_height, extra_v, natural),
    };
    let border_height = content_height + extra_v;
    let baseline = baseline.map_or(border_height, |b| b - slot.y);
    tree.set_frame(
        id,
        Frame {
            x: slot.x,
            y: slot.y,
            width: slot.content_width + extra_h,
            height: border_height,
            baseline,
        },
    );

    if positioned {
        let padding_box = Rect {
            x: slot.x + sides.border.left,
            y: slot.y + sides.border.top,
            width: slot.content_width + sides.padding.horizontal(),
            height: content_height + sides.padding.vertical(),
        };
        layout_deferred(pass, tree, own, padding_box);
    }

    let (dx, dy) = relative_offset(
        &style,
        Containing {
            x: slot.x,
            width: slot.cb_width,
            height: slot.cb_height,
        },
    );
    if !dx.is_zero() || !dy.is_zero() {
        tree.shift_subtree(id, dx, dy);
    }
    Some(Placed {
        border_height,
        baseline,
    })
}

/// Moves the deferred entries from `from` onward by `(dx, dy)`, to match a moved subtree.
pub(crate) fn shift_deferred(
    deferred: &mut [Deferred],
    range: core::ops::Range<usize>,
    dx: Px,
    dy: Px,
) {
    for entry in deferred[range].iter_mut() {
        entry.static_x += dx;
        entry.static_y += dy;
    }
}

/// Lays out the content of `id` inside `content`, starting at `top`, by display type.
///
/// Returns the bottom edge of the last child's border box, its trailing margin (not yet placed),
/// and the first child's absolute baseline.
pub(crate) fn layout_content(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    content: Containing,
    top: Px,
    inherit_first: bool,
    deferred: &mut Vec<Deferred>,
) -> (Px, Margin, Option<Px>) {
    let Some(node) = tree.get(id) else {
        return (top, Margin::ZERO, None);
    };
    if let Some(text) = node.text.as_deref() {
        let lines = pass.measurer.line_boxes(text, content.width);
        let mut y = top;
        let mut baseline = None;
        for line in &lines {
            if baseline.is_none() {
                baseline = Some(y + line.baseline);
            }
            y += line.height;
        }
        tree.set_lines(id, lines);
        return (y, Margin::ZERO, baseline);
    }
    match node.style.display {
        Display::Flex => flex::layout_flex(pass, tree, id, content, top, deferred),
        Display::Grid => grid::layout_grid(pass, tree, id, content, top, deferred),
        Display::Table => table::layout_table(pass, tree, id, content, top, deferred),
        _ if node.style.column_count > 1 => {
            pagination::layout_columns(pass, tree, id, content, top, deferred)
        }
        _ => layout_block_children(pass, tree, id, content, top, inherit_first, deferred),
    }
}

/// Block children of `id`, with inline runs, floats, clearance, and margin collapsing.
pub(crate) fn layout_block_children(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    content: Containing,
    top: Px,
    inherit_first: bool,
    deferred: &mut Vec<Deferred>,
) -> (Px, Margin, Option<Px>) {
    let children = tree.children(id).to_vec();
    let mut cursor = Cursor::new(top);
    let mut floats = Floats::default();
    let mut baseline = None;
    let mut first_block = true;
    let mut index = 0;
    while index < children.len() {
        let child = children[index];
        let Some(style) = tree.get(child).map(|n| n.style) else {
            index += 1;
            continue;
        };
        if style.position == Position::Absolute {
            deferred.push(Deferred {
                id: child,
                static_x: content.x,
                static_y: cursor.y,
            });
            index += 1;
        } else if style.float != Float::None {
            cursor.flush();
            cursor.y = cursor.y.max(floats.clear_y(style.clear));
            place_float(
                pass,
                tree,
                child,
                content,
                &mut cursor,
                &mut floats,
                deferred,
            );
            index += 1;
        } else if style.display.is_inline_level() {
            let start = index;
            while index < children.len()
                && tree.get(children[index]).is_some_and(|n| {
                    n.style.display.is_inline_level()
                        && n.style.float == Float::None
                        && n.style.position != Position::Absolute
                })
            {
                index += 1;
            }
            cursor.flush();
            let run = &children[start..index];
            let run_baseline =
                inline::layout_run(pass, tree, run, content, &mut cursor, &mut floats, deferred);
            baseline = baseline.or(run_baseline);
            first_block = false;
        } else {
            let inherit = first_block && inherit_first;
            first_block = false;
            if style.clear != Clear::None {
                cursor.flush();
                cursor.y = cursor.y.max(floats.clear_y(style.clear));
            }
            let child_baseline =
                layout_in_flow(pass, tree, child, content, &mut cursor, inherit, deferred);
            baseline = baseline.or(child_baseline);
            index += 1;
        }
    }
    (cursor.y, cursor.pending, baseline)
}

/// Places a float at the current line position on its side of `content`.
fn place_float(
    pass: &Pass,
    tree: &mut BoxTree,
    child: NodeId,
    content: Containing,
    cursor: &mut Cursor,
    floats: &mut Floats,
    deferred: &mut Vec<Deferred>,
) {
    let Some(style) = tree.get(child).map(|n| n.style) else {
        return;
    };
    let sides = Sides::of(&style, content.width);
    let extra_h = sides.horizontal_extra();
    let content_width = match style.width.resolve(Some(content.width)) {
        Some(width) => clamp_inline(
            &style,
            content.width,
            extra_h,
            to_content(&style, width, extra_h),
        ),
        None => intrinsic::shrink_to_fit(pass, tree, child, content.width),
    };
    let mark = deferred.len();
    let Some(placed) = layout_detached(
        pass,
        tree,
        child,
        Slot {
            x: Px::ZERO,
            y: Px::ZERO,
            content_width,
            cb_width: content.width,
            cb_height: content.height,
            height: None,
        },
        deferred,
    ) else {
        return;
    };
    let border_width = content_width + extra_h;
    let top = cursor.y + sides.margin.top;
    let bottom = top + placed.border_height;
    let x = match style.float {
        Float::Right => {
            let right = floats
                .right_edge(top, bottom, content.x + content.width)
                .min(content.x + content.width);
            right - sides.margin.right - border_width
        }
        _ => {
            let left = floats.left_edge(top, bottom, content.x);
            left + sides.margin.left
        }
    };
    tree.shift_subtree(child, x, top);
    let end = deferred.len();
    shift_deferred(deferred, mark..end, x, top);
    floats.push(
        style.float,
        Rect {
            x,
            y: top,
            width: border_width,
            height: placed.border_height,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_shorten_lines_only_where_they_overlap() {
        let mut floats = Floats::default();
        floats.push(
            Float::Left,
            Rect {
                x: Px::ZERO,
                y: Px::ZERO,
                width: Px::from_px(50),
                height: Px::from_px(20),
            },
        );
        assert_eq!(
            floats.left_edge(Px::from_px(0), Px::from_px(10), Px::ZERO),
            Px::from_px(50)
        );
        assert_eq!(
            floats.left_edge(Px::from_px(20), Px::from_px(30), Px::ZERO),
            Px::ZERO
        );
        assert_eq!(floats.clear_y(Clear::Left), Px::from_px(20));
        // No match gives the smallest value, so `cursor.max(clear_y)` leaves the cursor alone.
        assert_eq!(floats.clear_y(Clear::Right), Px(i32::MIN));
    }
}
