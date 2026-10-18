//! Inline formatting context (TODO Phase 3): line boxes, word wrapping, inline-blocks, baseline
//! alignment, and float-aware line widths.
//!
//! A run is a sequence of consecutive inline-level children of a block container. Text is split
//! into words and spaces (measured by the [`TextMeasurer`]), and each inline-block is laid out
//! once as an atomic box. Pieces are then placed left to right, wrapping when a piece would pass
//! the right edge of the line. Each line is as tall as its tallest piece, and pieces sit on a
//! shared baseline.
//!
//! Simplifications: `text-align` is always start. Inline elements (`Display::Inline` with
//! children) contribute their children's pieces without their own padding, border or margin.
//! A line's width is decided from the floats that overlap its top edge only.

use alloc::vec::Vec;

use crate::{
    flow::{layout_detached, shift_deferred, Containing, Cursor, Floats, Pass, Slot},
    intrinsic,
    position::Deferred,
    px::Px,
    sizing::Sides,
    style::{Display, Position},
    tree::{BoxTree, Fragment, Frame, NodeId},
};

/// A measured piece of an inline run, before placement.
enum Piece {
    Word {
        node: NodeId,
        range: core::ops::Range<usize>,
        width: Px,
        height: Px,
        ascent: Px,
    },
    Space {
        width: Px,
        height: Px,
        ascent: Px,
    },
    Atomic {
        node: NodeId,
        margin_left: Px,
        margin_top: Px,
        outer_width: Px,
        outer_height: Px,
        baseline: Px,
        /// Deferred entries this box appended while it was laid out.
        mark: core::ops::Range<usize>,
    },
}

impl Piece {
    fn width(&self) -> Px {
        match self {
            Piece::Word { width, .. } | Piece::Space { width, .. } => *width,
            Piece::Atomic { outer_width, .. } => *outer_width,
        }
    }

    fn ascent(&self) -> Px {
        match self {
            Piece::Word { ascent, .. } | Piece::Space { ascent, .. } => *ascent,
            Piece::Atomic {
                margin_top,
                baseline,
                ..
            } => *margin_top + *baseline,
        }
    }

    fn height(&self) -> Px {
        match self {
            Piece::Word { height, .. } | Piece::Space { height, .. } => *height,
            Piece::Atomic { outer_height, .. } => *outer_height,
        }
    }
}

/// Pieces and inline elements gathered from one run, with the context needed to place them.
struct Gather {
    content: Containing,
    /// Y of the run's start, used as the static position of absolutely positioned pieces.
    static_y: Px,
    pieces: Vec<Piece>,
    elements: Vec<NodeId>,
}

/// A piece placed on a line, with its x offset from the line's left edge.
struct Placed {
    piece: usize,
    x: Px,
}

struct Line {
    top: Px,
    left: Px,
    items: Vec<Placed>,
    /// Largest distance from the top of a piece to its baseline.
    ascent: Px,
    /// Largest distance from the baseline to the bottom of a piece.
    descent: Px,
}

impl Line {
    fn new(top: Px, left: Px) -> Self {
        Line {
            top,
            left,
            items: Vec::new(),
            ascent: Px::ZERO,
            descent: Px::ZERO,
        }
    }

    fn height(&self) -> Px {
        self.ascent + self.descent
    }
}

