//! Named grid lines and replaced elements (images).

mod common;

use common::*;
use tpt_layout::{
    BoxStyle, BoxTree, Dimension, Display, GridLine, GridPlacement, TrackSize, Tracks,
};

#[test]
fn named_lines_place_items_between_them() {
    // Three 100 px columns. Line "a" is before track 1 and line "b" is after track 2.
    let mut tree = BoxTree::new();
    let tracks = Tracks::new(&[TrackSize::Fixed(Dimension::px(100)); 3])
        .with_line_name(1, 1)
        .with_line_name(3, 2);
    let root = grid(
        &mut tree,
        tpt_layout::GridTemplate {
            columns: tracks,
            ..Default::default()
        },
        BoxStyle::default(),
    );
    let item = child(
        &mut tree,
        root,
        BoxStyle {
            grid_column: GridPlacement {
                start: GridLine::Named(1),
                end: GridLine::Named(2),
            },
            ..height_only(20)
        },
    );
    layout(&mut tree, root, 300);
    // Lines 1 to 3 cover tracks 2 and 3.
    assert_eq!(rect(&tree, item).0, 100.0);
    assert_eq!(rect(&tree, item).2, 200.0);
}

#[test]
fn unknown_line_name_is_auto() {
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        tpt_layout::GridTemplate {
            columns: Tracks::new(&[TrackSize::Fixed(Dimension::px(100)); 2]),
            ..Default::default()
        },
        BoxStyle::default(),
    );
    let item = child(
        &mut tree,
        root,
        BoxStyle {
            grid_column: GridPlacement {
                start: GridLine::Named(9),
                end: GridLine::Auto,
            },
            ..height_only(20)
        },
    );
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, item).0, 0.0);
}

#[test]
fn inline_image_sits_on_the_baseline_and_expands_the_line() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: tpt_layout::Dimension::px(200),
        ..BoxStyle::default()
    });
    let text = text_child(
        &mut tree,
        root,
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        "aa",
    );
    let image = tree.insert_replaced(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        px(30),
        px(20),
    );
    tree.append_child(root, image).unwrap();
    layout(&mut tree, root, 200);
    // The image is 20 px tall with its baseline at its bottom, so it rises 5 px above the text's
    // baseline and the line is 25 px tall.
    assert_eq!(rect(&tree, image), (20.0, 0.0, 30.0, 20.0));
    assert_eq!(rect(&tree, root).3, 25.0);
    assert_eq!(tree.get(text).unwrap().fragments[0].y.to_f32(), 5.0);
}

#[test]
fn block_image_takes_intrinsic_size_and_explicit_size_wins() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: tpt_layout::Dimension::px(200),
        ..BoxStyle::default()
    });
    let natural = tree.insert_replaced(BoxStyle::default(), px(40), px(30));
    tree.append_child(root, natural).unwrap();
    let scaled = tree.insert_replaced(
        BoxStyle {
            width: Dimension::px(10),
            ..BoxStyle::default()
        },
        px(40),
        px(30),
    );
    tree.append_child(root, scaled).unwrap();
    layout(&mut tree, root, 200);
    assert_eq!(rect(&tree, natural), (0.0, 0.0, 40.0, 30.0));
    assert_eq!(rect(&tree, scaled), (0.0, 30.0, 10.0, 30.0));
}
