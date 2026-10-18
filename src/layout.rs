//! Output of a layout pass.

/// Computed geometry for one box, in `f32` pixels.
///
/// `x` and `y` are the border-box origin in absolute document coordinates. `width` and `height`
/// are the border box. `baseline` is measured from the top of the border box (for text, the first
/// line's baseline; for containers, the first in-flow child's baseline).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComputedLayout {
    /// Left edge of the border box, in document coordinates.
    pub x: f32,
    /// Top edge of the border box, in document coordinates.
    pub y: f32,
    /// Border-box width.
    pub width: f32,
    /// Border-box height.
    pub height: f32,
    /// Baseline, measured from the top of the box.
    pub baseline: f32,
}
