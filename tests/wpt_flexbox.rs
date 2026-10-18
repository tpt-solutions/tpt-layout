//! Adapted from the official CSS Flexbox tests in web-platform-tests (`css/css-flexbox`). Each
//! test names its source file. The official tests are pixel reftests; these check the geometry
//! that each test's "pass" condition describes. Font-dependent tests (Ahem) are not included.

mod common;

use common::*;
use tpt_layout::{AlignContent, AlignItems, BoxStyle, BoxTree, Dimension, Display, FlexWrap};

fn flex_box(width: i32, height: i32) -> BoxStyle {
    BoxStyle {
        display: Display::Flex,
        width: Dimension::px(width),
        height: Dimension::px(height),
        ..BoxStyle::default()
    }
}

/// Two 150x52 items in a 300x100 flex container (`align-items-001.htm`, `-002`, `-003`).
fn two_items_with_align(align: AlignItems, item_height: i32) -> Vec<(f32, f32, f32, f32)> {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        align_items: align,
        ..flex_box(300, 100)
    });
    let a = child(&mut tree, root, boxed(150, item_height));
    let b = child(&mut tree, root, boxed(150, item_height));
    layout(&mut tree, root, 300);
    vec![rect(&tree, a), rect(&tree, b)]
}

#[test]
fn align_items_center_centers_items_in_the_line() {
    // align-items-001.htm: 52 px items centred in 100 px.
    let items = two_items_with_align(AlignItems::Center, 52);
    assert_eq!(items[0], (0.0, 24.0, 150.0, 52.0));
    assert_eq!(items[1], (150.0, 24.0, 150.0, 52.0));
}

#[test]
fn align_items_flex_start_aligns_to_the_top() {
    // align-items-002.htm
    let items = two_items_with_align(AlignItems::FlexStart, 51);
    assert_eq!(items[0], (0.0, 0.0, 150.0, 51.0));
}

#[test]
fn align_items_flex_end_aligns_to_the_bottom() {
    // align-items-003.htm
    let items = two_items_with_align(AlignItems::FlexEnd, 51);
    assert_eq!(items[0], (0.0, 49.0, 150.0, 51.0));
}

#[test]
fn align_items_stretch_fills_the_line_when_height_is_auto() {
    // align-items-005.htm: the items have no height, so they stretch to the 100 px line.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        align_items: AlignItems::Stretch,
        ..flex_box(300, 100)
    });
    let a = child(
        &mut tree,
        root,
        BoxStyle {
            width: Dimension::px(150),
            ..BoxStyle::default()
        },
    );
    let b = child(
        &mut tree,
        root,
        BoxStyle {
            width: Dimension::px(150),
            ..BoxStyle::default()
        },
    );
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, a), (0.0, 0.0, 150.0, 100.0));
    assert_eq!(rect(&tree, b), (150.0, 0.0, 150.0, 100.0));
}

/// Three 50x50 items in an 80 px wide, wrapping, column of lines (`align-content_*.html`). Each
/// item is on its own line because two would need 100 px.
fn three_lines(align: AlignContent, container_height: i32) -> Vec<f32> {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        flex_wrap: FlexWrap::Wrap,
        align_content: align,
        ..flex_box(80, container_height)
    });
    let ids: Vec<_> = (0..3)
        .map(|_| child(&mut tree, root, boxed(50, 50)))
        .collect();
    layout(&mut tree, root, 80);
    ids.iter().map(|id| rect(&tree, *id).1).collect()
}

#[test]
fn align_content_center() {
    // align-content_center.html: the 150 px of lines sit in the middle of 200 px.
    assert_eq!(three_lines(AlignContent::Center, 200), [25.0, 75.0, 125.0]);
}

#[test]
fn align_content_flex_end() {
    assert_eq!(
        three_lines(AlignContent::FlexEnd, 200),
        [50.0, 100.0, 150.0]
    );
}

#[test]
fn align_content_flex_start() {
    assert_eq!(
        three_lines(AlignContent::FlexStart, 200),
        [0.0, 50.0, 100.0]
    );
}

