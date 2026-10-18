//! Flexbox layout (TODO Phase 2).
//!
//! Algorithm, in the order the spec prescribes (no step iterates to convergence):
//! 1. Collect in-flow items and sort them by `order` (stable).
//! 2. Resolve each item's flex base size and clamp range.
//! 3. Break items into lines (`wrap`) against the container's main size.
//! 4. Per line, resolve flexible lengths with the bounded freeze loop in [`resolve_line`]. Each
//!    round freezes at least one item or stops, so it runs at most `items + 1` rounds.
//! 5. Place items along the main axis (`justify-content`, auto margins).
//! 6. Lay each item out once at its final main position, then align on the cross axis and shift
//!    it into place.
//!
//! Simplifications, documented in TODO.md where they matter:
//! - `min-width: auto` is treated as `0`, not as the min-content size.
//! - Column items are measured once (to get their hypothetical height) and laid out again at
//!   their final size. Row items are laid out once.
//! - `align-items: stretch` on a column item uses the container's width, even for wrapped lines.
//! - Auto margins apply on the main axis for row containers only.

use alloc::vec::Vec;

use crate::{
    flow::{layout_detached, shift_deferred, Containing, Margin, Pass, Slot},
    intrinsic,
    position::Deferred,
    px::Px,
    sizing::{clamp_inline, to_content, Sides},
    style::{
        AlignContent, AlignItems, Dimension, Direction, FlexDirection, FlexWrap, JustifyContent,
        Position,
    },
    tree::{BoxTree, NodeId},
};

/// One in-flow flex item and its resolved sizes. Sizes are content-box lengths in the main
/// axis unless noted.
struct Item {
    id: NodeId,
    sides: Sides,
    grow: i64,
    shrink: i64,
    base: Px,
    min: Px,
    max: Option<Px>,
    /// Resolved main content size.
    size: Px,
    /// Cross content size, known before layout for column containers.
    cross_content: Option<Px>,
    align: AlignItems,
    /// Main-axis margins, with auto margins replaced by their share of free space.
    main_margin_start: Px,
    main_margin_end: Px,
    /// Border-box start along the main axis, relative to the content start (physical order).
    main_border_start: Px,
    /// Filled in during cross layout.
    cross_border: Px,
    baseline: Px,
    provisional: (Px, Px),
}

