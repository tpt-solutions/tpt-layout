//! Table layout (TODO Phase 4, table module).
//!
//! A table's children are rows (`Display::TableRow`), and a row's children are cells. A child
//! that is not a row is treated as a row holding just itself.
//!
//! Column widths come from the cells' max-content widths. If they fit, the spare width is shared
//! in proportion to them. If they don't, every column shrinks in the same proportion. Each cell
//! is laid out once at its column width, and its row is as tall as its tallest cell. Cells are
//! stretched to the row height.
//!
//! Not supported yet: `colspan` and `rowspan`, `border-collapse`, and `vertical-align` other than
//! top. Header rows (`table_header`) are reported to pagination, which repeats them on each page.

use alloc::vec::Vec;

use crate::{
    flow::{layout_detached, shift_deferred, Containing, Margin, Pass, Slot},
    intrinsic,
    position::Deferred,
    px::Px,
    style::Display,
    tree::{BoxTree, NodeId},
};

/// Lays out table `id` inside `content`, starting at `top`.
pub(crate) fn layout_table(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    content: Containing,
    top: Px,
    deferred: &mut Vec<Deferred>,
) -> (Px, Margin, Option<Px>) {
    // Rows as lists of cells, in document order.
    let mut rows: Vec<Vec<NodeId>> = Vec::new();
    for child in tree.children(id).to_vec() {
        let Some(node) = tree.get(child) else {
            continue;
        };
        if node.style.position == crate::style::Position::Absolute {
            deferred.push(Deferred {
                id: child,
                static_x: content.x,
                static_y: top,
            });
            continue;
        }
        if node.style.display == Display::TableRow {
            rows.push(tree.children(child).to_vec());
        } else {
            rows.push(alloc::vec![child]);
        }
    }
    if rows.is_empty() {
        return (top, Margin::ZERO, None);
    }

    let column_count = rows.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut column_max = alloc::vec![Px::ZERO; column_count];
    for row in &rows {
        for (c, cell) in row.iter().enumerate() {
            let sides = match tree.get(*cell) {
                Some(node) => crate::sizing::Sides::of(&node.style, content.width),
                None => continue,
            };
            let outer = intrinsic::max_content(pass, tree, *cell)
                + sides.horizontal_extra()
                + sides.margin.horizontal();
            column_max[c] = column_max[c].max(outer);
        }
    }
    let column_widths = fit_columns(&column_max, content.width);
    let column_starts = {
        let mut out = Vec::with_capacity(column_count);
        let mut x = Px::ZERO;
        for width in &column_widths {
            out.push(x);
            x += *width;
        }
        out
    };

    // Lay out every cell once at its column width, recording row heights.
    let mut row_heights = alloc::vec![Px::ZERO; rows.len()];
    let mut cells: Vec<(usize, usize, NodeId, Px, core::ops::Range<usize>, Px)> = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            let Some(node) = tree.get(*cell) else {
                continue;
            };
            let style = node.style;
            let sides = crate::sizing::Sides::of(&style, content.width);
            let width = column_widths[c];
            let content_width =
                (width - sides.horizontal_extra() - sides.margin.horizontal()).max(Px::ZERO);
            let mark = deferred.len();
            let Some(placed) = layout_detached(
                pass,
                tree,
                *cell,
                Slot {
                    x: Px::ZERO,
                    y: Px::ZERO,
                    content_width,
                    cb_width: content.width,
                    cb_height: None,
                    height: None,
                },
                deferred,
            ) else {
                continue;
            };
            let outer_h = placed.border_height + sides.margin.vertical();
            row_heights[r] = row_heights[r].max(outer_h);
            cells.push((
                r,
                c,
                *cell,
                placed.border_height,
                mark..deferred.len(),
                placed.baseline,
            ));
        }
    }

    // Row y offsets.
    let mut row_tops = Vec::with_capacity(rows.len());
    let mut y = Px::ZERO;
    for height in &row_heights {
        row_tops.push(y);
        y += *height;
    }

    // Place cells and stretch them to their row.
    let mut baseline = None;
    for (r, c, cell, border_height, mark, cell_baseline) in cells {
        let Some(node) = tree.get(cell) else { continue };
        let style = node.style;
        let sides = crate::sizing::Sides::of(&style, content.width);
        let x = content.x + column_starts[c] + sides.margin.left;
        let cell_y = top + row_tops[r] + sides.margin.top;
        tree.shift_subtree(cell, x, cell_y);
        shift_deferred(deferred, mark, x, cell_y);
        if style.height == crate::style::Dimension::Auto {
            let stretched =
                (row_heights[r] - sides.vertical_extra() - sides.margin.vertical()).max(Px::ZERO);
            if let Some(mut frame) = tree.frame(cell) {
                frame.height = stretched;
                tree.set_frame(cell, frame);
            }
        }
        if baseline.is_none() && r == 0 {
            baseline = Some(cell_y + cell_baseline.min(border_height));
        }
    }

    let height = row_heights.iter().fold(Px::ZERO, |acc, h| acc + *h);
    (top + height, Margin::ZERO, baseline)
}

/// Column widths: the max-content widths when they fit, otherwise the same proportion of
/// `available`. Any spare space is shared in proportion to the max-content widths.
fn fit_columns(column_max: &[Px], available: Px) -> Vec<Px> {
    let total = column_max.iter().fold(Px::ZERO, |acc, w| acc + *w);
    if column_max.is_empty() {
        return Vec::new();
    }
    if total.is_zero() {
        let each = available / column_max.len() as i32;
        return alloc::vec![each; column_max.len()];
    }
    // Each column becomes `w * available / total`, rounded half away from zero.
    column_max
        .iter()
        .map(|w| w.mul_ratio(available.0 as i64, total.0 as i64))
        .collect()
}
