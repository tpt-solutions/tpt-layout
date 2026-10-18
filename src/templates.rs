//! Starter layouts. Each builder returns a laid-out [`Template`] built from simple data, using the
//! built-in [`FixedAdvanceMeasurer`]. Copy one as a starting point, then replace the measurer with
//! your own [`crate::text::TextMeasurer`] (a font-backed one, for example).
//!
//! The templates are also pinned by golden SVG files in `tests/golden/templates/`.

use alloc::{
    format,
    string::{String, ToString},
};

use crate::{
    layout_document, paginate, svg, BoxStyle, BoxTree, Dimension, Direction, Display,
    FixedAdvanceMeasurer, FlexDirection, GridTemplate, JustifyContent, LayoutConstraints,
    LayoutContext, NodeId, PageConstraints, PageLayout, Px, TrackSize, Tracks, WritingMode,
};

/// A laid-out template: the tree, its root, and the root's size in px.
pub struct Template {
    /// The laid-out tree.
    pub tree: BoxTree,
    /// The root box.
    pub root: NodeId,
    /// Width of the root.
    pub width: Px,
    /// Height of the root.
    pub height: Px,
}

impl Template {
    /// Splits the laid-out tree into pages of the given size.
    pub fn paginate(&self, page: &PageConstraints) -> PageLayout {
        paginate(&self.tree, self.root, page)
    }

    /// Renders the laid-out tree as a standalone SVG document.
    pub fn to_svg(&self) -> String {
        svg::render_svg(
            &self.tree,
            self.root,
            self.width.to_f32(),
            self.height.to_f32(),
        )
    }
}

/// The measurer every template uses: 8 px per character, 16 px lines.
fn measurer() -> FixedAdvanceMeasurer {
    FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12))
}

/// Lays out `tree` at `width` and packages the result.
fn finish(mut tree: BoxTree, root: NodeId, width: i32) -> Template {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let result = layout_document(
        &mut tree,
        root,
        LayoutConstraints {
            width: Px::from_px(width),
            height: None,
        },
        &ctx,
    )
    .expect("template layout");
    Template {
        tree,
        root,
        width: result.width,
        height: result.height,
    }
}

fn add(tree: &mut BoxTree, parent: NodeId, style: BoxStyle) -> NodeId {
    let id = tree.insert(style);
    tree.append_child(parent, id).expect("template child");
    id
}

/// Adds a paragraph holding `text` as an inline run. Inline text keeps its characters, so the SVG
/// renderer draws them; block-level text would show only line outlines.
fn add_text(tree: &mut BoxTree, parent: NodeId, text: &str) -> NodeId {
    let paragraph = add(tree, parent, BoxStyle::default());
    let id = tree.insert_text(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        text,
    );
    tree.append_child(paragraph, id).expect("template text");
    paragraph
}

fn px(v: i32) -> Dimension {
    Dimension::Length(Px::from_px(v))
}

/// A page-width invoice: a heading, the customer, and a table of items with a header row and a
/// total. `items` are `(description, quantity, unit price)`, in whole currency units.
pub fn invoice(customer: &str, items: &[(&str, i32, i32)]) -> Template {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: px(400),
        ..BoxStyle::default()
    });
    add_text(&mut tree, root, "Invoice");
    add_text(&mut tree, root, &format!("Bill to: {customer}"));

    let table = add(
        &mut tree,
        root,
        BoxStyle {
            display: Display::Table,
            ..BoxStyle::default()
        },
    );
    let header = add(
        &mut tree,
        table,
        BoxStyle {
            display: Display::TableRow,
            table_header: true,
            ..BoxStyle::default()
        },
    );
    for label in ["Item", "Qty", "Total"] {
        let cell = add(
            &mut tree,
            header,
            BoxStyle {
                display: Display::TableCell,
                ..BoxStyle::default()
            },
        );
        add_text(&mut tree, cell, label);
    }

    let mut total = 0;
    for (description, quantity, unit) in items {
        let line = quantity * unit;
        total += line;
        let row = add(
            &mut tree,
            table,
            BoxStyle {
                display: Display::TableRow,
                ..BoxStyle::default()
            },
        );
        for value in [
            description.to_string(),
            quantity.to_string(),
            line.to_string(),
        ] {
            let cell = add(
                &mut tree,
                row,
                BoxStyle {
                    display: Display::TableCell,
                    ..BoxStyle::default()
                },
            );
            add_text(&mut tree, cell, &value);
        }
    }
    add_text(&mut tree, root, &format!("Total due: {total}"));
    finish(tree, root, 400)
}

