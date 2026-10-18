//! Pagination and multi-column layout.
//!
//! Pagination runs on a finished layout and does not move boxes. It collects *atoms* in document
//! order (text lines, whole boxes, table rows, and break markers), fills pages with them, and
//! reports where each one landed. Tree frames stay in continuous document coordinates.
//!
//! Rules applied:
//! - `break-before` / `break-after`: `always` forces a page break. `left` and `right` also force
//!   one, and add a blank page when the parity is wrong (page 1 is a right-hand page).
//! - `break-inside: avoid`: the box is one atom and is never split.
//! - Text: a paragraph splits between lines, keeping at least `orphans` lines before the break
//!   and at least `widows` lines after it. This applies to block-level text leaves, and to blocks
//!   whose children are all inline (their inline lines are the lines that split).
//! - Oversized boxes: a box taller than a page is placed alone and overflows its page.
//! - Tables: header rows (`table_header`) repeat at the top of each page the table continues to.
//!
//! Multi-column layout lays out a container's block children at column width, then fills the
//! columns greedily to a balanced height. Children are never split across columns.
//!
//! Running headers and footers reserve space with `header_height` and `footer_height`, and
//! [`PageLayout::place_running`] adds the same header and footer nodes to every page.
//!
//! Not supported: splitting a block that mixes inline and block-level children, and per-page header
//! or footer content such as page numbers.

use alloc::{collections::VecDeque, vec::Vec};
use core::ops::Range;

use crate::{
    flow::{layout_block_children, Containing, Margin, Pass},
    position::Deferred,
    px::Px,
    sizing::Sides,
    style::{Break, Display},
    text::LineBox,
    tree::{BoxTree, NodeId},
};

/// Page size and margins, all in px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageConstraints {
    /// Page width.
    pub page_width: Px,
    /// Page height.
    pub page_height: Px,
    /// Margin above the content area.
    pub margin_top: Px,
    /// Margin to the right of the content area.
    pub margin_right: Px,
    /// Margin below the content area.
    pub margin_bottom: Px,
    /// Margin to the left of the content area.
    pub margin_left: Px,
    /// Space reserved at the top of the content area for a running header.
    pub header_height: Px,
    /// Space reserved at the bottom of the content area for a running footer.
    pub footer_height: Px,
}

impl PageConstraints {
    /// Page of the given size with no margins and no header or footer.
    pub fn new(page_width: Px, page_height: Px) -> Self {
        PageConstraints {
            page_width,
            page_height,
            margin_top: Px::ZERO,
            margin_right: Px::ZERO,
            margin_bottom: Px::ZERO,
            margin_left: Px::ZERO,
            header_height: Px::ZERO,
            footer_height: Px::ZERO,
        }
    }

    /// ISO A4 at 96 dpi, with 1 inch margins.
    pub fn a4() -> Self {
        Self::with_inch_margins(Px::from_px(794), Px::from_px(1123))
    }

    /// US Letter at 96 dpi, with 1 inch margins.
    pub fn letter() -> Self {
        Self::with_inch_margins(Px::from_px(816), Px::from_px(1056))
    }

    fn with_inch_margins(width: Px, height: Px) -> Self {
        let margin = Px::from_px(96);
        PageConstraints {
            margin_top: margin,
            margin_right: margin,
            margin_bottom: margin,
            margin_left: margin,
            ..PageConstraints::new(width, height)
        }
    }

    /// Height available for content on each page.
    pub fn content_height(&self) -> Px {
        (self.page_height
            - self.margin_top
            - self.margin_bottom
            - self.header_height
            - self.footer_height)
            .max(Px::ZERO)
    }
}

/// One page of the output. Sizes and offsets are `f32` pixels, the output boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    /// Page width.
    pub width: f32,
    /// Page height.
    pub height: f32,
    /// Where this page starts in the continuous document flow.
    pub content_offset_y: f32,
    /// Boxes with any content on this page, in document order.
    pub boxes: Vec<NodeId>,
}

