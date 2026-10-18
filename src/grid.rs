//! CSS Grid layout (TODO Phase 3).
//!
//! Algorithm (single pass, no convergence loop):
//! 1. Resolve placements: named areas, explicit lines and spans, then auto-placement in the
//!    spec's order (both-definite, definite-major, definite-minor, fully auto). Auto-placement
//!    works in "major/minor" coordinates so `grid-auto-flow: column` reuses the row code.
//! 2. Size column tracks: fixed lengths and percentages, content-sized tracks from the
//!    single-span items, then `fr` tracks from the leftover space.
//! 3. Lay out each item once at its span's width, which gives its height.
//! 4. Size row tracks the same way, using item heights for content-sized rows. `fr` rows only
//!    take leftover space when the container height is definite.
//! 5. Position each item in its cell: shift it into place, then align it within the cell.
//!
//! Simplifications, documented in TODO.md:
//! - Named line names (`[name]`) are not supported. Named areas are.
//! - Content contributions from items spanning several tracks are spread over the last
//!   content-sized track they span, rather than solved exactly.
//! - `justify-content` and `align-content` are not applied (tracks fill the container).
//! - Intrinsic widths for content-sized tracks use `intrinsic::max_content`.

use alloc::vec::Vec;

use crate::{
    flow::{layout_detached, shift_deferred, Containing, Margin, Pass, Slot},
    intrinsic,
    position::Deferred,
    px::Px,
    sizing::{clamp_inline, to_content, Sides},
    style::{
        AlignItems, Dimension, Direction, GridAutoFlow, GridLine, Position, TrackSize, Tracks,
        MAX_AREA_DIM,
    },
    tree::{BoxTree, NodeId},
};

/// A grid item's placement on one axis, in 0-based track indices.
#[derive(Clone, Copy, Debug)]
struct Span {
    /// Start track if definite; `None` for auto-placement.
    start: Option<usize>,
    len: usize,
}

struct GridItem {
    id: NodeId,
    /// Placement in physical (row, column) terms after auto-placement.
    row: usize,
    col: usize,
    row_span: usize,
    col_span: usize,
    sides: Sides,
    width_is_auto: bool,
    height_is_auto: bool,
    justify: AlignItems,
    align: AlignItems,
    /// Filled in by the single layout pass.
    content_width: Px,
    border_height: Px,
    baseline: Px,
    /// Entries this item appended to the deferred list while it was laid out.
    mark: core::ops::Range<usize>,
}

