//! Output of a layout pass.

/// Computed geometry for one box, in `f32` pixels.
///
/// `x` and `y` are the border-box origin in absolute document coordinates. `width` and `height`
/// are the border box. `baseline` is measured from the top of the border box (for text, the first
/// line's baseline; for containers, the first in-flow child's baseline).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComputedLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
}
