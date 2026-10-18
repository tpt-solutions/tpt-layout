//! Geometry helpers shared by the layout passes.

use crate::px::Px;

/// Four-sided values in top/right/bottom/left order (CSS order).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edges<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> Edges<T> {
    pub fn all(value: T) -> Self {
        Edges {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

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
    pub fn horizontal(self) -> Px {
        self.left + self.right
    }

    pub fn vertical(self) -> Px {
        self.top + self.bottom
    }
}

/// An axis-aligned rectangle in absolute document coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: Px,
    pub y: Px,
    pub width: Px,
    pub height: Px,
}