/// Lays out the in-flow children of grid container `id`.
pub(crate) fn layout_grid(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    content: Containing,
    top: Px,
    deferred: &mut Vec<Deferred>,
) -> (Px, Margin, Option<Px>) {
    let Some(node) = tree.get(id) else {
        return (top, Margin::ZERO, None);
    };
    let style = node.style;
    let template = node.grid;
    let rtl = style.direction == Direction::Rtl;
    let column_flow = style.grid_auto_flow == GridAutoFlow::Column;
    let col_gap = gap(style.column_gap, content.width);
    let row_gap = gap(style.row_gap, content.width);

    let explicit_cols = template.columns.len as usize;
    let explicit_rows = template.rows.len as usize;

    let mut entries = Vec::new();
    for child in tree.children(id).to_vec() {
        let Some(child_node) = tree.get(child) else {
            continue;
        };
        if child_node.style.position == Position::Absolute {
            deferred.push(Deferred {
                id: child,
                static_x: content.x,
                static_y: top,
            });
            continue;
        }
        entries.push((child, child_node.style));
    }

    // 1. Placement.
    let mut placements: Vec<(Span, Span)> = Vec::with_capacity(entries.len());
    for (_, item_style) in &entries {
        let (row, col) = resolve_area(&template.areas, item_style.grid_area).unwrap_or_else(|| {
            (
                axis_span(
                    item_style.grid_row.start,
                    item_style.grid_row.end,
                    &template.rows.line_names,
                    explicit_rows,
                ),
                axis_span(
                    item_style.grid_column.start,
                    item_style.grid_column.end,
                    &template.columns.line_names,
                    explicit_cols,
                ),
            )
        });
        placements.push((row, col));
    }
    let (positions, occupied_rows, occupied_cols) = auto_place(
        &placements,
        column_flow,
        style.grid_auto_flow_dense,
        explicit_rows,
        explicit_cols,
    );
    let row_count = occupied_rows.max(explicit_rows).max(1);
    let col_count = occupied_cols.max(explicit_cols).max(1);

    // Items and their alignment.
    let mut items: Vec<GridItem> = Vec::with_capacity(entries.len());
    for (((child, item_style), (row, col)), (row_placement, col_placement)) in
        entries.iter().zip(positions).zip(&placements)
    {
        let sides = Sides::of(item_style, content.width);
        items.push(GridItem {
            id: *child,
            row,
            col,
            row_span: row_placement.len.max(1),
            col_span: col_placement.len.max(1),
            sides,
            width_is_auto: item_style.width == Dimension::Auto,
            height_is_auto: item_style.height == Dimension::Auto,
            justify: item_style.justify_self.unwrap_or(style.justify_items),
            align: item_style.align_self.unwrap_or(style.align_items),
            content_width: Px::ZERO,
            border_height: Px::ZERO,
            baseline: Px::ZERO,
            mark: 0..0,
        });
    }
    // 2. Column tracks. Content sizes come from single-span items.
    let mut col_content = alloc::vec![Px::ZERO; col_count];
    for item in &items {
        if item.col_span == 1 {
            let outer = intrinsic::max_content(pass, tree, item.id)
                + item.sides.horizontal_extra()
                + item.sides.margin.horizontal();
            col_content[item.col] = col_content[item.col].max(outer);
        }
    }
    let cols = size_tracks(
        &template.columns,
        style.grid_auto_columns,
        col_count,
        Some(content.width),
        col_gap,
        &col_content,
    );
    let col_starts = starts(&cols, col_gap);
    let col_width = |item: &GridItem| span_extent(&cols, item.col, item.col_span, col_gap);

    // 3. Lay out every item once at its span width.
    for item in items.iter_mut() {
        let width = col_width(item);
        let justify_stretch = item.justify == AlignItems::Stretch && item.width_is_auto;
        let item_style = tree.get(item.id).map(|n| n.style).unwrap_or_default();
        let extra = item.sides.horizontal_extra();
        let content_width = if !item.width_is_auto {
            let explicit = item_style
                .width
                .resolve(Some(content.width))
                .unwrap_or(Px::ZERO);
            clamp_inline(
                &item_style,
                content.width,
                extra,
                to_content(&item_style, explicit, extra),
            )
        } else if justify_stretch {
            clamp_inline(
                &item_style,
                content.width,
                extra,
                (width - extra - item.sides.margin.horizontal()).max(Px::ZERO),
            )
        } else {
            intrinsic::shrink_to_fit(pass, tree, item.id, width)
        };
        item.content_width = content_width;
        let mark_start = deferred.len();
        if let Some(placed) = layout_detached(
            pass,
            tree,
            item.id,
            Slot {
                x: Px::ZERO,
                y: Px::ZERO,
                content_width,
                cb_width: content.width,
                cb_height: content.height,
                height: None,
            },
            deferred,
        ) {
            item.border_height = placed.border_height;
            item.baseline = placed.baseline;
        }
        item.mark = mark_start..deferred.len();
    }

    // 4. Row tracks. Content sizes come from single-span items; multi-span items spread the rest.
    let mut row_content = alloc::vec![Px::ZERO; row_count];
    for item in &items {
        if item.row_span == 1 {
            let outer =
                item.border_height + item.sides.vertical_extra() + item.sides.margin.vertical();
            row_content[item.row] = row_content[item.row].max(outer);
        }
    }
    for item in &items {
        if item.row_span > 1 {
            let outer =
                item.border_height + item.sides.vertical_extra() + item.sides.margin.vertical();
            let spanned = span_extent(&row_content, item.row, item.row_span, row_gap);
            if outer > spanned {
                let last = item.row + item.row_span - 1;
                row_content[last] += outer - spanned;
            }
        }
    }
    let rows = size_tracks(
        &template.rows,
        style.grid_auto_rows,
        row_count,
        content.height,
        row_gap,
        &row_content,
    );
    let row_starts = starts(&rows, row_gap);

    // 5. Position each item in its cell.
    for item in &items {
        let cell_w = span_extent(&cols, item.col, item.col_span, col_gap);
        let cell_h = span_extent(&rows, item.row, item.row_span, row_gap);
        let col_x = col_starts[item.col];
        let logical_x = if rtl {
            content.width - (col_x + cell_w)
        } else {
            col_x
        };
        let outer_w =
            item.content_width + item.sides.horizontal_extra() + item.sides.margin.horizontal();
        let outer_h = item.border_height + item.sides.margin.vertical();
        let offset_x = align_offset(item.justify, cell_w, outer_w);
        let offset_y = align_offset(item.align, cell_h, outer_h);
        let final_x = content.x + logical_x + item.sides.margin.left + offset_x;
        let final_y = top + row_starts[item.row] + item.sides.margin.top + offset_y;
        tree.shift_subtree(item.id, final_x, final_y);
        shift_deferred(deferred, item.mark.clone(), final_x, final_y);

        if item.align == AlignItems::Stretch && item.height_is_auto {
            let stretched =
                (cell_h - item.sides.vertical_extra() - item.sides.margin.vertical()).max(Px::ZERO);
            if let Some(mut frame) = tree.frame(item.id) {
                frame.height = stretched;
                tree.set_frame(item.id, frame);
            }
        }
    }

    let total_rows = rows.iter().fold(Px::ZERO, |acc, size| acc + *size)
        + row_gap * (row_count as i32 - 1).max(0);
    let height = content.height.unwrap_or(total_rows);
    let baseline = items
        .first()
        .and_then(|item| tree.frame(item.id).map(|frame| frame.y + item.baseline));
    (top + height, Margin::ZERO, baseline)
}