/// Lays out the in-flow children of flex container `id`. Returns the container's content
/// bottom, a zero trailing margin, and the first item's baseline.
pub(crate) fn layout_flex(
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
    let row = matches!(
        style.flex_direction,
        FlexDirection::Row | FlexDirection::RowReverse
    );
    let reverse = matches!(
        style.flex_direction,
        FlexDirection::RowReverse | FlexDirection::ColumnReverse
    );
    // In a right-to-left row, the main axis starts on the right.
    let main_reverse = if row {
        reverse != (style.direction == Direction::Rtl)
    } else {
        reverse
    };
    let wrap = style.flex_wrap != FlexWrap::NoWrap;
    let wrap_reverse = style.flex_wrap == FlexWrap::WrapReverse;

    let column_gap = resolve_gap(style.column_gap, content.width);
    let row_gap = resolve_gap(style.row_gap, content.width);
    let (gap_main, gap_cross) = if row {
        (column_gap, row_gap)
    } else {
        (row_gap, column_gap)
    };
    let main_inner: Option<Px> = if row {
        Some(content.width)
    } else {
        content.height
    };
    let cross_inner: Option<Px> = if row {
        content.height
    } else {
        Some(content.width)
    };

    let mut entries: Vec<(i32, NodeId)> = Vec::new();
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
        entries.push((child_node.style.order, child));
    }
    entries.sort_by_key(|entry| entry.0);

    let mut items = Vec::with_capacity(entries.len());
    for (_, child) in entries {
        items.push(make_item(
            pass,
            tree,
            child,
            row,
            content,
            style.align_items,
        ));
    }
    if items.is_empty() {
        let height = content.height.unwrap_or(Px::ZERO);
        return (top + height, Margin::ZERO, None);
    }

    let lines = break_lines(&items, row, wrap, main_inner, gap_main);

    // Step 4 and 5: flexible lengths and main-axis placement, per line.
    for line in &lines {
        resolve_line(&mut items, line, row, main_inner, gap_main);
        place_main(
            &mut items,
            line,
            row,
            main_inner,
            gap_main,
            style.justify_content,
            main_reverse,
        );
    }

    // Step 6a: lay out each item once at its provisional position.
    let mut marks = Vec::with_capacity(items.len());
    for item in items.iter_mut() {
        let (x, y) = provisional_origin(item, row, content, top);
        item.provisional = (x, y);
        let content_width = if row {
            item.size
        } else {
            item.cross_content.unwrap_or(Px::ZERO)
        };
        let height = if row { None } else { Some(item.size) };
        let mark = deferred.len();
        let placed = layout_detached(
            pass,
            tree,
            item.id,
            Slot {
                x,
                y,
                content_width,
                cb_width: content.width,
                cb_height: content.height,
                height,
            },
            deferred,
        );
        marks.push(mark..deferred.len());
        if let Some(placed) = placed {
            item.baseline = placed.baseline;
            item.cross_border = if row {
                placed.border_height
            } else {
                content_width + item.sides.horizontal_extra()
            };
        }
    }

    // Step 6b: cross sizes per line, then align-content.
    let line_cross = line_cross_sizes(&items, &lines, row, cross_inner);
    let (line_starts, line_sizes) =
        align_lines(&line_cross, cross_inner, gap_cross, style.align_content);

    // Step 6c: cross-axis alignment and final shift. With wrap-reverse, lines stack from the far
    // edge, so the logical line offsets are mirrored.
    let total_cross = cross_inner.unwrap_or_else(|| {
        let used = line_sizes.iter().fold(Px::ZERO, |acc, size| acc + *size);
        used + gap_cross * (line_sizes.len() as i32 - 1).max(0)
    });
    for (line_index, line) in lines.iter().enumerate() {
        let size = line_sizes[line_index];
        let line_start = if wrap_reverse {
            total_cross - (line_starts[line_index] + size)
        } else {
            line_starts[line_index]
        };
        for &index in line {
            let item = &items[index];
            let outer_cross = item_outer_cross(item, row);
            let align = item.align;
            let flip = wrap_reverse;
            let offset = match align {
                AlignItems::FlexStart | AlignItems::Stretch => Px::ZERO,
                AlignItems::FlexEnd => size - outer_cross,
                AlignItems::Center => (size - outer_cross) / 2,
                AlignItems::Baseline if row => {
                    line_cross[line_index].1 - baseline_ascent(item, row)
                }
                AlignItems::Baseline => Px::ZERO,
            };
            let outer_start = line_start
                + if flip {
                    size - offset - outer_cross
                } else {
                    offset
                };
            let cross_margin_start = cross_margin(item, row, true);
            let border_cross_start = outer_start + cross_margin_start;

            let (final_x, final_y) = if row {
                let (px, _) = item.provisional;
                (px, top + border_cross_start)
            } else {
                let (_, py) = item.provisional;
                (content.x + border_cross_start, py)
            };
            let (prov_x, prov_y) = item.provisional;
            let dx = final_x - prov_x;
            let dy = final_y - prov_y;
            if !dx.is_zero() || !dy.is_zero() {
                tree.shift_subtree(item.id, dx, dy);
                shift_deferred(deferred, marks[index].clone(), dx, dy);
            }

            // Stretch fills the line's cross size. Only the frame changes, not the content.
            if row
                && align == AlignItems::Stretch
                && tree
                    .get(item.id)
                    .is_some_and(|n| n.style.height == Dimension::Auto)
            {
                let stretched =
                    size - cross_margin(item, row, true) - cross_margin(item, row, false);
                if let Some(frame) = tree.frame(item.id) {
                    let mut frame = frame;
                    frame.height = stretched.max(Px::ZERO);
                    tree.set_frame(item.id, frame);
                }
            }
        }
    }

    // Container content size.
    let content_height = match (row, content.height) {
        (_, Some(height)) => height,
        (true, None) => {
            let total = line_cross
                .iter()
                .fold(Px::ZERO, |acc, (size, _)| acc + *size);
            total + gap_cross * (line_cross.len() as i32 - 1).max(0)
        }
        (false, None) => lines
            .iter()
            .map(|line| main_total(&items, line, gap_main))
            .fold(Px::ZERO, |acc, total| acc.max(total)),
    };

    let baseline = if row {
        items.first().map(|item| {
            let frame_top = tree.frame(item.id).map_or(Px::ZERO, |f| f.y);
            frame_top + item.baseline
        })
    } else {
        None
    };
    (top + content_height, Margin::ZERO, baseline)
}

