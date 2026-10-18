//! Arena-backed box tree. Nodes are addressed by [`NodeId`] and owned by [`BoxTree`].
//!
//! Freed slots are reused, but each `NodeId` carries a generation. A stale `NodeId` (one whose node
//! was removed) resolves to `None` instead of aliasing whatever now occupies the slot.
//!
//! Dirty tracking: any mutable access through [`BoxTree::get_mut`] and any structural edit marks
//! the affected node dirty. [`crate::engine::relayout`] uses the flags to re-lay-out only the
//! affected subtrees.

use alloc::{string::String, vec::Vec};
use core::{fmt, ops::Range};

use crate::{
    layout::ComputedLayout,
    px::Px,
    style::{BoxStyle, Dimension, Display, GridTemplate},
    text::LineBox,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Handle to a box in a [`BoxTree`]. It stops matching once the box is removed.
pub struct NodeId {
    slot: u32,
    generation: u32,
}

impl NodeId {
    /// Arena slot index. Stable while the node is alive; the slot may be reused after removal.
    pub fn index(self) -> usize {
        self.slot as usize
    }
}

/// Internal fixed-point geometry. `BoxNode::layout` is the `f32` view of this.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Frame {
    pub x: Px,
    pub y: Px,
    pub width: Px,
    pub height: Px,
    pub baseline: Px,
}

/// One run of text placed on a line inside an inline formatting context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fragment {
    /// Left edge, relative to the tree's origin.
    pub x: Px,
    /// Top edge, relative to the tree's origin.
    pub y: Px,
    /// Width.
    pub width: Px,
    /// Height.
    pub height: Px,
    /// Byte range of the source text this fragment covers.
    pub text_range: Range<usize>,
}

#[derive(Clone, Debug, PartialEq)]
/// A box in the tree, with its style and computed layout.
pub struct BoxNode {
    /// Its handle.
    pub id: NodeId,
    /// Its style.
    pub style: BoxStyle,
    /// Text content. A node with text is a leaf sized by the [`TextMeasurer`](crate::TextMeasurer).
    pub text: Option<String>,
    /// Grid template, used when this box is a grid container.
    pub grid: GridTemplate,
    /// Computed layout, filled in by the layout pass.
    pub layout: ComputedLayout,
    /// Line boxes of a block-level text leaf, set by layout. Used by pagination for orphans and
    /// widows.
    pub lines: Vec<LineBox>,
    /// Per-line text fragments of an inline text leaf, set by layout.
    pub fragments: Vec<Fragment>,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    pub(crate) frame: Frame,
    pub(crate) dirty: bool,
    /// A replaced element (for example an image): a leaf with an intrinsic size.
    pub(crate) replaced: bool,
    /// True once layout has written this node's frame.
    pub(crate) laid_out: bool,
}