#[test]
fn align_content_space_between() {
    // align-content_space-between.html: the first and last lines touch the edges.
    assert_eq!(
        three_lines(AlignContent::SpaceBetween, 200),
        [0.0, 75.0, 150.0]
    );
}

#[test]
fn align_content_space_around() {
    // align-content_space-around.html (210 px): half-size space at each end.
    assert_eq!(
        three_lines(AlignContent::SpaceAround, 210),
        [10.0, 80.0, 150.0]
    );
}

#[test]
fn align_content_stretch() {
    // align-content_stretch.html (210 px): each line grows by the 20 px share of free space.
    assert_eq!(three_lines(AlignContent::Stretch, 210), [0.0, 70.0, 140.0]);
}

/// Four 25x50 items in a 100x100 flex row (`align-self-001.html`, `-002.html`).
fn four_items_with_self(align_self: AlignItems) -> Vec<f32> {
    let mut tree = BoxTree::new();
    let root = tree.insert(flex_box(100, 100));
    let ids: Vec<_> = (0..4)
        .map(|_| {
            child(
                &mut tree,
                root,
                BoxStyle {
                    align_self: Some(align_self),
                    ..boxed(25, 50)
                },
            )
        })
        .collect();
    layout(&mut tree, root, 100);
    ids.iter().map(|id| rect(&tree, *id).1).collect()
}

#[test]
fn align_self_flex_start() {
    assert_eq!(
        four_items_with_self(AlignItems::FlexStart),
        [0.0, 0.0, 0.0, 0.0]
    );
}

#[test]
fn align_self_flex_end() {
    assert_eq!(
        four_items_with_self(AlignItems::FlexEnd),
        [50.0, 50.0, 50.0, 50.0]
    );
}

#[test]
fn flex_basis_positive_wins_over_width() {
    // flex-basis-001.html and flex-basis-002.html: a 60 px basis beats the item's own width.
    let mut tree = BoxTree::new();
    let root = tree.insert(flex_box(100, 100));
    let test = child(
        &mut tree,
        root,
        BoxStyle {
            flex_basis: Dimension::px(60),
            width: Dimension::px(80),
            ..height_only(100)
        },
    );
    let reference = child(
        &mut tree,
        root,
        BoxStyle {
            width: Dimension::px(40),
            ..height_only(100)
        },
    );
    layout(&mut tree, root, 100);
    assert_eq!(rect(&tree, test), (0.0, 0.0, 60.0, 100.0));
    assert_eq!(rect(&tree, reference), (60.0, 0.0, 40.0, 100.0));
}

#[test]
fn flex_basis_negative_is_treated_as_zero() {
    // flex-basis-003.html: a negative basis is invalid, so the item has no main size of its own.
    let mut tree = BoxTree::new();
    let root = tree.insert(flex_box(100, 100));
    let test = child(
        &mut tree,
        root,
        BoxStyle {
            flex_basis: Dimension::px(-50),
            ..height_only(100)
        },
    );
    let reference = child(
        &mut tree,
        root,
        BoxStyle {
            width: Dimension::px(50),
            ..height_only(100)
        },
    );
    layout(&mut tree, root, 100);
    assert_eq!(rect(&tree, test).2, 0.0);
    assert_eq!(rect(&tree, reference), (0.0, 0.0, 50.0, 100.0));
}

#[test]
fn order_starts_with_the_lowest_order_item() {
    // flexbox-order-from-lowest.html: items are laid out by `order`, not by document order.
    let mut tree = BoxTree::new();
    let root = tree.insert(flex_box(300, 20));
    let rightmost = child(
        &mut tree,
        root,
        BoxStyle {
            order: 1,
            ..boxed(50, 20)
        },
    );
    let middle = child(
        &mut tree,
        root,
        BoxStyle {
            order: 0,
            ..boxed(50, 20)
        },
    );
    let leftmost = child(
        &mut tree,
        root,
        BoxStyle {
            order: -1,
            ..boxed(50, 20)
        },
    );
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, leftmost).0, 0.0);
    assert_eq!(rect(&tree, middle).0, 50.0);
    assert_eq!(rect(&tree, rightmost).0, 100.0);
}

