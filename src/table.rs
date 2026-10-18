//! Table layout.
//!
//! A table's children are rows (`Display::TableRow`), and a row's children are cells. A child
//! that is not a row is treated as a row holding just itself.
//!
//! Cells are placed on a grid. A row's cells fill the columns that no cell from an earlier row
//! occupies, and a cell with `row_span` also occupies its columns in the rows below it.
//!
//! Column widths come from the cells' max-content widths. A cell that spans several columns
//! (`column_span`) widens them evenly when it needs more room than they have. If the columns fit
//! the table, the spare width is shared in proportion to them. If they don't, every column shrinks
//! in the same proportion. Each cell is laid out once at the width of the columns it spans.
//!
//! Row heights: each row is as tall as its tallest single-row cell. A cell that spans rows must fit
//! the rows it covers; any shortfall is added to its last row. Cells are stretched to the height of
//! the rows they cover.
//!
//! Not supported yet: `border-collapse`, and `vertical-align` other than top. Pagination treats a
//! row as one unit, so a cell that spans rows stays with the row it starts in. Header rows
//! (`table_header`) are reported to pagination, which repeats them on each page.

use alloc::vec::Vec;

use crate::{
    flow::{layout_detached, shift_deferred, Containing, Margin, Pass, Slot},
    intrinsic,
    position::Deferred,
    px::Px,
    sizing::Sides,
    style::{Dimension, Display, TableLayout},
    tree::{BoxTree, NodeId},
};

/// A cell on the grid: its node, the first column and row it occupies, and its spans.
#[derive(Clone, Copy)]
struct Placed {
    cell: NodeId,
    column: usize,
    span: usize,
    row_span: usize,
}

/// A cell after its layout, ready to be placed.
struct Laid {
    row: usize,
    column: usize,
    row_span: usize,
    cell: NodeId,
    border_height: Px,
    mark: core::ops::Range<usize>,
    baseline: Px,
}

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
    for child in tree.laid_children(id).to_vec() {
        let Some(node) = tree.get(child) else {
            continue;
        };
        if node.style.position.is_out_of_flow() {
            deferred.push(Deferred {
                id: child,
                static_x: content.x,
                static_y: top,
            });
            continue;
        }
        if node.style.display == Display::TableRow {
            rows.push(tree.laid_children(child).to_vec());
        } else {
            rows.push(alloc::vec![child]);
        }
    }
    if rows.is_empty() {
        return (top, Margin::ZERO, None);
    }

    // Place each row's cells on the grid. Columns held by a cell from an earlier row are skipped.
    let mut occupied: Vec<Vec<bool>> = Vec::new();
    let mut placements: Vec<Vec<Placed>> = Vec::with_capacity(rows.len());
    for (r, row) in rows.iter().enumerate() {
        let mut placed_row = Vec::new();
        let mut column = 0;
        for cell in row {
            while is_occupied(&occupied, r, column) {
                column += 1;
            }
            let span = span_of(tree, *cell);
            let row_span = row_span_of(tree, *cell);
            mark(&mut occupied, r, column, span, row_span);
            placed_row.push(Placed {
                cell: *cell,
                column,
                span,
                row_span,
            });
            column += span;
        }
        placements.push(placed_row);
    }
    let total_rows = rows.len().max(occupied.len());
    let column_count = placements
        .iter()
        .flatten()
        .map(|p| p.column + p.span)
        .max()
        .unwrap_or(0);

    // Single-column cells set their column's width first. Spanning cells then widen their columns
    // evenly, when they need more room than those columns give.
    let mut column_max = alloc::vec![Px::ZERO; column_count];
    for placed in placements.iter().flatten().filter(|p| p.span == 1) {
        let outer = outer_width(pass, tree, placed.cell, content.width);
        column_max[placed.column] = column_max[placed.column].max(outer);
    }
    for placed in placements.iter().flatten().filter(|p| p.span > 1) {
        let outer = outer_width(pass, tree, placed.cell, content.width);
        let range = placed.column..placed.column + placed.span;
        let have = column_max[range.clone()]
            .iter()
            .fold(Px::ZERO, |acc, w| acc + *w);
        if outer > have {
            let deficit = outer - have;
            let each = Px(deficit.0 / placed.span as i32);
            let remainder = deficit - Px(each.0 * placed.span as i32);
            for width in &mut column_max[range.clone()] {
                *width += each;
            }
            column_max[placed.column] += remainder;
        }
    }

    let column_widths = match tree.get(id).map(|n| n.style.table_layout) {
        Some(TableLayout::Fixed) => fixed_columns(tree, &placements, column_count, content.width),
        _ => fit_columns(&column_max, content.width),
    };
    let column_starts = {
        let mut out = Vec::with_capacity(column_count);
        let mut x = Px::ZERO;
        for width in &column_widths {
            out.push(x);
            x += *width;
        }
        out
    };

    // Lay out every cell once at the width of the columns it spans, recording row heights.
    let mut row_heights = alloc::vec![Px::ZERO; total_rows];
    let mut spanning: Vec<(usize, usize, Px)> = Vec::new();
    let mut laid_cells: Vec<Laid> = Vec::new();
    for (r, row) in placements.iter().enumerate() {
        for placed in row {
            let Some(node) = tree.get(placed.cell) else {
                continue;
            };
            let sides = Sides::of(&node.style, content.width);
            let width: Px = column_widths[placed.column..placed.column + placed.span]
                .iter()
                .fold(Px::ZERO, |acc, w| acc + *w);
            let content_width =
                (width - sides.horizontal_extra() - sides.margin.horizontal()).max(Px::ZERO);
            let mark = deferred.len();
            let Some(laid) = layout_detached(
                pass,
                tree,
                placed.cell,
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
            let outer_h = laid.border_height + sides.margin.vertical();
            if placed.row_span == 1 {
                row_heights[r] = row_heights[r].max(outer_h);
            } else {
                spanning.push((r, placed.row_span, outer_h));
            }
            laid_cells.push(Laid {
                row: r,
                column: placed.column,
                row_span: placed.row_span,
                cell: placed.cell,
                border_height: laid.border_height,
                mark: mark..deferred.len(),
                baseline: laid.baseline,
            });
        }
    }

    // A cell that spans rows must fit the rows it covers. Any shortfall goes to its last row.
    for (r, row_span, outer_h) in spanning {
        let have = row_heights[r..r + row_span]
            .iter()
            .fold(Px::ZERO, |acc, h| acc + *h);
        if outer_h > have {
            row_heights[r + row_span - 1] += outer_h - have;
        }
    }

    // Row y offsets.
    let mut row_tops = Vec::with_capacity(total_rows);
    let mut y = Px::ZERO;
    for height in &row_heights {
        row_tops.push(y);
        y += *height;
    }

    // Place cells and stretch them to the rows they cover.
    let mut baseline = None;
    for laid in laid_cells {
        let Some(node) = tree.get(laid.cell) else {
            continue;
        };
        let style = node.style;
        let sides = Sides::of(&style, content.width);
        let x = content.x + column_starts[laid.column] + sides.margin.left;
        let cell_y = top + row_tops[laid.row] + sides.margin.top;
        tree.shift_subtree(laid.cell, x, cell_y);
        shift_deferred(deferred, laid.mark, x, cell_y);
        if style.height == Dimension::Auto {
            let covered = row_heights[laid.row..laid.row + laid.row_span]
                .iter()
                .fold(Px::ZERO, |acc, h| acc + *h);
            let stretched =
                (covered - sides.vertical_extra() - sides.margin.vertical()).max(Px::ZERO);
            if let Some(mut frame) = tree.frame(laid.cell) {
                frame.height = stretched;
                tree.set_frame(laid.cell, frame);
            }
        }
        if baseline.is_none() && laid.row == 0 {
            baseline = Some(cell_y + laid.baseline.min(laid.border_height));
        }
    }

    let height = row_heights.iter().fold(Px::ZERO, |acc, h| acc + *h);
    (top + height, Margin::ZERO, baseline)
}