fn gap(value: Dimension, base: Px) -> Px {
    value.resolve(Some(base)).unwrap_or(Px::ZERO).max(Px::ZERO)
}

/// Rows and columns of a named area, as (row span, column span) in 0-based tracks.
fn resolve_area(areas: &[[u8; MAX_AREA_DIM]; MAX_AREA_DIM], area: u8) -> Option<(Span, Span)> {
    if area == 0 {
        return None;
    }
    let mut bounds: Option<(usize, usize, usize, usize)> = None;
    for (r, row) in areas.iter().enumerate() {
        for (c, name) in row.iter().enumerate() {
            if *name == area {
                bounds = Some(match bounds {
                    None => (r, r, c, c),
                    Some((r0, r1, c0, c1)) => (r0.min(r), r1.max(r), c0.min(c), c1.max(c)),
                });
            }
        }
    }
    bounds.map(|(r0, r1, c0, c1)| {
        (
            Span {
                start: Some(r0),
                len: r1 - r0 + 1,
            },
            Span {
                start: Some(c0),
                len: c1 - c0 + 1,
            },
        )
    })
}

/// Resolves one axis of an explicit placement. Negative line numbers count from the end.
/// One side of a placement, resolved against the track list.
#[derive(Clone, Copy)]
enum Ref {
    /// 0-based grid line.
    Line(usize),
    Span(usize),
    Auto,
}

fn resolve_ref(line: GridLine, names: &[u8], tracks: usize) -> Ref {
    let index_line = |index: i32| -> usize {
        // 1-based line numbers; `-1` is the last line, which is `tracks + 1`.
        let one_based = if index > 0 {
            index
        } else {
            tracks as i32 + 2 + index
        };
        (one_based - 1).max(0) as usize
    };
    match line {
        GridLine::Auto => Ref::Auto,
        GridLine::Index(index) => Ref::Line(index_line(index)),
        GridLine::Span(n) => Ref::Span(n.max(1) as usize),
        GridLine::Named(name) => {
            if name == 0 {
                return Ref::Auto;
            }
            names
                .iter()
                .position(|&n| n == name)
                .map_or(Ref::Auto, Ref::Line)
        }
    }
}

/// Span of one axis from its start and end lines.
fn axis_span(start: GridLine, end: GridLine, names: &[u8], tracks: usize) -> Span {
    match (
        resolve_ref(start, names, tracks),
        resolve_ref(end, names, tracks),
    ) {
        (Ref::Line(s), Ref::Line(e)) => Span {
            start: Some(s),
            len: e.saturating_sub(s).max(1),
        },
        (Ref::Line(s), Ref::Span(n)) => Span {
            start: Some(s),
            len: n,
        },
        (Ref::Line(s), _) => Span {
            start: Some(s),
            len: 1,
        },
        (Ref::Auto, Ref::Line(e)) => Span {
            start: Some(e.saturating_sub(1)),
            len: 1,
        },
        (Ref::Span(n), _) | (Ref::Auto, Ref::Span(n)) => Span {
            start: None,
            len: n,
        },
        (Ref::Auto, Ref::Auto) => Span {
            start: None,
            len: 1,
        },
    }
}

