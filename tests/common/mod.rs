//! Shared helpers for the integration tests.

#![allow(dead_code)]

use tpt_layout::{layout_flow, BoxStyle, BoxTree, Dimension, FixedAdvanceMeasurer, NodeId, Px};

pub fn px(v: i32) -> Px {
    Px::from_px(v)
}

/// 10 px per character, 20 px lines, baseline 15 px below the top of each line.
pub fn measurer() -> FixedAdvanceMeasurer {
    FixedAdvanceMeasurer::new(px(10), px(20), px(15))
}

pub fn layout(tree: &mut BoxTree, root: NodeId, width: i32) {
    layout_flow(tree, root, px(width), &measurer()).expect("layout");
}

/// A block-level box with a fixed size.
pub fn boxed(width: i32, height: i32) -> BoxStyle {
    BoxStyle {
        width: Dimension::px(width),
        height: Dimension::px(height),
        ..BoxStyle::default()
    }
}

/// Inserts `style` as a child of `parent`.
pub fn child(tree: &mut BoxTree, parent: NodeId, style: BoxStyle) -> NodeId {
    let id = tree.insert(style);
    tree.append_child(parent, id).unwrap();
    id
}

/// Inserts a text leaf as a child of `parent`.
pub fn text_child(tree: &mut BoxTree, parent: NodeId, style: BoxStyle, text: &str) -> NodeId {
    let id = tree.insert_text(style, text);
    tree.append_child(parent, id).unwrap();
    id
}

/// Absolute (x, y) of a node's border box, as f32, plus its size.
pub fn rect(tree: &BoxTree, id: NodeId) -> (f32, f32, f32, f32) {
    let l = tree.get(id).expect("node").layout;
    (l.x, l.y, l.width, l.height)
}

/// A box with a fixed height and an automatic width (so it stretches to its grid area).
pub fn height_only(height: i32) -> BoxStyle {
    BoxStyle {
        height: Dimension::px(height),
        ..BoxStyle::default()
    }
}

/// A grid container with `template`, styled by `style` (its `display` is set to `Grid`).
pub fn grid(
    tree: &mut BoxTree,
    template: tpt_layout::GridTemplate,
    style: BoxStyle,
) -> tpt_layout::NodeId {
    let id = tree.insert(BoxStyle {
        display: tpt_layout::Display::Grid,
        ..style
    });
    tree.set_grid_template(id, template);
    id
}