/// A piece of a box on one page. `lines` is the range of a text box's lines placed here.
#[derive(Clone, Debug, PartialEq)]
pub struct PageFragment {
    /// The box this fragment belongs to.
    pub node: NodeId,
    /// Index of the page, counted from 0.
    pub page: usize,
    /// Top of the fragment, relative to the top of the page's content area.
    pub y: f32,
    /// Height of the fragment on its page.
    pub height: f32,
    /// For text, the lines placed on this page, as indices into the box's lines.
    pub lines: Option<Range<usize>>,
}

#[derive(Clone, Debug, PartialEq)]
/// The pages of a paginated layout, and where each box landed.
pub struct PageLayout {
    /// The pages, in order.
    pub pages: Vec<Page>,
    /// Height of the continuous flow.
    pub total_height: f32,
    /// Where each box or line range landed, across all pages.
    pub fragments: Vec<PageFragment>,
}

#[derive(Clone, Debug)]
struct Atom {
    nodes: Vec<NodeId>,
    /// Document-space top and height.
    top: Px,
    height: Px,
    /// Text only: the lines still to place. Empty for other atoms.
    lines: Vec<LineBox>,
    /// Index of `lines[0]` within the text node's full line list.
    first_line: usize,
    /// Table this atom belongs to (a row), if any.
    group: Option<NodeId>,
    header: bool,
    orphans: u16,
    widows: u16,
    break_before: Break,
    break_after: Break,
    /// A zero-height marker that only carries a forced break.
    marker: bool,
}

impl Atom {
    fn whole(nodes: Vec<NodeId>, top: Px, height: Px) -> Self {
        Atom {
            nodes,
            top,
            height,
            lines: Vec::new(),
            first_line: 0,
            group: None,
            header: false,
            orphans: 2,
            widows: 2,
            break_before: Break::Auto,
            break_after: Break::Auto,
            marker: false,
        }
    }
}

impl PageLayout {
    /// Adds a running header and footer to every page.
    ///
    /// `header` is placed at the top of each page's content area, and `footer` at the bottom, in
    /// the space reserved by `header_height` and `footer_height`. The same nodes repeat on every
    /// page, so per-page content such as page numbers is not supported. Their layouts are not
    /// changed: lay them out separately, with a width of `page_width`, before calling this.
    pub fn place_running(
        &mut self,
        constraints: &PageConstraints,
        header: Option<NodeId>,
        footer: Option<NodeId>,
    ) {
        let footer_y = constraints.header_height + constraints.content_height();
        for (index, page) in self.pages.iter_mut().enumerate() {
            if let Some(node) = header {
                self.fragments.push(PageFragment {
                    node,
                    page: index,
                    y: 0.0,
                    height: constraints.header_height.to_f32(),
                    lines: None,
                });
                page.boxes.push(node);
            }
            if let Some(node) = footer {
                self.fragments.push(PageFragment {
                    node,
                    page: index,
                    y: footer_y.to_f32(),
                    height: constraints.footer_height.to_f32(),
                    lines: None,
                });
                page.boxes.push(node);
            }
        }
    }
}

/// Splits a laid-out tree into pages. `root` is the box whose descendants are paginated.
pub fn paginate(tree: &BoxTree, root: NodeId, constraints: &PageConstraints) -> PageLayout {
    let mut atoms = Vec::new();
    collect(tree, root, &mut atoms);
    let mut filler = Filler::new(constraints);
    let mut queue: VecDeque<Atom> = atoms.into();
    while let Some(atom) = queue.pop_front() {
        filler.place(atom, &mut queue);
    }
    filler.finish()
}

