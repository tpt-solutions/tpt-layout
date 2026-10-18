//! Arena-backed box tree. Nodes are addressed by [`NodeId`] and owned by [`BoxTree`].
//!
//! Slots are never reused, so a stale `NodeId` after `remove` resolves to `None` instead of
//! silently aliasing a new node.
//!
//! Dirty tracking: any mutable access through [`BoxTree::get_mut`] and any structural edit marks
//! the affected node dirty. [`crate::engine::relayout`] uses the flags to re-lay-out only the
//! affected subtrees.

use alloc::{string::String, vec::Vec};
use core::{fmt, ops::Range};

use crate::{
    layout::ComputedLayout,
    px::Px,
    style::{BoxStyle, Dimension, GridTemplate},
    text::LineBox,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u32);

impl NodeId {
    /// Arena slot index. Stable for the life of the tree.
    pub fn index(self) -> usize {
        self.0 as usize
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
    pub x: Px,
    pub y: Px,
    pub width: Px,
    pub height: Px,
    /// Byte range of the source text this fragment covers.
    pub text_range: Range<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoxNode {
    pub id: NodeId,
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
    NotFound(NodeId),
    AlreadyHasParent(NodeId),
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
pub struct BoxTree {
    slots: Vec<Option<BoxNode>>,
}

impl BoxTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of live nodes.
    pub fn len(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

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
        let index = u32::try_from(self.slots.len()).expect("box tree exceeds u32::MAX nodes");
        let id = NodeId(index);
        self.slots.push(Some(BoxNode {
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
        }));
        id
    }

    pub fn get(&self, id: NodeId) -> Option<&BoxNode> {
        self.slots.get(id.index())?.as_ref()
    }

    /// Mutable access. Marks the node dirty, since the caller may change anything about it.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut BoxNode> {
        let node = self.slots.get_mut(id.index())?.as_mut()?;
        node.dirty = true;
        Some(node)
    }

    /// Mutable access without marking dirty. For layout-internal writes only.
    fn node_mut(&mut self, id: NodeId) -> Option<&mut BoxNode> {
        self.slots.get_mut(id.index())?.as_mut()
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.get(id)?.parent
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
            if let Some(node) = self.slots.get_mut(current.index()).and_then(Option::take) {
                stack.extend(node.children);
            }
        }
        Ok(())
    }

    /// Marks `id` dirty without reading or writing anything else.
    pub fn mark_dirty(&mut self, id: NodeId) {
        let _ = self.get_mut(id);
    }

    pub(crate) fn is_dirty(&self, id: NodeId) -> bool {
        self.get(id).is_some_and(|n| n.dirty)
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
