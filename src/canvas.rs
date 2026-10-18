//! Output boundary for `tpt-canvas`. Stub only: the canvas crate does not exist yet.
//!
//! tpt-layout never draws. A renderer walks the tree and reads each node's
//! [`ComputedLayout`].

use crate::{layout::ComputedLayout, tree::NodeId};

/// Receives boxes while a renderer walks a laid-out tree.
pub trait Canvas {
    /// Draws one box at its computed layout.
    fn draw_box(&mut self, node: NodeId, layout: &ComputedLayout);
}