/// Whether the grid slot at `row`, `column` is held by a cell from an earlier row.
fn is_occupied(occupied: &[Vec<bool>], row: usize, column: usize) -> bool {
    occupied
        .get(row)
        .and_then(|cells| cells.get(column))
        .copied()
        .unwrap_or(false)
}

/// Marks the grid slots a cell covers: `span` columns across `row_span` rows from `row`.
fn mark(occupied: &mut Vec<Vec<bool>>, row: usize, column: usize, span: usize, row_span: usize) {
    for r in row..row + row_span {
        if occupied.len() <= r {
            occupied.resize(r + 1, Vec::new());
        }
        let cells = &mut occupied[r];
        if cells.len() < column + span {
            cells.resize(column + span, false);
        }
        for slot in &mut cells[column..column + span] {
            *slot = true;
        }
    }
}

/// Column widths for `table-layout: fixed`. The first row's single-column cells with a fixed width
/// set their columns. Undeclared columns share the rest of `available` equally. When the declared
/// widths already add up to more than `available`, they are kept and the table overflows.
fn fixed_columns(
    tree: &BoxTree,
    placements: &[Vec<Placed>],
    column_count: usize,
    available: Px,
) -> Vec<Px> {
    let mut widths: Vec<Option<Px>> = alloc::vec![None; column_count];
    if let Some(first) = placements.first() {
        for placed in first.iter().filter(|p| p.span == 1) {
            let declared = tree
                .get(placed.cell)
                .and_then(|node| match node.style.width {
                    Dimension::Length(width) => Some(width),
                    _ => None,
                });
            widths[placed.column] = declared;
        }
    }
    let declared_total = widths.iter().flatten().fold(Px::ZERO, |acc, w| acc + *w);
    let undeclared = widths.iter().filter(|w| w.is_none()).count() as i32;
    let share = if undeclared > 0 {
        Px(((available - declared_total).max(Px::ZERO)).0 / undeclared)
    } else {
        Px::ZERO
    };
    widths.iter().map(|w| w.unwrap_or(share)).collect()
}

/// Columns a cell spans. `column_span` of `0` counts as `1`.
fn span_of(tree: &BoxTree, cell: NodeId) -> usize {
    tree.get(cell)
        .map_or(1, |node| node.style.column_span.max(1) as usize)
}

/// Rows a cell spans. `row_span` of `0` counts as `1`.
fn row_span_of(tree: &BoxTree, cell: NodeId) -> usize {
    tree.get(cell)
        .map_or(1, |node| node.style.row_span.max(1) as usize)
}

/// Max-content width of a cell's border box, including its margins.
fn outer_width(pass: &Pass, tree: &BoxTree, cell: NodeId, container: Px) -> Px {
    let sides = match tree.get(cell) {
        Some(node) => Sides::of(&node.style, container),
        None => return Px::ZERO,
    };
    intrinsic::max_content(pass, tree, cell) + sides.horizontal_extra() + sides.margin.horizontal()
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
