//! Relative and absolute positioning.

use tpt_layout::{
    layout_flow, BoxStyle, BoxTree, Dimension, Edges, FixedAdvanceMeasurer, Position, Px,
};

fn measurer() -> FixedAdvanceMeasurer {
    FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(20), Px::from_px(15))
}

#[test]
fn relative_offset_moves_box_without_affecting_siblings() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let style = BoxStyle {
        position: Position::Relative,
        inset: Edges {
            top: Dimension::px(5),
            left: Dimension::px(3),
            ..Edges::all(Dimension::Auto)
        },
        ..BoxStyle::default()
    };
    let shifted = tree.insert_text(style, "moved");
    let sibling = tree.insert_text(BoxStyle::default(), "after");
    tree.append_child(root, shifted).unwrap();
    tree.append_child(root, sibling).unwrap();
    layout_flow(&mut tree, root, Px::from_px(400), &measurer()).unwrap();

    let shifted = tree.get(shifted).unwrap().layout;
    assert_eq!((shifted.x, shifted.y), (3.0, 5.0));
    // The sibling sits where the unshifted box ended: at y = 20.
    assert_eq!(tree.get(sibling).unwrap().layout.y, 20.0);
}

#[test]
fn absolute_child_anchors_to_positioned_ancestor() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let container_style = BoxStyle {
        position: Position::Relative,
        width: Dimension::px(300),
        height: Dimension::px(200),
        margin: Edges {
            top: Dimension::px(10),
            ..Edges::all(Dimension::ZERO)
        },
        ..BoxStyle::default()
    };
    let container = tree.insert(container_style);
    tree.append_child(root, container).unwrap();

    let abs_style = BoxStyle {
        position: Position::Absolute,
        width: Dimension::px(50),
        height: Dimension::px(30),
        inset: Edges {
            top: Dimension::px(10),
            right: Dimension::px(10),
            ..Edges::all(Dimension::Auto)
        },
        ..BoxStyle::default()
    };
    let abs = tree.insert(abs_style);
    tree.append_child(container, abs).unwrap();
    layout_flow(&mut tree, root, Px::from_px(400), &measurer()).unwrap();

    let container = tree.get(container).unwrap().layout;
    assert_eq!((container.x, container.y), (0.0, 10.0));

    let abs = tree.get(abs).unwrap().layout;
    // Right edge 10 px inside a 300 px container: x = 300 - 10 - 50 = 240.
    assert_eq!(
        (abs.x, abs.y, abs.width, abs.height),
        (240.0, 20.0, 50.0, 30.0)
    );
    // The absolute child is out of flow, so the root is only as tall as the container.
    assert_eq!(tree.get(root).unwrap().layout.height, 210.0);
}

#[test]
fn bottom_anchored_auto_height_box_is_placed_from_the_bottom() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let container = tree.insert(BoxStyle {
        position: Position::Relative,
        height: Dimension::px(100),
        ..BoxStyle::default()
    });
    tree.append_child(root, container).unwrap();

    let abs = tree.insert_text(
        BoxStyle {
            position: Position::Absolute,
            inset: Edges {
                bottom: Dimension::px(10),
                ..Edges::all(Dimension::Auto)
            },
            ..BoxStyle::default()
        },
        "hello",
    );
    tree.append_child(container, abs).unwrap();
    layout_flow(&mut tree, root, Px::from_px(400), &measurer()).unwrap();

    // Container height 100 - bottom inset 10 - one 20 px line = top at 70.
    assert_eq!(tree.get(abs).unwrap().layout.y, 70.0);
}

#[test]
fn absolute_box_without_insets_uses_static_position() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let abs = tree.insert_text(
        BoxStyle {
            position: Position::Absolute,
            ..BoxStyle::default()
        },
        "float",
    );
    let after = tree.insert_text(BoxStyle::default(), "after");
    tree.append_child(root, abs).unwrap();
    tree.append_child(root, after).unwrap();
    layout_flow(&mut tree, root, Px::from_px(400), &measurer()).unwrap();

    // The absolute box does not take up space, so the next box starts at the top.
    assert_eq!(tree.get(after).unwrap().layout.y, 0.0);
    let abs = tree.get(abs).unwrap().layout;
    assert_eq!((abs.x, abs.y), (0.0, 0.0));
}
