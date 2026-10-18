//! Flexbox. Expected values follow the CSS Flexbox spec's worked examples where one exists.

mod common;

use common::*;
use tpt_layout::{
    AlignItems, BoxStyle, BoxTree, Dimension, Display, Edges, FlexDirection, FlexWrap,
    JustifyContent,
};

fn flex_root(tree: &mut BoxTree, style: BoxStyle) -> tpt_layout::NodeId {
    tree.insert(BoxStyle {
        display: Display::Flex,
        ..style
    })
}

fn item(width: i32, height: i32) -> BoxStyle {
    boxed(width, height)
}

#[test]
fn equal_grow_splits_free_space() {
    let mut tree = BoxTree::new();
    let root = flex_root(&mut tree, BoxStyle::default());
    let mut ids = Vec::new();
    for _ in 0..3 {
        let style = BoxStyle {
            flex_basis: Dimension::px(0),
            flex_grow: 1.0,
            height: Dimension::px(20),
            ..BoxStyle::default()
        };
        ids.push(child(&mut tree, root, style));
    }
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, ids[0]).0, 0.0);
    assert_eq!(rect(&tree, ids[1]).0, 100.0);
    assert_eq!(rect(&tree, ids[2]).0, 200.0);
    assert_eq!(rect(&tree, ids[2]).2, 100.0);
}

#[test]
fn shrink_is_weighted_by_basis() {
    // Three 200 px items in 300 px: each shrinks by 100 px (equal shrink, equal basis).
    let mut tree = BoxTree::new();
    let root = flex_root(&mut tree, BoxStyle::default());
    let ids: Vec<_> = (0..3)
        .map(|_| {
            child(
                &mut tree,
                root,
                BoxStyle {
                    flex_basis: Dimension::px(200),
                    height: Dimension::px(20),
                    ..BoxStyle::default()
                },
            )
        })
        .collect();
    layout(&mut tree, root, 300);
    for id in ids {
        assert_eq!(rect(&tree, id).2, 100.0);
    }
}

#[test]
fn justify_center_and_space_between() {
    for (justify, expected) in [
        (JustifyContent::Center, [75.0, 125.0, 175.0]),
        (JustifyContent::SpaceBetween, [0.0, 125.0, 250.0]),
    ] {
        let mut tree = BoxTree::new();
        let root = flex_root(
            &mut tree,
            BoxStyle {
                justify_content: justify,
                ..BoxStyle::default()
            },
        );
        let ids: Vec<_> = (0..3)
            .map(|_| child(&mut tree, root, item(50, 20)))
            .collect();
        layout(&mut tree, root, 300);
        let xs: Vec<f32> = ids.iter().map(|id| rect(&tree, *id).0).collect();
        assert_eq!(xs, expected, "{justify:?}");
    }
}

#[test]
fn column_direction_stacks_with_gap() {
    let mut tree = BoxTree::new();
    let root = flex_root(
        &mut tree,
        BoxStyle {
            flex_direction: FlexDirection::Column,
            row_gap: Dimension::px(10),
            ..BoxStyle::default()
        },
    );
    let a = child(&mut tree, root, item(50, 20));
    let b = child(&mut tree, root, item(50, 20));
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, a).1, 0.0);
    assert_eq!(rect(&tree, b).1, 30.0);
    assert_eq!(tree.get(root).unwrap().layout.height, 50.0);
}

#[test]
fn wrap_breaks_into_lines() {
    let mut tree = BoxTree::new();
    let root = flex_root(
        &mut tree,
        BoxStyle {
            flex_wrap: FlexWrap::Wrap,
            ..BoxStyle::default()
        },
    );
    let ids: Vec<_> = (0..3)
        .map(|_| child(&mut tree, root, item(150, 20)))
        .collect();
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, ids[0]).0, 0.0);
    assert_eq!(rect(&tree, ids[1]).0, 150.0);
    assert_eq!((rect(&tree, ids[2]).0, rect(&tree, ids[2]).1), (0.0, 20.0));
    assert_eq!(tree.get(root).unwrap().layout.height, 40.0);
}