/// Lays out one inline run at the cursor. Advances the cursor past the last line. Returns the
/// absolute baseline of the first line, if there is one.
pub(crate) fn layout_run(
    pass: &Pass,
    tree: &mut BoxTree,
    run: &[NodeId],
    content: Containing,
    cursor: &mut Cursor,
    floats: &mut Floats,
    deferred: &mut Vec<Deferred>,
) -> Option<Px> {
    let mut gather = Gather {
        content,
        static_y: cursor.y,
        pieces: Vec::new(),
        elements: Vec::new(),
    };
    for node in run {
        flatten(pass, tree, *node, &mut gather, deferred);
    }
    let Gather {
        pieces, elements, ..
    } = gather;
    if pieces.is_empty() {
        settle_elements(tree, &elements, (content.x, cursor.y));
        return None;
    }

    let right_edge = content.x + content.width;
    let bounds = |floats: &Floats, top: Px| -> (Px, Px) {
        let bottom = top + Px(1);
        (
            floats.left_edge(top, bottom, content.x),
            floats.right_edge(top, bottom, right_edge),
        )
    };

    // Break pieces into lines.
    let mut lines: Vec<Line> = Vec::new();
    let mut top = cursor.y;
    let (mut left, mut right) = bounds(floats, top);
    let mut current = Line::new(top, left);
    let mut x = Px::ZERO;
    for (index, piece) in pieces.iter().enumerate() {
        let is_space = matches!(piece, Piece::Space { .. });
        if current.items.is_empty() && is_space {
            continue;
        }
        let width = piece.width();
        if !current.items.is_empty() && left + x + width > right {
            let finished = core::mem::replace(&mut current, Line::new(Px::ZERO, Px::ZERO));
            top += finished.height();
            lines.push(finished);
            let (next_left, next_right) = bounds(floats, top);
            left = next_left;
            right = next_right;
            current = Line::new(top, left);
            x = Px::ZERO;
            if is_space {
                continue;
            }
        }
        current.items.push(Placed { piece: index, x });
        x += width;
        current.ascent = current.ascent.max(piece.ascent());
        current.descent = current.descent.max(piece.height() - piece.ascent());
    }
    if !current.items.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        return None;
    }

    // Place pieces on their baselines.
    let mut fragments: Vec<(NodeId, Vec<Fragment>)> = Vec::new();
    let mut first_baseline = None;
    for line in &lines {
        let baseline_y = line.top + line.ascent;
        if first_baseline.is_none() {
            first_baseline = Some(baseline_y);
        }
        for placed in &line.items {
            let piece = &pieces[placed.piece];
            let piece_top = baseline_y - piece.ascent();
            let piece_x = line.left + placed.x;
            match piece {
                Piece::Word {
                    node,
                    range,
                    width,
                    height,
                    ..
                } => {
                    push_fragment(
                        &mut fragments,
                        *node,
                        Fragment {
                            x: piece_x,
                            y: piece_top,
                            width: *width,
                            height: *height,
                            text_range: range.clone(),
                        },
                    );
                }
                Piece::Space { .. } => {}
                Piece::Atomic {
                    node,
                    margin_left,
                    margin_top,
                    mark,
                    ..
                } => {
                    let dx = piece_x + *margin_left;
                    let dy = piece_top + *margin_top;
                    tree.shift_subtree(*node, dx, dy);
                    shift_deferred(deferred, mark.clone(), dx, dy);
                }
            }
        }
    }
    for (node, list) in fragments {
        // An inline text node's frame is the bounding box of its fragments.
        // Folds start from the extremes, not zero: fragments can lie at negative coordinates.
        let left = list.iter().map(|f| f.x).fold(Px::MAX, |acc, x| acc.min(x));
        let top = list.iter().map(|f| f.y).fold(Px::MAX, |acc, y| acc.min(y));
        let right = list
            .iter()
            .map(|f| f.x + f.width)
            .fold(Px(i32::MIN), |acc, x| acc.max(x));
        let bottom = list
            .iter()
            .map(|f| f.y + f.height)
            .fold(Px(i32::MIN), |acc, y| acc.max(y));
        tree.set_frame(
            node,
            Frame {
                x: left,
                y: top,
                width: right - left,
                height: bottom - top,
                baseline: Px::ZERO,
            },
        );
        tree.set_fragments(node, list);
    }

    let origin = lines
        .first()
        .map_or((content.x, cursor.y), |l| (l.left, l.top));
    settle_elements(tree, &elements, origin);

    let last = lines.last().map_or(cursor.y, |l| l.top + l.height());
    cursor.y = last;
    cursor.pending = crate::flow::Margin::ZERO;
    first_baseline
}

/// Sets the frame of each inline element from its children: the union of their frames, or a
/// zero-size frame at `origin` when it has none. `elements` lists outer elements before inner
/// ones, so walking backwards settles inner elements first.
fn settle_elements(tree: &mut BoxTree, elements: &[NodeId], origin: (Px, Px)) {
    for element in elements.iter().rev() {
        let mut bounds: Option<(Px, Px, Px, Px)> = None;
        for child in tree.children(*element).to_vec() {
            let Some(node) = tree.get(child) else {
                continue;
            };
            // Absolutely positioned children are placed later, by their containing block, and are
            // not part of the inline box.
            if !node.laid_out || node.style.position == Position::Absolute {
                continue;
            }
            let f = node.frame;
            bounds = Some(match bounds {
                None => (f.x, f.y, f.x + f.width, f.y + f.height),
                Some((l, t, r, b)) => (
                    l.min(f.x),
                    t.min(f.y),
                    r.max(f.x + f.width),
                    b.max(f.y + f.height),
                ),
            });
        }
        // The origin is taken from the run's own coordinates, not the document origin, so the
        // frame moves with the run when an ancestor shifts it.
        let frame = match bounds {
            Some((l, t, r, b)) => Frame {
                x: l,
                y: t,
                width: r - l,
                height: b - t,
                baseline: Px::ZERO,
            },
            None => Frame {
                x: origin.0,
                y: origin.1,
                width: Px::ZERO,
                height: Px::ZERO,
                baseline: Px::ZERO,
            },
        };
        tree.set_frame(*element, frame);
    }
}

