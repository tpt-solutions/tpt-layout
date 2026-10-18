//! Adapted from web-platform-tests `css/css-flexbox/flexbox-writing-mode-*.html`. Those are pixel
//! reftests: the four items must read cyan, magenta, yellow, black (CMYK) in the right places.
//! Positions here are the ones that pass condition implies.

mod common;

use common::*;
use tpt_layout::{BoxStyle, BoxTree, Dimension, Display, Edges, FlexWrap, WritingMode};

#[test]
fn flex_row_wrap_in_vertical_rl_fills_columns_from_the_right() {
    // flexbox-writing-mode-005.html is the rtl variant; this is -001's vertical-rl counterpart:
    // a 40x30 container with a 1px border and four 20x15 items, `flex-flow: row wrap`.
    // In vertical-rl the inline (row) axis is vertical, so two items fill each 30 px column,
    // and the first column is the rightmost one.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let container = child(
        &mut tree,
        root,
        BoxStyle {
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            writing_mode: WritingMode::VerticalRl,
            width: Dimension::px(40),
            height: Dimension::px(30),
            border: Edges::all(tpt_layout::Px::from_px(1)),
            ..BoxStyle::default()
        },
    );
    let cyan = child(&mut tree, container, boxed(20, 15));
    let magenta = child(&mut tree, container, boxed(20, 15));
    let yellow = child(&mut tree, container, boxed(20, 15));
    let black = child(&mut tree, container, boxed(20, 15));
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, cyan), (21.0, 1.0, 20.0, 15.0));
    assert_eq!(rect(&tree, magenta), (21.0, 16.0, 20.0, 15.0));
    assert_eq!(rect(&tree, yellow), (1.0, 1.0, 20.0, 15.0));
    assert_eq!(rect(&tree, black), (1.0, 16.0, 20.0, 15.0));
}

#[test]
fn flex_row_wrap_in_sideways_lr_fills_columns_from_the_left_bottom_up() {
    // flexbox-writing-mode-slr.html: lines run bottom to top, and the columns stack left to right.
    // The first item sits at the bottom of the first column.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let container = child(
        &mut tree,
        root,
        BoxStyle {
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            writing_mode: WritingMode::SidewaysLr,
            width: Dimension::px(40),
            height: Dimension::px(30),
            border: Edges::all(tpt_layout::Px::from_px(1)),
            ..BoxStyle::default()
        },
    );
    let cyan = child(&mut tree, container, boxed(20, 15));
    let magenta = child(&mut tree, container, boxed(20, 15));
    let yellow = child(&mut tree, container, boxed(20, 15));
    let black = child(&mut tree, container, boxed(20, 15));
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, cyan), (1.0, 16.0, 20.0, 15.0));
    assert_eq!(rect(&tree, magenta), (1.0, 1.0, 20.0, 15.0));
    assert_eq!(rect(&tree, yellow), (21.0, 16.0, 20.0, 15.0));
    assert_eq!(rect(&tree, black), (21.0, 1.0, 20.0, 15.0));
}
