//! Geometry helpers shared by the layout passes.

use crate::px::Px;

/// Four-sided values in top/right/bottom/left order (CSS order).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edges<T> {
    /// Top edge.
    pub top: T,
    /// Right edge.
    pub right: T,
    /// Bottom edge.
    pub bottom: T,
    /// Left edge.
    pub left: T,
}

impl<T: Copy> Edges<T> {
    /// The same value on every side.
    pub fn all(value: T) -> Self {
        Edges {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Applies `f` to each side.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Edges<U> {
        Edges {
            top: f(self.top),
            right: f(self.right),
            bottom: f(self.bottom),
            left: f(self.left),
        }
    }
}

impl Edges<Px> {
    /// Sum of the left and right sides.
    pub fn horizontal(self) -> Px {
        self.left + self.right
    }

    /// Sum of the top and bottom sides.
    pub fn vertical(self) -> Px {
        self.top + self.bottom
    }
}

/// An axis-aligned rectangle in absolute document coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: Px,
    /// Top edge.
    pub y: Px,
    /// Width.
    pub width: Px,
    /// Height.
    pub height: Px,
}
