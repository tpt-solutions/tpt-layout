//! # tpt-layout
//!
//! Deterministic box, flow, and page layout engine. See `README.md` for the architecture overview
//! and `spec.txt` for the design contract.
//!
//! ## Conventions
//! - Internal math is integer fixed-point ([`Px`], 1/64 px). `f32` appears only at the output
//!   boundary ([`ComputedLayout`], [`pagination::Page`]) and in `Px::from_f32` / `to_f32`.
//! - Layout is single-pass and top-down. Nothing iterates to convergence.
//! - `ComputedLayout::x`/`y` are the border-box origin in absolute document coordinates.
//! - Halves round away from zero everywhere.

#![no_std]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

extern crate alloc;

#[cfg(any(test, feature = "std"))]
extern crate std;

pub mod canvas;
pub mod engine;
pub mod flow;
pub mod layout;
pub mod math;
pub mod pagination;
pub mod paint;
pub mod px;
pub mod snapshot;
pub mod style;
pub mod svg;
pub mod templates;
pub mod text;
pub mod tree;

pub(crate) mod flex;
pub(crate) mod grid;
pub(crate) mod inline;
pub(crate) mod intrinsic;
pub(crate) mod position;
pub(crate) mod sizing;
pub(crate) mod table;
pub(crate) mod writing;

pub use engine::{layout_document, relayout, LayoutConstraints, LayoutContext, LayoutResult};
pub use flow::layout_flow;
pub use layout::ComputedLayout;
pub use math::{Edges, Rect};
pub use pagination::{paginate, Page, PageConstraints, PageFragment, PageLayout};
pub use paint::{paint_order, PaintItem};
pub use px::Px;
pub use style::{
    AlignContent, AlignItems, BoxSizing, BoxStyle, Break, Clear, Dimension, Direction, Display,
    FlexDirection, FlexWrap, Float, GridAutoFlow, GridLine, GridPlacement, GridTemplate,
    JustifyContent, Overflow, Position, TableLayout, TextAlign, TrackSize, Tracks, WhiteSpace,
    WritingMode,
};
pub use text::{FixedAdvanceMeasurer, LineBox, TextMeasurer};
pub use tree::{BoxNode, BoxTree, Fragment, NodeId, TreeError, TreeScope};