/// Collects break candidates in document order.
fn collect(tree: &BoxTree, id: NodeId, out: &mut Vec<Atom>) {
    let Some(node) = tree.get(id) else { return };
    let Some(frame) = tree.frame(id) else { return };
    let style = node.style;
    if style.position.is_out_of_flow() || style.display.is_inline_level() {
        return;
    }
    let (top, height) = (frame.y, frame.height);

    if node.text.is_some() {
        let mut atom = Atom::whole(alloc::vec![id], top, height);
        atom.lines = node.lines.clone();
        atom.orphans = style.orphans;
        atom.widows = style.widows;
        atom.break_before = style.break_before;
        atom.break_after = style.break_after;
        out.push(atom);
        return;
    }

    if style.display == Display::Table {
        collect_table(tree, id, out);
        return;
    }

    let has_block_children = tree.laid_children(id).iter().any(|child| {
        tree.get(*child).is_some_and(|c| {
            !c.style.position.is_out_of_flow() && !c.style.display.is_inline_level()
        })
    });
    // A block whose children are all inline is divided between its line boxes, like a text leaf.
    // Its top is the content top, below any border and padding.
    if !has_block_children && !node.lines.is_empty() && style.break_inside != Break::Avoid {
        let inset = Sides::of(&style, Px::ZERO);
        let inset_top = inset.border.top + inset.padding.top;
        let height = node
            .lines
            .iter()
            .fold(Px::ZERO, |acc, line| acc + line.height);
        let mut atom = Atom::whole(alloc::vec![id], top + inset_top, height);
        atom.lines = node.lines.clone();
        atom.orphans = style.orphans;
        atom.widows = style.widows;
        atom.break_before = style.break_before;
        atom.break_after = style.break_after;
        out.push(atom);
        return;
    }
    let indivisible = style.break_inside == Break::Avoid
        || !has_block_children
        || matches!(style.display, Display::Flex | Display::Grid);
    if indivisible {
        let mut atom = Atom::whole(alloc::vec![id], top, height);
        atom.break_before = style.break_before;
        atom.break_after = style.break_after;
        out.push(atom);
        return;
    }

    push_marker(out, style.break_before, top);
    for child in tree.laid_children(id).to_vec() {
        collect(tree, child, out);
    }
    push_marker(out, style.break_after, top + height);
}

fn push_marker(out: &mut Vec<Atom>, kind: Break, top: Px) {
    if matches!(kind, Break::Auto | Break::Avoid) {
        return;
    }
    let mut marker = Atom::whole(Vec::new(), top, Px::ZERO);
    marker.marker = true;
    marker.break_before = kind;
    out.push(marker);
}

/// Table rows become atoms. Cells are grouped by the row they sit in, using their frames.
fn collect_table(tree: &BoxTree, table: NodeId, out: &mut Vec<Atom>) {
    for row in tree.laid_children(table).to_vec() {
        let cells: Vec<NodeId> = if tree
            .get(row)
            .is_some_and(|n| n.style.display == Display::TableRow)
        {
            tree.laid_children(row).to_vec()
        } else {
            alloc::vec![row]
        };
        let mut top: Option<Px> = None;
        let mut bottom = Px::ZERO;
        // The row is a header when its row is marked, or when one of its cells is.
        let mut header = tree.get(row).is_some_and(|n| n.style.table_header);
        for cell in &cells {
            if let (Some(frame), Some(node)) = (tree.frame(*cell), tree.get(*cell)) {
                top = Some(top.map_or(frame.y, |t| t.min(frame.y)));
                bottom = bottom.max(frame.y + frame.height);
                header |= node.style.table_header;
            }
        }
        let Some(top) = top else { continue };
        let mut atom = Atom::whole(cells, top, bottom - top);
        atom.group = Some(table);
        atom.header = header;
        out.push(atom);
    }
}

struct Filler {
    content_height: Px,
    pages: Vec<Page>,
    fragments: Vec<PageFragment>,
    /// Local y of the next free position on the current page.
    cursor: Px,
    /// Document y of the bottom of the last atom placed on this page, for gap measurement.
    prev_bottom: Option<Px>,
    /// Table whose rows are on the current page, for header repetition.
    current_group: Option<NodeId>,
    page_has_content: bool,
    /// A break requested by a marker, applied before the next real atom.
    pending_break: Break,
    /// Header rows seen so far, repeated on continuation pages.
    headers: Vec<Atom>,
    first_top: Option<Px>,
    last_bottom: Px,
}

impl Filler {
    fn new(constraints: &PageConstraints) -> Self {
        let mut filler = Filler {
            content_height: constraints.content_height(),
            pages: Vec::new(),
            fragments: Vec::new(),
            cursor: Px::ZERO,
            prev_bottom: None,
            current_group: None,
            page_has_content: false,
            pending_break: Break::Auto,
            headers: Vec::new(),
            first_top: None,
            last_bottom: Px::ZERO,
        };
        filler.pages.push(Page {
            width: constraints.page_width.to_f32(),
            height: constraints.page_height.to_f32(),
            content_offset_y: 0.0,
            boxes: Vec::new(),
        });
        filler
    }

