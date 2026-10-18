//! A short document laid out in block flow, with a flex toolbar and a grid of cards, then split
//! into A4 pages.
//!
//! Run with `cargo run --example document`.

use tpt_layout::{
    layout_flow, paginate, paint_order, BoxStyle, BoxTree, Dimension, Display,
    FixedAdvanceMeasurer, FlexDirection, JustifyContent, PageConstraints, Px, TrackSize, Tracks,
};

fn main() {
    // 8 px per character, 20 px lines.
    let measurer = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(20), Px::from_px(15));
    let page = PageConstraints::a4();
    let width = Px::from_px(page.page_width.to_f32() as i32 - 192);

    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());

    // A title and a paragraph.
    let title = tree.insert_text(
        BoxStyle {
            margin: tpt_layout::Edges {
                top: Dimension::px(0),
                right: Dimension::px(0),
                bottom: Dimension::px(16),
                left: Dimension::px(0),
            },
            ..BoxStyle::default()
        },
        "Quarterly report",
    );
    tree.append_child(root, title).unwrap();
    let body = tree.insert_text(
        BoxStyle::default(),
        "lorem ipsum dolor sit amet ".repeat(40),
    );
    tree.append_child(root, body).unwrap();

    // A toolbar: a row of three buttons that space out.
    let toolbar = tree.insert(BoxStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        margin: tpt_layout::Edges {
            top: Dimension::px(16),
            right: Dimension::px(0),
            bottom: Dimension::px(16),
            left: Dimension::px(0),
        },
        ..BoxStyle::default()
    });
    tree.append_child(root, toolbar).unwrap();
    for _ in 0..3 {
        let button = tree.insert(BoxStyle {
            width: Dimension::px(80),
            height: Dimension::px(28),
            ..BoxStyle::default()
        });
        tree.append_child(toolbar, button).unwrap();
    }

    // A grid of six cards in two columns.
    let grid = tree.insert(BoxStyle {
        display: Display::Grid,
        ..BoxStyle::default()
    });
    tree.set_grid_template(
        grid,
        tpt_layout::GridTemplate {
            columns: Tracks::new(&[TrackSize::Fr(100), TrackSize::Fr(100)]),
            ..Default::default()
        },
    );
    tree.append_child(root, grid).unwrap();
    for index in 0..6 {
        let card = tree.insert(BoxStyle {
            height: Dimension::px(if index % 2 == 0 { 120 } else { 90 }),
            ..BoxStyle::default()
        });
        tree.append_child(grid, card).unwrap();
    }

    layout_flow(&mut tree, root, width, &measurer).unwrap();
    let root_layout = tree.get(root).unwrap().layout;
    println!(
        "content width {}px, height {}px",
        root_layout.width, root_layout.height
    );

    let pages = paginate(&tree, root, &page);
    println!(
        "{} page(s), flow height {:.0}px, A4 content height {:.0}px",
        pages.pages.len(),
        pages.total_height,
        page.content_height().to_f32()
    );
    for (index, page) in pages.pages.iter().enumerate() {
        println!(
            "  page {}: {} box(es) from y={:.0}",
            index + 1,
            page.boxes.len(),
            page.content_offset_y
        );
    }

    let painted = paint_order(&tree, root).len();
    println!("{painted} boxes in paint order");
}
