//! Output boundary for `tpt-canvas`. Stub only: the canvas crate does not exist yet.
//!
//! tpt-layout never draws. A renderer walks the tree and reads each node's
//! [`ComputedLayout`](crate::ComputedLayout).

use crate::{layout::ComputedLayout, tree::NodeId};

pub trait Canvas {
    fn draw_box(&mut self, node: NodeId, layout: &ComputedLayout);
}
