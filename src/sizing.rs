//! CSS box-size and edge resolution shared by the flow and positioning passes.
//!
//! Horizontal edges and widths resolve percentages against the containing block width. Vertical
//! sizes resolve percentages only when the containing block height is definite.

use crate::{
    math::Edges,
    px::Px,
    style::{BoxSizing, BoxStyle, Dimension},
};

/// Resolved edge thicknesses for one box. Auto horizontal margins are recorded in `auto_*` and
/// stored as zero in `margin`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sides {
    pub margin: Edges<Px>,
    pub auto_left: bool,
    pub auto_right: bool,
    pub padding: Edges<Px>,
    pub border: Edges<Px>,
}

impl Sides {
    pub(crate) fn of(style: &BoxStyle, cb_width: Px) -> Self {
        Sides {
            margin: style
                .margin
                .map(|d| d.resolve(Some(cb_width)).unwrap_or(Px::ZERO)),
            auto_left: style.margin.left == Dimension::Auto,
            auto_right: style.margin.right == Dimension::Auto,
            padding: style
                .padding
                .map(|d| d.resolve(Some(cb_width)).unwrap_or(Px::ZERO).max(Px::ZERO)),
            border: style.border.map(|b| b.max(Px::ZERO)),
        }
    }

    pub(crate) fn horizontal_extra(&self) -> Px {
        self.padding.horizontal() + self.border.horizontal()
    }

    pub(crate) fn vertical_extra(&self) -> Px {
        self.padding.vertical() + self.border.vertical()
    }

    pub(crate) fn top_separated(&self) -> bool {
        !self.padding.top.is_zero() || !self.border.top.is_zero()
    }

    pub(crate) fn bottom_separated(&self) -> bool {
        !self.padding.bottom.is_zero() || !self.border.bottom.is_zero()
    }
}

pub(crate) fn is_zero_or_auto(d: Dimension) -> bool {
    match d {
        Dimension::Auto => true,
        Dimension::Length(v) => v.is_zero(),
        Dimension::Percent(h) => h == 0,
    }
}

/// Converts a resolved border-box or content-box size to a content-box size.
pub(crate) fn to_content(style: &BoxStyle, size: Px, extra: Px) -> Px {
    match style.box_sizing {
        BoxSizing::ContentBox => size,
        BoxSizing::BorderBox => (size - extra).max(Px::ZERO),
    }
}

/// Applies `min-width`/`max-width` to a content width.
pub(crate) fn clamp_inline(style: &BoxStyle, cb_width: Px, extra: Px, content: Px) -> Px {
    let min = style
        .min_width
        .resolve(Some(cb_width))
        .map_or(Px::ZERO, |v| to_content(style, v, extra));
    let max = style
        .max_width
        .resolve(Some(cb_width))
        .map(|v| to_content(style, v, extra));
    content.clamp_to(min, max)
}

/// Explicit content height, if the height is definite.
pub(crate) fn block_size(style: &BoxStyle, cb_height: Option<Px>, extra: Px) -> Option<Px> {
    style
        .height
        .resolve(cb_height)
        .map(|v| to_content(style, v, extra))
}

/// Content height implied by `aspect-ratio` for an `auto` height, given the content width.
pub(crate) fn aspect_height(style: &BoxStyle, content_width: Px) -> Option<Px> {
    let ratio = style.aspect_ratio.filter(|r| r.is_finite() && *r > 0.0)?;
    Some(Px::from_f32(content_width.to_f32() / ratio))
}

/// Applies `min-height`/`max-height` to a content height.
pub(crate) fn clamp_block(style: &BoxStyle, cb_height: Option<Px>, extra: Px, content: Px) -> Px {
    let min = style
        .min_height
        .resolve(cb_height)
        .map_or(Px::ZERO, |v| to_content(style, v, extra));
    let max = style
        .max_height
        .resolve(cb_height)
        .map(|v| to_content(style, v, extra));
    content.clamp_to(min, max)
}

/// Resolves the in-flow width of a block. Returns `(content width, left margin, right margin)`.
///
/// An explicit width with auto margins centers the box: the leftover space is split equally,
/// and an odd pixel goes to the right.
pub(crate) fn inline_size(style: &BoxStyle, sides: &Sides, cb_width: Px) -> (Px, Px, Px) {
    let extra = sides.horizontal_extra();
    match style.width.resolve(Some(cb_width)) {
        Some(width) => {
            let content = clamp_inline(style, cb_width, extra, to_content(style, width, extra));
            let remaining = cb_width - content - extra - sides.margin.left - sides.margin.right;
            let (ml, mr) = match (sides.auto_left, sides.auto_right) {
                (true, true) => {
                    let space = remaining.max(Px::ZERO);
                    let left = space / 2;
                    (left, space - left)
                }
                (true, false) => (remaining, sides.margin.right),
                (false, true) => (sides.margin.left, remaining),
                (false, false) => (sides.margin.left, sides.margin.right),
            };
            (content, ml, mr)
        }
        None => {
            let available =
                (cb_width - extra - sides.margin.left - sides.margin.right).max(Px::ZERO);
            (
                clamp_inline(style, cb_width, extra, available),
                sides.margin.left,
                sides.margin.right,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::aspect_height;
    use crate::{px::Px, style::BoxStyle};

    #[test]
    fn aspect_height_divides_width_by_ratio() {
        let style = BoxStyle {
            aspect_ratio: Some(2.0),
            ..BoxStyle::default()
        };
        assert_eq!(
            aspect_height(&style, Px::from_px(100)),
            Some(Px::from_px(50))
        );
    }

    #[test]
    fn missing_or_invalid_ratio_gives_none() {
        for ratio in [
            None,
            Some(0.0),
            Some(-1.0),
            Some(f32::NAN),
            Some(f32::INFINITY),
        ] {
            let style = BoxStyle {
                aspect_ratio: ratio,
                ..BoxStyle::default()
            };
            assert_eq!(aspect_height(&style, Px::from_px(100)), None, "{ratio:?}");
        }
    }
}

#[cfg(test)]
mod length_tests {
    use super::{clamp_block, to_content};
    use crate::{
        px::Px,
        style::{BoxSizing, BoxStyle, Dimension},
    };

    fn px(v: i32) -> Px {
        Px::from_px(v)
    }

    #[test]
    fn content_box_sizes_are_already_content_sizes() {
        let style = BoxStyle::default();
        assert_eq!(to_content(&style, px(100), px(20)), px(100));
    }

    #[test]
    fn border_box_sizes_subtract_the_extra_and_never_go_negative() {
        let style = BoxStyle {
            box_sizing: BoxSizing::BorderBox,
            ..BoxStyle::default()
        };
        assert_eq!(to_content(&style, px(100), px(20)), px(80));
        assert_eq!(to_content(&style, px(10), px(20)), px(0));
    }

    #[test]
    fn min_and_max_height_clamp_the_content_height() {
        let style = BoxStyle {
            min_height: Dimension::Length(px(40)),
            max_height: Dimension::Length(px(60)),
            ..BoxStyle::default()
        };
        assert_eq!(clamp_block(&style, None, px(0), px(10)), px(40));
        assert_eq!(clamp_block(&style, None, px(0), px(50)), px(50));
        assert_eq!(clamp_block(&style, None, px(0), px(90)), px(60));
    }
}