/// A dashboard of cards in a three-column grid with 120 px tracks and 10 px gaps. `cards` are
/// `(title, value)` pairs.
pub fn dashboard(cards: &[(&str, &str)]) -> Template {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: px(400),
        ..BoxStyle::default()
    });
    let grid = add(
        &mut tree,
        root,
        BoxStyle {
            display: Display::Grid,
            width: px(380),
            row_gap: px(10),
            column_gap: px(10),
            ..BoxStyle::default()
        },
    );
    tree.set_grid_template(
        grid,
        GridTemplate {
            columns: Tracks::new(&[
                TrackSize::Fixed(px(120)),
                TrackSize::Fixed(px(120)),
                TrackSize::Fixed(px(120)),
            ]),
            ..GridTemplate::default()
        },
    );
    for (title, value) in cards {
        let card = add(
            &mut tree,
            grid,
            BoxStyle {
                height: px(60),
                ..BoxStyle::default()
            },
        );
        add_text(&mut tree, card, title);
        add_text(&mut tree, card, value);
    }
    finish(tree, root, 400)
}

/// A navbar: the brand on the left and the links on the right, in a row with the free space
/// between them.
pub fn navbar(brand: &str, links: &[&str]) -> Template {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        display: Display::Flex,
        width: px(400),
        justify_content: JustifyContent::SpaceBetween,
        ..BoxStyle::default()
    });
    add_text(&mut tree, root, brand);
    let nav = add(
        &mut tree,
        root,
        BoxStyle {
            display: Display::Flex,
            ..BoxStyle::default()
        },
    );
    for link in links {
        add_text(&mut tree, nav, link);
    }
    finish(tree, root, 400)
}

/// A two-column resume: a fixed-width sidebar and a main column that takes the rest. Each slice
/// is one line of text.
pub fn resume(name: &str, sidebar: &[&str], main: &[&str]) -> Template {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        width: px(480),
        ..BoxStyle::default()
    });
    let side = add(
        &mut tree,
        root,
        BoxStyle {
            width: px(160),
            ..BoxStyle::default()
        },
    );
    for line in sidebar {
        add_text(&mut tree, side, line);
    }
    let body = add(
        &mut tree,
        root,
        BoxStyle {
            flex_grow: 1.0,
            ..BoxStyle::default()
        },
    );
    add_text(&mut tree, body, name);
    for line in main {
        add_text(&mut tree, body, line);
    }
    finish(tree, root, 480)
}

/// A right-to-left page: each paragraph is a block whose inline text starts at the right edge.
pub fn rtl_page(paragraphs: &[&str]) -> Template {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: px(400),
        direction: Direction::Rtl,
        ..BoxStyle::default()
    });
    for paragraph in paragraphs {
        // The paragraphs inherit the root's direction.
        let block = add(&mut tree, root, BoxStyle::default());
        let text = tree.insert_text(
            BoxStyle {
                display: Display::Inline,
                ..BoxStyle::default()
            },
            *paragraph,
        );
        tree.append_child(block, text).expect("template text");
    }
    finish(tree, root, 400)
}

/// A vertical (top-to-bottom, right-to-left) page of one column of text. The page is 320 px tall,
/// so each line runs down it and the columns stack from the right.
pub fn vertical_page(lines: &[&str]) -> Template {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: px(320),
        height: px(320),
        writing_mode: WritingMode::VerticalRl,
        ..BoxStyle::default()
    });
    for line in lines {
        add_text(&mut tree, root, line);
    }
    finish(tree, root, 320)
}

/// A newsletter: paragraphs flow into two balanced columns with a 16 px gap.
pub fn newsletter(paragraphs: &[&str]) -> Template {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: px(400),
        column_count: 2,
        column_gap: px(16),
        ..BoxStyle::default()
    });
    for paragraph in paragraphs {
        add_text(&mut tree, root, paragraph);
    }
    finish(tree, root, 400)
}

/// A report: a title and paragraphs of text, each paragraph set to split across pages with two
/// lines kept at the top and bottom of a page (orphans and widows). Paragraphs are 400 px wide and
/// separated by 8 px. Paginate it with [`Template::paginate`].
pub fn report(title: &str, paragraphs: &[&str]) -> Template {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: px(400),
        ..BoxStyle::default()
    });
    add_text(&mut tree, root, title);
    for paragraph in paragraphs {
        let block = tree.insert(BoxStyle {
            orphans: 2,
            widows: 2,
            margin: crate::math::Edges {
                top: Dimension::Auto,
                right: Dimension::Auto,
                bottom: px(8),
                left: Dimension::Auto,
            },
            ..BoxStyle::default()
        });
        tree.append_child(root, block).expect("template paragraph");
        let text = tree.insert_text(
            BoxStyle {
                display: Display::Inline,
                ..BoxStyle::default()
            },
            *paragraph,
        );
        tree.append_child(block, text).expect("template text");
    }
    finish(tree, root, 400)
}