fn push_fragment(fragments: &mut Vec<(NodeId, Vec<Fragment>)>, node: NodeId, fragment: Fragment) {
    match fragments.iter_mut().find(|(id, _)| *id == node) {
        Some((_, list)) => list.push(fragment),
        None => fragments.push((node, alloc::vec![fragment])),
    }
}

/// Appends the pieces for one inline-level node: words for text, recursion for inline elements,
/// and an atomic piece for inline-blocks.
fn flatten(
    pass: &Pass,
    tree: &mut BoxTree,
    node: NodeId,
    gather: &mut Gather,
    deferred: &mut Vec<Deferred>,
) {
    let Some(entry) = tree.get(node) else { return };
    let style = entry.style;
    let content = gather.content;
    if let Some(text) = entry.text.clone() {
        split_text(pass, node, &text, &mut gather.pieces);
        return;
    }
    if style.display == Display::Inline && !entry.replaced {
        // An inline element has no box of its own. Its frame is set after placement.
        gather.elements.push(node);
        for child in tree.children(node).to_vec() {
            flatten(pass, tree, child, gather, deferred);
        }
        return;
    }
    if style.position == Position::Absolute {
        deferred.push(Deferred {
            id: node,
            static_x: content.x,
            static_y: gather.static_y,
        });
        return;
    }

    // Anything else is an atomic box: an inline-block, or a block-level box that sits inside an
    // inline element. Such a box is laid out once at its shrink-to-fit width.
    let sides = Sides::of(&style, content.width);
    let content_width = intrinsic::shrink_to_fit(pass, tree, node, content.width);
    let mark = deferred.len();
    let Some(placed) = layout_detached(
        pass,
        tree,
        node,
        Slot {
            x: Px::ZERO,
            y: Px::ZERO,
            content_width,
            cb_width: content.width,
            cb_height: content.height,
            height: None,
        },
        deferred,
    ) else {
        return;
    };
    let border_width = content_width + sides.horizontal_extra();
    gather.pieces.push(Piece::Atomic {
        node,
        margin_left: sides.margin.left,
        margin_top: sides.margin.top,
        outer_width: border_width + sides.margin.horizontal(),
        outer_height: placed.border_height + sides.margin.vertical(),
        baseline: placed.baseline,
        mark: mark..deferred.len(),
    });
}

/// Splits text into word and space pieces, measured with the measurer.
fn split_text(pass: &Pass, node: NodeId, text: &str, pieces: &mut Vec<Piece>) {
    let mut start = 0;
    let mut in_space: Option<bool> = None;
    let flush = |pieces: &mut Vec<Piece>, from: usize, to: usize, space: bool| {
        if from == to {
            return;
        }
        let segment = &text[from..to];
        let measured = pass.measurer.line_boxes(segment, Px::MAX);
        let (width, height, ascent) =
            measured
                .first()
                .map_or((Px::ZERO, Px::ZERO, Px::ZERO), |line| {
                    let total = measured.iter().fold(Px::ZERO, |acc, l| acc.max(l.width));
                    (total, line.height, line.baseline)
                });
        if space {
            pieces.push(Piece::Space {
                width,
                height,
                ascent,
            });
        } else {
            pieces.push(Piece::Word {
                node,
                range: from..to,
                width,
                height,
                ascent,
            });
        }
    };
    for (index, ch) in text.char_indices() {
        let space = ch.is_whitespace();
        match in_space {
            None => in_space = Some(space),
            Some(current) if current != space => {
                flush(pieces, start, index, current);
                start = index;
                in_space = Some(space);
            }
            Some(_) => {}
        }
    }
    if let Some(current) = in_space {
        flush(pieces, start, text.len(), current);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_text_alternates_words_and_spaces() {
        use crate::{style::BoxStyle, text::FixedAdvanceMeasurer};
        let measurer = FixedAdvanceMeasurer::new(Px::from_px(10), Px::from_px(20), Px::from_px(15));
        let mut tree = BoxTree::new();
        let node = tree.insert_text(BoxStyle::default(), "ab  c");
        let pass = Pass::new(&measurer, node);
        let mut pieces = Vec::new();
        split_text(&pass, node, "ab  c", &mut pieces);
        let kinds: Vec<&str> = pieces
            .iter()
            .map(|p| match p {
                Piece::Word { .. } => "w",
                Piece::Space { .. } => "s",
                Piece::Atomic { .. } => "a",
            })
            .collect();
        assert_eq!(kinds, ["w", "s", "w"]);
        assert_eq!(pieces[0].width(), Px::from_px(20));
        assert_eq!(pieces[1].width(), Px::from_px(20));
    }
}