fn resolve_gap(gap: Dimension, base: Px) -> Px {
    gap.resolve(Some(base)).unwrap_or(Px::ZERO).max(Px::ZERO)
}

fn make_item(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    row: bool,
    content: Containing,
    container_align: AlignItems,
) -> Item {
    let style = tree.get(id).map(|n| n.style).unwrap_or_default();
    let sides = Sides::of(&style, content.width);
    let main_extra = if row {
        sides.horizontal_extra()
    } else {
        sides.vertical_extra()
    };
    let cb_main = if row {
        Some(content.width)
    } else {
        content.height
    };
    let align = style.align_self.unwrap_or(container_align);

    // Cross size of a column item is decided before measuring its height.
    let cross_content = if row {
        None
    } else {
        let cross_extra = sides.horizontal_extra();
        let width = match style.width.resolve(Some(content.width)) {
            Some(w) => clamp_inline(
                &style,
                content.width,
                cross_extra,
                to_content(&style, w, cross_extra),
            ),
            None if align == AlignItems::Stretch => {
                let available = content.width - cross_extra - sides.margin.horizontal();
                clamp_inline(&style, content.width, cross_extra, available.max(Px::ZERO))
            }
            None => intrinsic::shrink_to_fit(pass, tree, id, content.width),
        };
        Some(width)
    };

    let base = match style.flex_basis.resolve(cb_main) {
        Some(v) => to_content(&style, v, main_extra),
        None if row => match style.width.resolve(Some(content.width)) {
            Some(w) => to_content(&style, w, main_extra),
            None => intrinsic::max_content(pass, tree, id),
        },
        None => match style.height.resolve(content.height) {
            Some(h) => to_content(&style, h, main_extra),
            None => measure_height(
                pass,
                tree,
                id,
                content,
                cross_content.unwrap_or(Px::ZERO),
                main_extra,
            ),
        },
    };

    let (min, max) = if row {
        (
            style
                .min_width
                .resolve(Some(content.width))
                .map_or(Px::ZERO, |v| to_content(&style, v, main_extra)),
            style
                .max_width
                .resolve(Some(content.width))
                .map(|v| to_content(&style, v, main_extra)),
        )
    } else {
        (
            style
                .min_height
                .resolve(content.height)
                .map_or(Px::ZERO, |v| to_content(&style, v, main_extra)),
            style
                .max_height
                .resolve(content.height)
                .map(|v| to_content(&style, v, main_extra)),
        )
    };

    Item {
        id,
        sides,
        grow: (style.flex_grow.max(0.0) * 100.0 + 0.5) as i64,
        shrink: (style.flex_shrink.max(0.0) * 100.0 + 0.5) as i64,
        base,
        min,
        max,
        size: base,
        cross_content,
        align,
        main_margin_start: Px::ZERO,
        main_margin_end: Px::ZERO,
        main_border_start: Px::ZERO,
        cross_border: Px::ZERO,
        baseline: Px::ZERO,
        provisional: (Px::ZERO, Px::ZERO),
    }
}

/// Lays out a column item at the given cross width to find its natural height.
fn measure_height(
    pass: &Pass,
    tree: &mut BoxTree,
    id: NodeId,
    content: Containing,
    cross_content: Px,
    main_extra: Px,
) -> Px {
    let mut scratch = Vec::new();
    let placed = layout_detached(
        pass,
        tree,
        id,
        Slot {
            x: Px::ZERO,
            y: Px::ZERO,
            content_width: cross_content,
            cb_width: content.width,
            cb_height: content.height,
            height: None,
        },
        &mut scratch,
    );
    match placed {
        Some(placed) => (placed.border_height - main_extra).max(Px::ZERO),
        None => Px::ZERO,
    }
}