#[test]
fn wrap_reverse_stacks_lines_bottom_up() {
    let mut tree = BoxTree::new();
    let root = flex_root(
        &mut tree,
        BoxStyle {
            flex_wrap: FlexWrap::WrapReverse,
            height: Dimension::px(100),
            ..BoxStyle::default()
        },
    );
    let first = child(&mut tree, root, item(300, 20));
    let second = child(&mut tree, root, item(300, 20));
    layout(&mut tree, root, 300);
    // Lines stretch to 50 px each. The first line is at the bottom of the container, and its
    // item sits on that line's cross-start edge (the bottom, when reversed). The second line is
    // above it, so its item is at y = 30.
    assert_eq!(rect(&tree, first).1, 80.0);
    assert_eq!(rect(&tree, second).1, 30.0);
}

#[test]
fn align_items_center_and_self_override() {
    let mut tree = BoxTree::new();
    let root = flex_root(
        &mut tree,
        BoxStyle {
            height: Dimension::px(100),
            align_items: AlignItems::Center,
            ..BoxStyle::default()
        },
    );
    let centered = child(&mut tree, root, item(50, 20));
    let pinned = child(
        &mut tree,
        root,
        BoxStyle {
            align_self: Some(AlignItems::FlexEnd),
            ..item(50, 20)
        },
    );
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, centered).1, 40.0);
    assert_eq!(rect(&tree, pinned).1, 80.0);
}

#[test]
fn order_reorders_items() {
    let mut tree = BoxTree::new();
    let root = flex_root(&mut tree, BoxStyle::default());
    let late = child(
        &mut tree,
        root,
        BoxStyle {
            order: 2,
            ..item(50, 20)
        },
    );
    let early = child(
        &mut tree,
        root,
        BoxStyle {
            order: 1,
            ..item(50, 20)
        },
    );
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, early).0, 0.0);
    assert_eq!(rect(&tree, late).0, 50.0);
}

#[test]
fn row_reverse_places_first_item_on_the_right() {
    let mut tree = BoxTree::new();
    let root = flex_root(
        &mut tree,
        BoxStyle {
            flex_direction: FlexDirection::RowReverse,
            ..BoxStyle::default()
        },
    );
    let first = child(&mut tree, root, item(50, 20));
    let second = child(&mut tree, root, item(50, 20));
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, first).0, 250.0);
    assert_eq!(rect(&tree, second).0, 200.0);
}

#[test]
fn rtl_row_starts_on_the_right() {
    let mut tree = BoxTree::new();
    let root = flex_root(
        &mut tree,
        BoxStyle {
            direction: tpt_layout::Direction::Rtl,
            ..BoxStyle::default()
        },
    );
    let first = child(&mut tree, root, item(50, 20));
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, first).0, 250.0);
}

#[test]
fn auto_margin_pushes_item_to_the_end() {
    let mut tree = BoxTree::new();
    let root = flex_root(&mut tree, BoxStyle::default());
    let style = BoxStyle {
        margin: Edges {
            left: Dimension::Auto,
            right: Dimension::ZERO,
            top: Dimension::ZERO,
            bottom: Dimension::ZERO,
        },
        ..item(50, 20)
    };
    let pushed = child(&mut tree, root, style);
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, pushed).0, 250.0);
}

#[test]
fn column_flex_grow_fills_definite_height() {
    let mut tree = BoxTree::new();
    let root = flex_root(
        &mut tree,
        BoxStyle {
            flex_direction: FlexDirection::Column,
            height: Dimension::px(100),
            ..BoxStyle::default()
        },
    );
    let fixed = child(&mut tree, root, item(50, 20));
    let grow = child(
        &mut tree,
        root,
        BoxStyle {
            flex_grow: 1.0,
            width: Dimension::px(50),
            ..BoxStyle::default()
        },
    );
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, fixed).3, 20.0);
    assert_eq!(rect(&tree, grow).1, 20.0);
    assert_eq!(rect(&tree, grow).3, 80.0);
}

#[test]
fn justify_flex_end() {
    let mut tree = BoxTree::new();
    let root = flex_root(
        &mut tree,
        BoxStyle {
            justify_content: JustifyContent::FlexEnd,
            ..BoxStyle::default()
        },
    );
    let id = child(&mut tree, root, item(50, 20));
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, id).0, 250.0);
}
