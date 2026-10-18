//! Vertical writing modes. Expected positions are worked out by hand in physical (page) terms.

mod common;

use common::*;
use tpt_layout::{BoxStyle, BoxTree, Dimension, Edges, WritingMode};

fn vertical(mode: WritingMode, width: i32, height: i32) -> BoxStyle {
    BoxStyle {
        writing_mode: mode,
        width: Dimension::px(width),
        height: Dimension::px(height),
        ..BoxStyle::default()
    }
}

#[test]
fn vertical_rl_blocks_run_right_to_left() {
    // V is 100 px wide and 200 px tall. Its blocks stack from the right edge.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let v = child(&mut tree, root, vertical(WritingMode::VerticalRl, 100, 200));
    // Blocks with an auto inline size fill the container. The inline axis is vertical here, so each
    // block is the full 200 px tall and 20 px across (the line height).
    let first = text_child(&mut tree, v, BoxStyle::default(), "abc");
    let second = text_child(&mut tree, v, BoxStyle::default(), "ab");
    layout(&mut tree, root, 300);

    assert_eq!(rect(&tree, v), (0.0, 0.0, 100.0, 200.0));
    // The first block sits at the physical right edge; the second is to its left.
    assert_eq!(rect(&tree, first), (80.0, 0.0, 20.0, 200.0));
    assert_eq!(rect(&tree, second), (60.0, 0.0, 20.0, 200.0));
}

#[test]
fn vertical_lr_blocks_run_left_to_right() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let v = child(&mut tree, root, vertical(WritingMode::VerticalLr, 100, 200));
    let first = text_child(&mut tree, v, BoxStyle::default(), "abc");
    let second = text_child(&mut tree, v, BoxStyle::default(), "ab");
    layout(&mut tree, root, 300);

    assert_eq!(rect(&tree, first), (0.0, 0.0, 20.0, 200.0));
    assert_eq!(rect(&tree, second), (20.0, 0.0, 20.0, 200.0));
}

#[test]
fn vertical_boxes_stack_vertically_in_a_horizontal_flow() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let a = child(&mut tree, root, vertical(WritingMode::VerticalRl, 100, 200));
    let b = child(&mut tree, root, vertical(WritingMode::VerticalRl, 100, 200));
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, a).1, 0.0);
    assert_eq!(rect(&tree, b).1, 200.0);
}

#[test]
fn padding_maps_to_the_logical_sides() {
    // Physical top padding is the logical inline-start padding, so the content starts 10 px down.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let style = BoxStyle {
        padding: Edges {
            top: Dimension::px(10),
            ..Edges::all(Dimension::ZERO)
        },
        ..vertical(WritingMode::VerticalRl, 100, 200)
    };
    let v = child(&mut tree, root, style);
    let text = text_child(&mut tree, v, BoxStyle::default(), "abc");
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, text).1, 10.0);
    // Content-box: the 200 px height is the content, so the border box is 210 px tall.
    assert_eq!(rect(&tree, v), (0.0, 0.0, 100.0, 210.0));
}

#[test]
fn styles_are_restored_after_layout() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let v = child(&mut tree, root, vertical(WritingMode::VerticalRl, 100, 200));
    let inner = child(&mut tree, v, boxed(30, 10));
    layout(&mut tree, root, 300);
    // The physical styles are back in place, so a second pass gives the same answer.
    assert_eq!(tree.get(inner).unwrap().style.width, Dimension::px(30));
    assert_eq!(tree.get(v).unwrap().style.width, Dimension::px(100));
    let first = rect(&tree, inner);
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, inner), first);
}

fn horizontal(width: i32, height: i32) -> BoxStyle {
    BoxStyle {
        writing_mode: WritingMode::HorizontalTb,
        ..boxed(width, height)
    }
}

#[test]
fn horizontal_box_inside_vertical_rl_is_a_root_of_its_own() {
    // V is 100 px wide and 200 px tall. H is a 60x30 horizontal box inside it. Its block extent is
    // its page width, so it sits at the right edge of V, and its 30 px height is its inline extent.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let v = child(&mut tree, root, vertical(WritingMode::VerticalRl, 100, 200));
    let h = child(&mut tree, v, horizontal(60, 30));
    layout(&mut tree, root, 300);

    assert_eq!(rect(&tree, v), (0.0, 0.0, 100.0, 200.0));
    assert_eq!(rect(&tree, h), (40.0, 0.0, 60.0, 30.0));
    // The box's own physical style is back in place.
    assert_eq!(tree.get(h).unwrap().style.width, Dimension::px(60));
    assert_eq!(tree.get(h).unwrap().style.height, Dimension::px(30));
}

#[test]
fn horizontal_blocks_inside_vertical_rl_stack_right_to_left() {
    // The second horizontal block stacks to the left of the first one, in the block direction.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let v = child(&mut tree, root, vertical(WritingMode::VerticalRl, 100, 200));
    let first = child(&mut tree, v, horizontal(60, 30));
    let second = child(&mut tree, v, horizontal(40, 30));
    layout(&mut tree, root, 300);

    assert_eq!(rect(&tree, first), (40.0, 0.0, 60.0, 30.0));
    assert_eq!(rect(&tree, second), (0.0, 0.0, 40.0, 30.0));
}

#[test]
fn horizontal_box_inside_vertical_lr_is_at_the_left_edge() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let v = child(&mut tree, root, vertical(WritingMode::VerticalLr, 100, 200));
    let h = child(&mut tree, v, horizontal(60, 30));
    layout(&mut tree, root, 300);

    assert_eq!(rect(&tree, h), (0.0, 0.0, 60.0, 30.0));
}

#[test]
fn vertical_lr_box_inside_vertical_rl_is_a_nested_root() {
    // V2 is 50 px wide and 80 px tall on the page. Inside V (vertical-rl) it is the first block, so
    // it sits at the right edge, and it keeps its physical size.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let v = child(&mut tree, root, vertical(WritingMode::VerticalRl, 100, 200));
    let v2 = child(&mut tree, v, vertical(WritingMode::VerticalLr, 50, 80));
    layout(&mut tree, root, 300);

    assert_eq!(rect(&tree, v2), (50.0, 0.0, 50.0, 80.0));
}

#[test]
fn inherit_keeps_the_parents_mode() {
    // Children that do not set a mode take the vertical root's mode, so they stack as before.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let v = child(&mut tree, root, vertical(WritingMode::VerticalRl, 100, 200));
    let a = child(&mut tree, v, boxed(60, 30));
    layout(&mut tree, root, 300);

    assert_eq!(rect(&tree, a), (40.0, 0.0, 60.0, 30.0));
}