/// Picks one axis's span out of a placement pair.type SpanPicker = fn(&(Span, Span)) -> Span;
/// Picks one axis's span out of a placement pair.
type SpanPicker = fn(&(Span, Span)) -> Span;

/// Auto-placement. Returns placements in physical (row, column) order and the occupied extents.
fn auto_place(
    placements: &[(Span, Span)],
    column_flow: bool,
    dense: bool,
    explicit_rows: usize,
    explicit_cols: usize,
) -> (Vec<(usize, usize)>, usize, usize) {
    // Work in major/minor coordinates: major is the auto-flow direction.
    let (major_of, minor_of): (SpanPicker, SpanPicker) = if column_flow {
        (|p| p.1, |p| p.0)
    } else {
        (|p| p.0, |p| p.1)
    };
    let fixed_minor_base = if column_flow {
        explicit_rows
    } else {
        explicit_cols
    }
    .max(1);

    let mut occupied: Vec<Vec<bool>> = Vec::new();
    let mut placed: Vec<Option<(usize, usize)>> = alloc::vec![None; placements.len()];
    let mut minor_extent = fixed_minor_base;
    let mut major_extent = 0usize;

    let mark = |occupied: &mut Vec<Vec<bool>>, major: usize, minor: usize, ms: usize, ns: usize| {
        while occupied.len() < major + ms {
            occupied.push(Vec::new());
        }
        for row in occupied.iter_mut().skip(major).take(ms) {
            if row.len() < minor + ns {
                row.resize(minor + ns, false);
            }
            for cell in row.iter_mut().skip(minor).take(ns) {
                *cell = true;
            }
        }
    };
    let free =
        |occupied: &Vec<Vec<bool>>, major: usize, minor: usize, ms: usize, ns: usize| -> bool {
            (major..major + ms).all(|r| {
                occupied.get(r).is_none_or(|row| {
                    (minor..minor + ns).all(|c| !row.get(c).copied().unwrap_or(false))
                })
            })
        };

    // Phase 1: both axes definite.
    for (i, p) in placements.iter().enumerate() {
        let (maj, min) = (major_of(p), minor_of(p));
        if let (Some(m0), Some(n0)) = (maj.start, min.start) {
            placed[i] = Some((m0, n0));
            mark(&mut occupied, m0, n0, maj.len, min.len);
            major_extent = major_extent.max(m0 + maj.len);
            minor_extent = minor_extent.max(n0 + min.len);
        }
    }

    // Phase 2: definite major, auto minor.
    for (i, p) in placements.iter().enumerate() {
        let (maj, min) = (major_of(p), minor_of(p));
        if placed[i].is_none() && maj.start.is_some() {
            let m0 = maj.start.unwrap_or(0);
            minor_extent = minor_extent.max(min.len);
            let limit = minor_extent.saturating_sub(min.len);
            // Columns at or beyond `minor_extent` are always free, so that is the fallback.
            let n0 = (0..=limit)
                .find(|&n| free(&occupied, m0, n, maj.len, min.len))
                .unwrap_or(minor_extent);
            placed[i] = Some((m0, n0));
            mark(&mut occupied, m0, n0, maj.len, min.len);
            major_extent = major_extent.max(m0 + maj.len);
            minor_extent = minor_extent.max(n0 + min.len);
        }
    }

    // Phase 3 and 4: auto major. A definite minor start is kept; otherwise the cursor walks.
    let mut cursor = (0usize, 0usize);
    for (i, p) in placements.iter().enumerate() {
        if placed[i].is_some() {
            continue;
        }
        let (maj, min) = (major_of(p), minor_of(p));
        minor_extent = minor_extent.max(min.len);
        let fixed_minor = min.start;
        let (start_major, start_minor) = if dense { (0, 0) } else { cursor };
        let mut m = start_major;
        let (m0, n0) = loop {
            let begin = if m == start_major { start_minor } else { 0 };
            let candidates: Vec<usize> = match fixed_minor {
                Some(n) => alloc::vec![n],
                None => (begin..=minor_extent.saturating_sub(min.len)).collect(),
            };
            if let Some(n) = candidates
                .into_iter()
                .find(|&n| free(&occupied, m, n, maj.len, min.len))
            {
                break (m, n);
            }
            m += 1;
        };
        placed[i] = Some((m0, n0));
        mark(&mut occupied, m0, n0, maj.len, min.len);
        major_extent = major_extent.max(m0 + maj.len);
        minor_extent = minor_extent.max(n0 + min.len);
        cursor = (m0, n0 + min.len);
    }

    // Convert back to physical (row, column) and report occupied extents in physical order.
    let physical: Vec<(usize, usize)> = placed
        .into_iter()
        .map(|p| {
            let (m, n) = p.unwrap_or((0, 0));
            if column_flow {
                (n, m)
            } else {
                (m, n)
            }
        })
        .collect();
    let (rows, cols) = if column_flow {
        (minor_extent, major_extent)
    } else {
        (major_extent, minor_extent)
    };
    (physical, rows, cols)
}