    /// Starts a new page whose content begins at document y `doc_top`.
    fn new_page(&mut self, doc_top: Px) {
        let width = self.pages[0].width;
        let height = self.pages[0].height;
        self.pages.push(Page {
            width,
            height,
            content_offset_y: doc_top.to_f32(),
            boxes: Vec::new(),
        });
        self.cursor = Px::ZERO;
        self.prev_bottom = None;
        self.current_group = None;
        self.page_has_content = false;
    }

    /// Breaks to a page whose number has the parity `kind` asks for. Page 1 is a right page.
    fn parity_break(&mut self, kind: Break, doc_top: Px) {
        self.new_page(doc_top);
        let page_number = self.pages.len();
        let want_even = kind == Break::Left;
        if (page_number % 2 == 0) != want_even {
            self.new_page(doc_top);
        }
    }

    fn place(&mut self, atom: Atom, queue: &mut VecDeque<Atom>) {
        if atom.marker {
            self.pending_break = atom.break_before;
            return;
        }
        self.first_top.get_or_insert(atom.top);
        self.last_bottom = self.last_bottom.max(atom.top + atom.height);

        let requested = match self.pending_break {
            Break::Auto | Break::Avoid => atom.break_before,
            other => other,
        };
        self.pending_break = Break::Auto;
        if self.page_has_content {
            match requested {
                Break::Always => self.new_page(atom.top),
                Break::Left | Break::Right => self.parity_break(requested, atom.top),
                Break::Auto | Break::Avoid => {}
            }
        }

        let gap = self.gap_before(&atom);
        if self.cursor + gap + atom.height <= self.content_height {
            self.emit(&atom, gap, None, false);
            return;
        }
        if !atom.lines.is_empty() {
            self.split_text(atom, gap, queue);
            return;
        }
        if !self.page_has_content {
            // Oversized and alone on an empty page: it overflows here.
            self.emit(&atom, Px::ZERO, None, false);
            return;
        }
        // Whole box moves to the next page, preceded by repeated table headers if it continues a
        // table.
        self.new_page(atom.top);
        self.repeat_headers_for(&atom);
        queue.push_front(atom);
    }

    fn gap_before(&self, atom: &Atom) -> Px {
        match self.prev_bottom {
            Some(prev) => (atom.top - prev).max(Px::ZERO),
            None => Px::ZERO,
        }
    }

    /// On a new page, puts the headers of `atom`'s table at the top, if the atom continues the
    /// table.
    fn repeat_headers_for(&mut self, atom: &Atom) {
        let Some(group) = atom.group else { return };
        if atom.header {
            return;
        }
        let headers: Vec<Atom> = self
            .headers
            .iter()
            .filter(|h| h.group == Some(group))
            .cloned()
            .collect();
        for header in headers {
            self.emit(&header, Px::ZERO, None, true);
        }
    }

    fn split_text(&mut self, atom: Atom, gap: Px, queue: &mut VecDeque<Atom>) {
        let total = atom.lines.len();
        let mut fit = 0;
        let mut used = self.cursor + gap;
        for line in &atom.lines {
            if used + line.height > self.content_height {
                break;
            }
            used += line.height;
            fit += 1;
        }
        let orphans = atom.orphans as usize;
        let widows = atom.widows as usize;
        let mut split = fit;
        if total - fit < widows {
            split = total.saturating_sub(widows);
        }
        let acceptable = split >= orphans && split > 0 && split < total;
        if !acceptable {
            if self.page_has_content {
                let top = atom.top;
                self.new_page(top);
                queue.push_front(atom);
            } else {
                // Cannot be split acceptably even on an empty page: place it whole.
                self.emit(&atom, Px::ZERO, None, false);
            }
            return;
        }

        let head_height = atom.lines[..split]
            .iter()
            .fold(Px::ZERO, |acc, l| acc + l.height);
        let rest_top = atom.top + head_height;
        let head = Atom {
            top: atom.top,
            height: head_height,
            lines: atom.lines[..split].to_vec(),
            ..atom.clone()
        };
        let tail = Atom {
            top: rest_top,
            height: atom.height - head_height,
            lines: atom.lines[split..].to_vec(),
            first_line: atom.first_line + split,
            ..atom
        };
        let range = head.first_line..head.first_line + split;
        self.emit(&head, gap, Some(range), false);
        self.new_page(rest_top);
        queue.push_front(tail);
    }