#[test]
fn flex_shrink_is_weighted_by_factor_and_basis() {
    // flex-shrink-001.html: 100 px of overflow shared 2:3 by two 100 px items.
    let mut tree = BoxTree::new();
    let root = tree.insert(flex_box(100, 100));
    let test1 = child(
        &mut tree,
        root,
        BoxStyle {
            flex_shrink: 2.0,
            ..boxed(100, 100)
        },
    );
    let test2 = child(
        &mut tree,
        root,
        BoxStyle {
            flex_shrink: 3.0,
            ..boxed(100, 100)
        },
    );
    layout(&mut tree, root, 100);
    assert_eq!(rect(&tree, test1), (0.0, 0.0, 60.0, 100.0));
    assert_eq!(rect(&tree, test2), (60.0, 0.0, 40.0, 100.0));
}

#[test]
fn flex_grow_zero_items_do_not_take_free_space() {
    // flex-grow-002.html: only the item with grow 1 takes the 40 px of free space.
    let mut tree = BoxTree::new();
    let root = tree.insert(flex_box(100, 100));
    let test1 = child(
        &mut tree,
        root,
        BoxStyle {
            flex_grow: 1.0,
            ..boxed(20, 100)
        },
    );
    let test2 = child(&mut tree, root, boxed(20, 100));
    let test3 = child(&mut tree, root, boxed(20, 100));
    layout(&mut tree, root, 100);
    assert_eq!(rect(&tree, test1), (0.0, 0.0, 60.0, 100.0));
    assert_eq!(rect(&tree, test2).0, 60.0);
    assert_eq!(rect(&tree, test3).0, 80.0);
}

#[test]
fn wrap_reverse_stacks_the_first_line_at_the_bottom() {
    // flexbox-flex-wrap-wrap-reverse.htm: a 3x3 grid of 40 px boxes in a 120 px square. Lines
    // stack from the bottom, so items 1 to 3 form the bottom row.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        flex_wrap: FlexWrap::WrapReverse,
        ..flex_box(120, 120)
    });
    let items: Vec<_> = (0..9)
        .map(|_| child(&mut tree, root, boxed(40, 40)))
        .collect();
    layout(&mut tree, root, 120);
    assert_eq!(rect(&tree, items[0]), (0.0, 80.0, 40.0, 40.0));
    assert_eq!(rect(&tree, items[2]), (80.0, 80.0, 40.0, 40.0));
    assert_eq!(rect(&tree, items[3]), (0.0, 40.0, 40.0, 40.0));
    assert_eq!(rect(&tree, items[8]), (80.0, 0.0, 40.0, 40.0));
}

#[test]
fn row_wrap_reverse_puts_the_first_line_at_the_bottom() {
    // flex-flow-003.html: DOM order 3, 4, 1, 2 in a 100 px square with `row wrap-reverse`. The
    // visible order is 1 2 / 3 4, so the first line (3 and 4) sits at the bottom.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        flex_wrap: FlexWrap::WrapReverse,
        ..flex_box(100, 100)
    });
    let three = child(&mut tree, root, boxed(50, 50));
    let four = child(&mut tree, root, boxed(50, 50));
    let one = child(&mut tree, root, boxed(50, 50));
    let two = child(&mut tree, root, boxed(50, 50));
    layout(&mut tree, root, 100);
    assert_eq!(rect(&tree, one), (0.0, 0.0, 50.0, 50.0));
    assert_eq!(rect(&tree, two), (50.0, 0.0, 50.0, 50.0));
    assert_eq!(rect(&tree, three), (0.0, 50.0, 50.0, 50.0));
    assert_eq!(rect(&tree, four), (50.0, 50.0, 50.0, 50.0));
}