/// Sizes one axis of tracks. `content` holds each track's content contribution.
fn size_tracks(
    template: &Tracks,
    auto: TrackSize,
    count: usize,
    base: Option<Px>,
    track_gap: Px,
    content: &[Px],
) -> Vec<Px> {
    let mut sizes = alloc::vec![Px::ZERO; count];
    let mut fr = alloc::vec![0i64; count];
    for (i, size) in sizes.iter_mut().enumerate() {
        let spec = if i < template.len as usize {
            template.items[i]
        } else {
            auto
        };
        let content_size = content.get(i).copied().unwrap_or(Px::ZERO);
        *size = match spec {
            TrackSize::Fixed(Dimension::Length(v)) => v,
            TrackSize::Fixed(d) => d.resolve(base).unwrap_or(content_size),
            TrackSize::Auto => content_size,
            TrackSize::Fr(f) => {
                fr[i] = f.max(0) as i64;
                Px::ZERO
            }
            TrackSize::MinMax { min, max } => {
                let lo = match min {
                    Dimension::Auto => Px::ZERO,
                    d => d.resolve(base).unwrap_or(Px::ZERO),
                };
                let hi = match max {
                    Dimension::Auto => None,
                    d => d.resolve(base),
                };
                content_size.max(lo).clamp_to(lo, hi)
            }
        };
    }
    let total_fr: i64 = fr.iter().sum();
    if total_fr > 0 {
        let fixed_total = sizes
            .iter()
            .zip(&fr)
            .filter(|(_, f)| **f == 0)
            .fold(Px::ZERO, |acc, (s, _)| acc + *s);
        let gaps = track_gap * (count as i32 - 1).max(0);
        let leftover = match base {
            Some(base) => (base - fixed_total - gaps).max(Px::ZERO),
            // Indefinite container: fr tracks size to their content.
            None => Px::ZERO,
        };
        for (i, size) in sizes.iter_mut().enumerate() {
            if fr[i] > 0 {
                *size = match base {
                    Some(_) => leftover.mul_ratio(fr[i], total_fr.max(100)),
                    None => content.get(i).copied().unwrap_or(Px::ZERO),
                };
            }
        }
    }
    sizes
}

/// Start offset of each track, including gaps before it.
fn starts(sizes: &[Px], track_gap: Px) -> Vec<Px> {
    let mut out = Vec::with_capacity(sizes.len());
    let mut cursor = Px::ZERO;
    for size in sizes {
        out.push(cursor);
        cursor = cursor + *size + track_gap;
    }
    out
}

/// Extent of `len` tracks starting at `start`, including the gaps between them.
fn span_extent(sizes: &[Px], start: usize, len: usize, track_gap: Px) -> Px {
    let end = (start + len).min(sizes.len());
    let total = sizes[start.min(end)..end]
        .iter()
        .fold(Px::ZERO, |acc, s| acc + *s);
    total + track_gap * (end.saturating_sub(start).max(1) as i32 - 1)
}

/// Offset of an outer box of size `outer` inside a cell of size `cell`.
fn align_offset(align: AlignItems, cell: Px, outer: Px) -> Px {
    match align {
        AlignItems::FlexEnd => (cell - outer).max(Px::ZERO),
        AlignItems::Center => ((cell - outer) / 2).max(Px::ZERO),
        _ => Px::ZERO,
    }
}