/// Splits sorted items into lines. With `wrap`, a new line starts when the next item would
/// overflow a definite main size.
fn break_lines(
    items: &[Item],
    row: bool,
    wrap: bool,
    main_inner: Option<Px>,
    gap: Px,
) -> Vec<Vec<usize>> {
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut used = Px::ZERO;
    for (index, item) in items.iter().enumerate() {
        let outer = hypothetical_outer(item, row);
        if let (true, Some(inner), false) = (wrap, main_inner, current.is_empty()) {
            if used + gap + outer > inner {
                lines.push(core::mem::take(&mut current));
                used = Px::ZERO;
            }
        }
        if !current.is_empty() {
            used += gap;
        }
        used += outer;
        current.push(index);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Main-axis outer size of an item at its clamped base size.
fn hypothetical_outer(item: &Item, row: bool) -> Px {
    let clamped = item.base.clamp_to(item.min, item.max);
    clamped
        + main_extra(item, row)
        + main_margin_fixed(item, row, true)
        + main_margin_fixed(item, row, false)
}

fn main_extra(item: &Item, row: bool) -> Px {
    if row {
        item.sides.horizontal_extra()
    } else {
        item.sides.vertical_extra()
    }
}

/// Margin on the main axis, excluding auto margins (which are resolved in `place_main`).
fn main_margin_fixed(item: &Item, row: bool, start: bool) -> Px {
    match (row, start) {
        (true, true) => item.sides.margin.left,
        (true, false) => item.sides.margin.right,
        (false, true) => item.sides.margin.top,
        (false, false) => item.sides.margin.bottom,
    }
}

fn cross_margin(item: &Item, row: bool, start: bool) -> Px {
    match (row, start) {
        (true, true) => item.sides.margin.top,
        (true, false) => item.sides.margin.bottom,
        (false, true) => item.sides.margin.left,
        (false, false) => item.sides.margin.right,
    }
}

fn item_outer_cross(item: &Item, row: bool) -> Px {
    item.cross_border + cross_margin(item, row, true) + cross_margin(item, row, false)
}

/// Ascent of a baseline-aligned item from its outer cross-start edge.
fn baseline_ascent(item: &Item, row: bool) -> Px {
    cross_margin(item, row, true) + item.baseline
}

/// Resolves flexible lengths for one line with the bounded freeze loop.
fn resolve_line(items: &mut [Item], line: &[usize], row: bool, main_inner: Option<Px>, gap: Px) {
    let inner = match main_inner {
        Some(inner) => inner,
        // Indefinite main size (auto-height column): no free space to distribute.
        None => {
            let total = line
                .iter()
                .fold(Px::ZERO, |acc, &i| acc + hypothetical_outer(&items[i], row));
            total + gap * (line.len() as i32 - 1).max(0)
        }
    };
    let gaps = gap * (line.len() as i32 - 1).max(0);
    let fixed_extra = |item: &Item| {
        main_extra(item, row)
            + main_margin_fixed(item, row, true)
            + main_margin_fixed(item, row, false)
    };

    for &i in line {
        let item = &mut items[i];
        item.size = item.base.clamp_to(item.min, item.max);
    }

    let mut frozen = alloc::vec![false; line.len()];
    for _round in 0..=line.len() {
        let mut used = gaps;
        for (k, &i) in line.iter().enumerate() {
            let item = &items[i];
            let size = if frozen[k] { item.size } else { item.base };
            used += size + fixed_extra(item);
        }
        let free = inner - used;
        if free.is_zero() {
            break;
        }

        let unfrozen: Vec<usize> = (0..line.len()).filter(|k| !frozen[*k]).collect();
        if unfrozen.is_empty() {
            break;
        }

        // Each unfrozen item's target size: base plus its share of the free space.
        let shares: Vec<i64> = if free > Px::ZERO {
            let total: i64 = unfrozen.iter().map(|&k| items[line[k]].grow).sum();
            if total == 0 {
                break;
            }
            unfrozen.iter().map(|&k| items[line[k]].grow).collect()
        } else {
            let total: i64 = unfrozen
                .iter()
                .map(|&k| items[line[k]].shrink * items[line[k]].base.0.max(0) as i64)
                .sum();
            if total == 0 {
                break;
            }
            unfrozen
                .iter()
                .map(|&k| items[line[k]].shrink * items[line[k]].base.0.max(0) as i64)
                .collect()
        };
        let total: i64 = shares.iter().sum();

        let mut violations = false;
        let mut targets = Vec::with_capacity(unfrozen.len());
        for (position, &k) in unfrozen.iter().enumerate() {
            let item = &items[line[k]];
            let share = free.mul_ratio(shares[position], total);
            let target = item.base + share;
            let clamped = target.clamp_to(item.min, item.max);
            if clamped != target {
                violations = true;
            }
            targets.push((k, target, clamped));
        }

        if !violations {
            for (k, target, _) in &targets {
                items[line[*k]].size = *target;
            }
            break;
        }
        for (k, _, clamped) in &targets {
            items[line[*k]].size = *clamped;
        }
        for (k, target, clamped) in targets {
            if clamped != target {
                frozen[k] = true;
                items[line[k]].size = clamped;
            }
        }
    }
}

/// Places items along the main axis of one line. Sets `main_border_start` (physical, from the
/// content start) and the resolved margins.
fn place_main(
    items: &mut [Item],
    line: &[usize],
    row: bool,
    main_inner: Option<Px>,
    gap: Px,
    justify: JustifyContent,
    main_reverse: bool,
) {
    let count = line.len() as i32;
    let gaps = gap * (count - 1).max(0);
    let total: Px = line.iter().fold(gaps, |acc, &i| {
        let item = &items[i];
        acc + item.size
            + main_extra(item, row)
            + main_margin_fixed(item, row, true)
            + main_margin_fixed(item, row, false)
    });
    let inner = main_inner.unwrap_or(total);
    let free = inner - total;

    // Auto margins (row only) take the free space before justify-content applies.
    let auto_count = if row {
        line.iter()
            .map(|&i| items[i].sides.auto_left as i32 + items[i].sides.auto_right as i32)
            .sum::<i32>()
    } else {
        0
    };
    let auto_share = if auto_count > 0 && free > Px::ZERO {
        free / auto_count
    } else {
        Px::ZERO
    };
    let free_for_justify = if auto_count > 0 && free > Px::ZERO {
        Px::ZERO
    } else {
        free
    };

    let (start, between) = justify_offsets(justify, free_for_justify, count);

    let mut cursor = start;
    for &i in line {
        let item = &mut items[i];
        let auto_start = if row && item.sides.auto_left {
            auto_share
        } else {
            Px::ZERO
        };
        let auto_end = if row && item.sides.auto_right {
            auto_share
        } else {
            Px::ZERO
        };
        item.main_margin_start = main_margin_fixed(item, row, true) + auto_start;
        item.main_margin_end = main_margin_fixed(item, row, false) + auto_end;
        let border = item.size + main_extra(item, row);
        let logical_start = cursor + item.main_margin_start;
        item.main_border_start = if main_reverse {
            inner - (logical_start + border)
        } else {
            logical_start
        };
        cursor = cursor + item.main_margin_start + border + item.main_margin_end + gap + between;
    }
}

fn justify_offsets(justify: JustifyContent, free: Px, count: i32) -> (Px, Px) {
    let spacing_fallback = free < Px::ZERO;
    match justify {
        JustifyContent::FlexStart => (Px::ZERO, Px::ZERO),
        JustifyContent::FlexEnd => (free, Px::ZERO),
        JustifyContent::Center => (free / 2, Px::ZERO),
        JustifyContent::SpaceBetween if spacing_fallback || count < 2 => (Px::ZERO, Px::ZERO),
        JustifyContent::SpaceBetween => (Px::ZERO, free / (count - 1)),
        JustifyContent::SpaceAround if spacing_fallback || count < 1 => (Px::ZERO, Px::ZERO),
        JustifyContent::SpaceAround => {
            let between = free / count;
            (between / 2, between)
        }
        JustifyContent::SpaceEvenly if spacing_fallback => (Px::ZERO, Px::ZERO),
        JustifyContent::SpaceEvenly => {
            let between = free / (count + 1);
            (between, between)
        }
    }
}

/// Cross size and baseline ascent for each line, as `(size, max_ascent)`.
fn line_cross_sizes(
    items: &[Item],
    lines: &[Vec<usize>],
    row: bool,
    cross_inner: Option<Px>,
) -> Vec<(Px, Px)> {
    let single = lines.len() == 1;
    lines
        .iter()
        .map(|line| {
            if single {
                if let Some(inner) = cross_inner {
                    let max_ascent = baseline_max_ascent(items, line, row);
                    return (inner, max_ascent);
                }
            }
            let mut max_outer = Px::ZERO;
            let mut max_asc = Px::ZERO;
            let mut max_desc = Px::ZERO;
            let mut has_baseline = false;
            for &i in line {
                let item = &items[i];
                let outer = item_outer_cross(item, row);
                if row && item.align == AlignItems::Baseline {
                    has_baseline = true;
                    let asc = baseline_ascent(item, row);
                    max_asc = max_asc.max(asc);
                    max_desc = max_desc.max(outer - asc);
                } else {
                    max_outer = max_outer.max(outer);
                }
            }
            let size = if has_baseline {
                max_outer.max(max_asc + max_desc)
            } else {
                max_outer
            };
            (size, max_asc)
        })
        .collect()
}

fn baseline_max_ascent(items: &[Item], line: &[usize], row: bool) -> Px {
    line.iter()
        .filter(|&&i| row && items[i].align == AlignItems::Baseline)
        .fold(Px(i32::MIN), |acc, &i| {
            acc.max(baseline_ascent(&items[i], row))
        })
}

/// Start offset and (possibly stretched) size of each line along the cross axis, in logical
/// order. Wrap-reverse is applied by the caller.
fn align_lines(
    lines: &[(Px, Px)],
    cross_inner: Option<Px>,
    gap: Px,
    align: AlignContent,
) -> (Vec<Px>, Vec<Px>) {
    let count = lines.len() as i32;
    let used = lines.iter().fold(Px::ZERO, |acc, (size, _)| acc + *size) + gap * (count - 1).max(0);
    // Single-line containers have no free space to distribute: the line fills the container.
    let multi = lines.len() > 1;
    let free = match cross_inner {
        Some(inner) if multi => inner - used,
        _ => Px::ZERO,
    };
    let (start, between, stretch) = if !multi {
        (Px::ZERO, Px::ZERO, Px::ZERO)
    } else {
        match align {
            AlignContent::Stretch if free > Px::ZERO => (Px::ZERO, Px::ZERO, free / count),
            AlignContent::Stretch | AlignContent::FlexStart => (Px::ZERO, Px::ZERO, Px::ZERO),
            AlignContent::FlexEnd => (free, Px::ZERO, Px::ZERO),
            AlignContent::Center => (free / 2, Px::ZERO, Px::ZERO),
            AlignContent::SpaceBetween if free > Px::ZERO && count > 1 => {
                (Px::ZERO, free / (count - 1), Px::ZERO)
            }
            AlignContent::SpaceAround if free > Px::ZERO => {
                let each = free / count;
                (each / 2, each, Px::ZERO)
            }
            AlignContent::SpaceBetween | AlignContent::SpaceAround => {
                (Px::ZERO, Px::ZERO, Px::ZERO)
            }
        }
    };

    let mut starts = Vec::with_capacity(lines.len());
    let mut sizes = Vec::with_capacity(lines.len());
    let mut cursor = start;
    for (size, _) in lines {
        starts.push(cursor);
        sizes.push(*size + stretch);
        cursor = cursor + *size + stretch + gap + between;
    }
    (starts, sizes)
}

/// Provisional border-box origin for an item, chosen so the main-axis position is final and
/// only the cross axis is adjusted later.
fn provisional_origin(item: &Item, row: bool, content: Containing, top: Px) -> (Px, Px) {
    if row {
        (
            content.x + item.main_border_start,
            top + cross_margin(item, row, true),
        )
    } else {
        (
            content.x + cross_margin(item, row, true),
            top + item.main_border_start,
        )
    }
}

/// Sum of outer main sizes plus gaps for one line.
fn main_total(items: &[Item], line: &[usize], gap: Px) -> Px {
    let gaps = gap * (line.len() as i32 - 1).max(0);
    line.iter().fold(gaps, |acc, &i| {
        let item = &items[i];
        acc + item.size
            + main_extra(item, false)
            + main_margin_fixed(item, false, true)
            + main_margin_fixed(item, false, false)
    })
}