/// Errors from tree edits. Layout itself never fails on a well-formed tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeError {
    /// No live box has this id.
    NotFound(NodeId),
    /// The box already has a parent.
    AlreadyHasParent(NodeId),
    /// The edit would make a box its own ancestor.
    WouldCycle,
    /// Layout descended past `flow::MAX_DEPTH`.
    TooDeep,
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TreeError::NotFound(id) => write!(f, "node {} does not exist", id.index()),
            TreeError::AlreadyHasParent(id) => {
                write!(f, "node {} already has a parent", id.index())
            }
            TreeError::WouldCycle => write!(f, "append would make a node its own ancestor"),
            TreeError::TooDeep => write!(f, "layout nested deeper than the supported maximum"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TreeError {}

#[derive(Clone, Debug, Default, PartialEq)]
/// An arena of boxes, addressed by [`NodeId`].
pub struct BoxTree {
    slots: Vec<Option<BoxNode>>,
    /// Generation of each slot. Bumped when a node is removed, so older ids no longer match.
    generations: Vec<u32>,
    /// Slots freed by `remove`, reused by the next insert.
    free: Vec<u32>,
    /// Root and constraints of the last full layout pass, so `relayout` can detect a change.
    last_layout: Option<(NodeId, crate::engine::LayoutConstraints)>,
}

impl BoxTree {
    /// An empty tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of live nodes.
    pub fn len(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }

    /// True when no box is live.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Adds a box and returns its id.
    pub fn insert(&mut self, style: BoxStyle) -> NodeId {
        self.push(style, None)
    }

    /// Inserts a replaced element (an image-like leaf) with an intrinsic size. `auto` width and
    /// height take the intrinsic size; explicit sizes in `style` win.
    pub fn insert_replaced(
        &mut self,
        mut style: BoxStyle,
        intrinsic_width: Px,
        intrinsic_height: Px,
    ) -> NodeId {
        if style.width == Dimension::Auto {
            style.width = Dimension::Length(intrinsic_width);
        }
        if style.height == Dimension::Auto {
            style.height = Dimension::Length(intrinsic_height);
        }
        let id = self.push(style, None);
        if let Some(node) = self.node_mut(id) {
            node.replaced = true;
        }
        id
    }

    /// Inserts a text leaf.
    pub fn insert_text(&mut self, style: BoxStyle, text: impl Into<String>) -> NodeId {
        self.push(style, Some(text.into()))
    }

    fn push(&mut self, style: BoxStyle, text: Option<String>) -> NodeId {
        let slot = match self.free.pop() {
            Some(slot) => slot,
            None => {
                let slot =
                    u32::try_from(self.slots.len()).expect("box tree exceeds u32::MAX nodes");
                self.slots.push(None);
                self.generations.push(0);
                slot
            }
        };
        let id = NodeId {
            slot,
            generation: self.generations[slot as usize],
        };
        self.slots[slot as usize] = Some(BoxNode {
            id,
            style,
            text,
            grid: GridTemplate::default(),
            layout: ComputedLayout::default(),
            lines: Vec::new(),
            fragments: Vec::new(),
            parent: None,
            children: Vec::new(),
            frame: Frame::default(),
            dirty: true,
            replaced: false,
            laid_out: false,
        });
        id
    }

    /// The box with this id, or `None` if it was removed.
    pub fn get(&self, id: NodeId) -> Option<&BoxNode> {
        self.slots
            .get(id.index())?
            .as_ref()
            .filter(|node| node.id == id)
    }

    /// Mutable access. Marks the node dirty, since the caller may change anything about it.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut BoxNode> {
        let node = self.node_mut(id)?;
        node.dirty = true;
        Some(node)
    }

    /// Mutable access without marking dirty. For layout-internal writes only.
    fn node_mut(&mut self, id: NodeId) -> Option<&mut BoxNode> {
        self.slots
            .get_mut(id.index())?
            .as_mut()
            .filter(|node| node.id == id)
    }

    /// The parent of `id`, if it has one.
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.get(id)?.parent
    }

    /// Direction of `id` after inheritance: its own value, or its nearest ancestor's that is set.
    /// `Ltr` at the top of the tree.
    pub(crate) fn effective_direction(&self, id: NodeId) -> crate::style::Direction {
        use crate::style::Direction;
        let mut current = Some(id);
        while let Some(node_id) = current {
            let Some(node) = self.get(node_id) else {
                break;
            };
            match node.style.direction {
                Direction::Inherit => current = node.parent,
                set => return set,
            }
        }
        Direction::Ltr
    }

    /// `text-align` of `id` after inheritance. `Start` at the top of the tree.
    pub(crate) fn effective_text_align(&self, id: NodeId) -> crate::style::TextAlign {
        use crate::style::TextAlign;
        let mut current = Some(id);
        while let Some(node_id) = current {
            let Some(node) = self.get(node_id) else {
                break;
            };
            match node.style.text_align {
                TextAlign::Inherit => current = node.parent,
                set => return set,
            }
        }
        TextAlign::Start
    }

    /// Children that take part in layout: every child except those with `display: none`. Used by
    /// the layout passes; the public [`BoxTree::children`] still returns every child.
    pub(crate) fn laid_children(&self, id: NodeId) -> Vec<NodeId> {
        self.children(id)
            .iter()
            .copied()
            .filter(|child| {
                self.get(*child)
                    .is_some_and(|node| node.style.display != Display::None)
            })
            .collect()
    }

    /// Children in document order. Empty for unknown ids.
    pub fn children(&self, id: NodeId) -> &[NodeId] {
        match self.get(id) {
            Some(node) => &node.children,
            None => &[],
        }
    }

    /// Appends `child` as the last child of `parent`. The child must be a detached root.
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), TreeError> {
        if self.get(parent).is_none() {
            return Err(TreeError::NotFound(parent));
        }
        let child_node = self.get(child).ok_or(TreeError::NotFound(child))?;
        if child_node.parent.is_some() {
            return Err(TreeError::AlreadyHasParent(child));
        }
        if self.is_self_or_ancestor(child, parent) {
            return Err(TreeError::WouldCycle);
        }
        if let Some(node) = self.node_mut(child) {
            node.parent = Some(parent);
        }
        if let Some(node) = self.get_mut(parent) {
            node.children.push(child);
        }
        Ok(())
    }

    /// Detaches `id` from its parent and frees its whole subtree.
    pub fn remove(&mut self, id: NodeId) -> Result<(), TreeError> {
        let parent = self.get(id).ok_or(TreeError::NotFound(id))?.parent;
        if let Some(parent) = parent {
            if let Some(node) = self.get_mut(parent) {
                node.children.retain(|child| *child != id);
            }
        }
        let mut stack = Vec::new();
        stack.push(id);
        while let Some(current) = stack.pop() {
            let slot = current.index();
            if let Some(node) = self.slots.get_mut(slot).and_then(Option::take) {
                stack.extend(node.children);
                self.generations[slot] = self.generations[slot].wrapping_add(1);
                self.free.push(current.slot);
            }
        }
        Ok(())
    }

    /// Detaches `id` from its parent without freeing it. The node and its subtree keep their ids
    /// and can be re-attached with [`BoxTree::append_child`], [`BoxTree::insert_child_at`] or
    /// [`BoxTree::insert_before`]. Detaching a root is a no-op.
    pub fn detach(&mut self, id: NodeId) -> Result<(), TreeError> {
        let parent = self.get(id).ok_or(TreeError::NotFound(id))?.parent;
        let Some(parent) = parent else {
            return Ok(());
        };
        if let Some(node) = self.get_mut(parent) {
            node.children.retain(|child| *child != id);
        }
        if let Some(node) = self.get_mut(id) {
            node.parent = None;
        }
        Ok(())
    }

    /// Inserts `child` (which must have no parent) at position `index` among `parent`'s children.
    /// `index` past the end appends.
    pub fn insert_child_at(
        &mut self,
        parent: NodeId,
        index: usize,
        child: NodeId,
    ) -> Result<(), TreeError> {
        self.append_child(parent, child)?;
        let children = &mut self
            .get_mut(parent)
            .ok_or(TreeError::NotFound(parent))?
            .children;
        let last = children.len() - 1;
        let child = children.remove(last);
        children.insert(index.min(last), child);
        Ok(())
    }

    /// Inserts `child` (which must have no parent) directly before `reference` under the same
    /// parent. Fails with `NotFound` if `reference` is a root.
    pub fn insert_before(&mut self, reference: NodeId, child: NodeId) -> Result<(), TreeError> {
        let parent = self
            .get(reference)
            .ok_or(TreeError::NotFound(reference))?
            .parent;
        let parent = parent.ok_or(TreeError::NotFound(reference))?;
        let index = self
            .children(parent)
            .iter()
            .position(|c| *c == reference)
            .ok_or(TreeError::NotFound(reference))?;
        self.insert_child_at(parent, index, child)
    }

    /// Moves `child` (with its subtree) under `new_parent`, appended after its existing siblings.
    /// Fails with `WouldCycle` if `new_parent` is inside `child`'s subtree.
    pub fn reparent(&mut self, child: NodeId, new_parent: NodeId) -> Result<(), TreeError> {
        self.get(child).ok_or(TreeError::NotFound(child))?;
        self.get(new_parent)
            .ok_or(TreeError::NotFound(new_parent))?;
        if self.is_self_or_ancestor(child, new_parent) {
            return Err(TreeError::WouldCycle);
        }
        self.detach(child)?;
        self.append_child(new_parent, child)
    }

    /// Marks `id` dirty without reading or writing anything else.
    pub fn mark_dirty(&mut self, id: NodeId) {
        let _ = self.get_mut(id);
    }

    pub(crate) fn is_dirty(&self, id: NodeId) -> bool {
        self.get(id).is_some_and(|n| n.dirty)
    }

    pub(crate) fn last_layout(&self) -> Option<(NodeId, crate::engine::LayoutConstraints)> {
        self.last_layout
    }

    pub(crate) fn set_last_layout(
        &mut self,
        root: NodeId,
        constraints: crate::engine::LayoutConstraints,
    ) {
        self.last_layout = Some((root, constraints));
    }

    pub(crate) fn clear_dirty(&mut self) {
        for node in self.slots.iter_mut().flatten() {
            node.dirty = false;
        }
    }

    fn is_self_or_ancestor(&self, ancestor: NodeId, node: NodeId) -> bool {
        let mut current = Some(node);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.parent(id);
        }
        false
    }

    /// `root` and all its descendants, in depth-first order.
    /// Ids of `root` and its descendants, in document order.
    pub fn subtree_ids(&self, root: NodeId) -> Vec<NodeId> {
        self.subtree(root)
    }

    pub(crate) fn subtree(&self, root: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = Vec::new();
        stack.push(root);
        while let Some(id) = stack.pop() {
            if let Some(node) = self.get(id) {
                out.push(id);
                stack.extend(node.children.iter().rev().copied());
            }
        }
        out
    }

    /// Sets the grid template of `id` (used when it is a grid container). Marks the node dirty.
    pub fn set_grid_template(&mut self, id: NodeId, template: GridTemplate) -> bool {
        match self.get_mut(id) {
            Some(node) => {
                node.grid = template;
                true
            }
            None => false,
        }
    }

    /// Replaces a node's style without marking it dirty. Used only while a vertical subtree is laid
    /// out in its logical space; the original style is restored before the pass returns.
    pub(crate) fn set_style_silently(&mut self, id: NodeId, style: BoxStyle) {
        if let Some(node) = self.node_mut(id) {
            node.style = style;
        }
    }

    /// Mutable fragments of a node, for mapping them between spaces.
    pub(crate) fn fragments_mut(&mut self, id: NodeId) -> Option<&mut Vec<Fragment>> {
        self.node_mut(id).map(|n| &mut n.fragments)
    }

    pub(crate) fn set_frame(&mut self, id: NodeId, frame: Frame) {
        if let Some(node) = self.node_mut(id) {
            node.frame = frame;
            node.laid_out = true;
            node.layout = ComputedLayout {
                x: frame.x.to_f32(),
                y: frame.y.to_f32(),
                width: frame.width.to_f32(),
                height: frame.height.to_f32(),
                baseline: frame.baseline.to_f32(),
            };
        }
    }

    pub(crate) fn frame(&self, id: NodeId) -> Option<Frame> {
        self.get(id).map(|n| n.frame)
    }

    pub(crate) fn set_lines(&mut self, id: NodeId, lines: Vec<LineBox>) {
        if let Some(node) = self.node_mut(id) {
            node.lines = lines;
        }
    }

    pub(crate) fn set_fragments(&mut self, id: NodeId, fragments: Vec<Fragment>) {
        if let Some(node) = self.node_mut(id) {
            node.fragments = fragments;
        }
    }

    /// Translates `id` and all its descendants by `(dx, dy)`, including their fragments.
    pub(crate) fn shift_subtree(&mut self, id: NodeId, dx: Px, dy: Px) {
        let mut stack = Vec::new();
        stack.push(id);
        while let Some(current) = stack.pop() {
            let Some(node) = self.node_mut(current) else {
                continue;
            };
            stack.extend(node.children.iter().copied());
            let mut frame = node.frame;
            for fragment in node.fragments.iter_mut() {
                fragment.x += dx;
                fragment.y += dy;
            }
            frame.x += dx;
            frame.y += dy;
            self.set_frame(current, frame);
        }
    }
}