    /// Records an atom on the current page at the cursor. A repeated header does not affect the
    /// gap bookkeeping for the rows that follow it.
    fn emit(&mut self, atom: &Atom, gap: Px, lines: Option<Range<usize>>, repeated: bool) {
        let y = self.cursor + gap;
        let page = self.pages.len() - 1;
        // Text atoms always report which of the node's lines they carry.
        let lines = lines.or_else(|| {
            (!atom.lines.is_empty()).then(|| atom.first_line..atom.first_line + atom.lines.len())
        });
        for node in &atom.nodes {
            self.fragments.push(PageFragment {
                node: *node,
                page,
                y: y.to_f32(),
                height: atom.height.to_f32(),
                lines: lines.clone(),
            });
            if !self.pages[page].boxes.contains(node) {
                self.pages[page].boxes.push(*node);
            }
        }
        if atom.header && !repeated && !self.headers.iter().any(|h| h.nodes == atom.nodes) {
            self.headers.push(atom.clone());
        }
        self.cursor = y + atom.height;
        self.page_has_content = true;
        if !repeated {
            self.prev_bottom = Some(atom.top + atom.height);
            self.current_group = atom.group;
        }
    }

    fn finish(mut self) -> PageLayout {
        // A trailing forced break leaves an empty last page: drop it.
        if self.pages.len() > 1 && !self.page_has_content {
            self.pages.pop();
        }
        let total = match self.first_top {
            Some(top) => (self.last_bottom - top).max(Px::ZERO),
            None => Px::ZERO,
        };
        PageLayout {
            pages: self.pages,
            total_height: total.to_f32(),
            fragments: self.fragments,
        }
    }
}

/// Lays out `id`'s block children in `column_count` columns, balancing their heights.
pub(crate) fn layout_columns(
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
    let count = style.column_count.max(1) as i32;
    let gap = style
        .column_gap
        .resolve(Some(content.width))
        .unwrap_or(Px::ZERO)
        .max(Px::ZERO);
    // Gaps larger than the content leave no room; clamp rather than produce a negative width.
    let column_width = ((content.width - gap * (count - 1)) / count).max(Px::ZERO);

    // Lay the children out in one tall column at column width.
    let column = Containing {
        x: content.x,
        width: column_width,
        height: content.height,
    };
    let (end, pending, baseline) =
        layout_block_children(pass, tree, id, column, top, false, deferred);
    let flow_height = (end + pending.resolve() - top).max(Px::ZERO);
    let target = Px(flow_height.0.saturating_add(count - 1) / count);

    // Block-level children with frames, in document order: (id, border top, border bottom).
    let children: Vec<(NodeId, Px, Px)> = tree
        .children(id)
        .iter()
        .filter_map(|child| {
            let node = tree.get(*child)?;
            if node.style.position.is_out_of_flow() || node.style.display.is_inline_level() {
                return None;
            }
            let frame = tree.frame(*child)?;
            Some((*child, frame.y, frame.y + frame.height))
        })
        .collect();

    // Greedy fill: a child that overflows the target starts the next column.
    let mut column_index = 0;
    let mut column_start = top;
    let mut column_has_child = false;
    let mut bottom = top;
    for (child, child_top, child_bottom) in children {
        // A column always takes at least one child, so the first child never starts a new one.
        if column_has_child && child_bottom - column_start > target && column_index + 1 < count {
            column_index += 1;
            column_start = child_top;
        }
        column_has_child = true;
        let dx = column_width * column_index + gap * column_index;
        let dy = top - column_start;
        tree.shift_subtree(child, dx, dy);
        bottom = bottom.max(child_bottom + dy);
    }
    (bottom, Margin::ZERO, baseline)
}
