//! Intrinsic (max-content) sizing. Pure queries: nothing here writes to the tree.
//!
//! These are used to size auto-width boxes that have no containing width to fill: flex items,
//! floats, inline-blocks, and grid or table tracks. They are approximations of the CSS algorithms.
//! Percentages inside intrinsic sizing resolve against zero, and grid and table content is
//! approximated from its items rather than solved exactly.

use crate::{
    flow::Pass,
    px::Px,
    sizing::{clamp_inline, to_content, Sides},
    style::{Dimension, Display, FlexDirection, Float, Position},
    tree::{BoxTree, NodeId},
};

/// Max-content width of the content box of `id`, honouring a fixed `width` and fixed min/max.
pub(crate) fn max_content(pass: &Pass, tree: &BoxTree, id: NodeId) -> Px {
    let Some(node) = tree.get(id) else {
        return Px::ZERO;
    };
    let style = &node.style;
    let extra = Sides::of(style, Px::ZERO).horizontal_extra();
    let natural = match node.text.as_deref() {
        Some(text) => pass
            .measurer
            .line_boxes(text, Px::MAX)
            .iter()
            .map(|line| line.width)
            .fold(Px::ZERO, |acc, w| acc.max(w)),
        None => natural_content(pass, tree, id),
    };
    let width = match style.width {
        Dimension::Length(w) => to_content(style, w, extra),
        _ => natural,
    };
    let min = match style.min_width {
        Dimension::Length(v) => to_content(style, v, extra),
        _ => Px::ZERO,
    };
    let max = match style.max_width {
        Dimension::Length(v) => Some(to_content(style, v, extra)),
        _ => None,
    };
    width.clamp_to(min, max)
}

/// Outer (margin-box) max-content width of a child, with percentages treated as zero.
fn outer_width(pass: &Pass, tree: &BoxTree, child: NodeId) -> Px {
    let Some(node) = tree.get(child) else {
        return Px::ZERO;
    };
    let sides = Sides::of(&node.style, Px::ZERO);
    max_content(pass, tree, child) + sides.horizontal_extra() + sides.margin.horizontal()
}

fn natural_content(pass: &Pass, tree: &BoxTree, id: NodeId) -> Px {
    let Some(node) = tree.get(id) else {
        return Px::ZERO;
    };
    let children = tree.children(id);
    match node.style.display {
        Display::Flex => {
            let items = children.iter().filter(|c| {
                tree.get(**c)
                    .is_some_and(|n| n.style.position != Position::Absolute)
            });
            let gap = gap_px(node.style.column_gap);
            match node.style.flex_direction {
                FlexDirection::Row | FlexDirection::RowReverse => {
                    let mut count = 0;
                    let mut sum = Px::ZERO;
                    for child in items {
                        sum += outer_width(pass, tree, *child);
                        count += 1;
                    }
                    if count > 1 {
                        sum += gap * (count - 1);
                    }
                    sum
                }
                FlexDirection::Column | FlexDirection::ColumnReverse => {
                    items.fold(Px::ZERO, |acc, c| acc.max(outer_width(pass, tree, *c)))
                }
            }
        }
        Display::Table => children
            .iter()
            .map(|row| {
                tree.children(*row)
                    .iter()
                    .fold(Px::ZERO, |acc, cell| acc + outer_width(pass, tree, *cell))
            })
            .fold(Px::ZERO, |acc, w| acc.max(w)),
        Display::Grid => {
            let columns = (node.grid.columns.len as i32).max(1);
            let widest = children
                .iter()
                .fold(Px::ZERO, |acc, c| acc.max(outer_width(pass, tree, *c)));
            widest * columns
        }
        _ => {
            // Block container: inline runs sum, block children take the maximum.
            let mut best = Px::ZERO;
            let mut run = Px::ZERO;
            for child in children {
                let Some(child_node) = tree.get(*child) else {
                    continue;
                };
                let style = &child_node.style;
                if style.position == Position::Absolute || style.float != Float::None {
                    continue;
                }
                let width = outer_width(pass, tree, *child);
                if style.display.is_inline_level() {
                    run += width;
                } else {
                    best = best.max(run).max(width);
                    run = Px::ZERO;
                }
            }
            best.max(run)
        }
    }
}

/// Shrink-to-fit width for a box placed in a containing block of `cb_width`: its max-content
/// width, limited to the space that is available once its own margins and edges are removed.
pub(crate) fn shrink_to_fit(pass: &Pass, tree: &BoxTree, id: NodeId, cb_width: Px) -> Px {
    let Some(node) = tree.get(id) else {
        return Px::ZERO;
    };
    let sides = Sides::of(&node.style, cb_width);
    let extra = sides.horizontal_extra();
    let available = (cb_width - extra - sides.margin.horizontal()).max(Px::ZERO);
    let natural = max_content(pass, tree, id).min(available);
    clamp_inline(&node.style, cb_width, extra, natural)
}

fn gap_px(gap: Dimension) -> Px {
    match gap {
        Dimension::Length(v) => v,
        _ => Px::ZERO,
    }
}
