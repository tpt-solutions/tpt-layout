//! `aspect-ratio` for auto heights, plus `position: fixed` (out of flow) and `position: sticky`
//! (offset like relative, never sticking).

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Edges, FixedAdvanceMeasurer, LayoutConstraints,
    LayoutContext, Position, Px,
};

fn measurer() -> FixedAdvanceMeasurer {
    FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12))
}

fn layout(tree: &mut BoxTree, root: tpt_layout::NodeId) -> f32 {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let result = layout_document(
        tree,
        root,
        LayoutConstraints {
            width: Px::from_px(200),
            height: None,
        },
        &ctx,
    )
    .unwrap();
    result.height.to_f32()
}

fn px(v: i32) -> Dimension {
    Dimension::Length(Px::from_px(v))
}

#[test]
fn auto_height_follows_the_aspect_ratio() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let media = tree.insert(BoxStyle {
        width: px(100),
        aspect_ratio: Some(2.0),
        ..BoxStyle::default()
    });
    tree.append_child(root, media).unwrap();
    // Width 100 at ratio 2 is 50 tall.
    assert_eq!(layout(&mut tree, root), 50.0);
    assert_eq!(tree.get(media).unwrap().layout.height, 50.0);
}

#[test]
fn explicit_height_wins_over_aspect_ratio() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let media = tree.insert(BoxStyle {
        width: px(100),
        height: px(30),
        aspect_ratio: Some(2.0),
        ..BoxStyle::default()
    });
    tree.append_child(root, media).unwrap();
    assert_eq!(layout(&mut tree, root), 30.0);
}

#[test]
fn no_aspect_ratio_leaves_an_empty_box_at_zero_height() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let media = tree.insert(BoxStyle {
        width: px(100),
        ..BoxStyle::default()
    });
    tree.append_child(root, media).unwrap();
    assert_eq!(layout(&mut tree, root), 0.0);
}

#[test]
fn fixed_box_is_out_of_flow() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let text = tree.insert(BoxStyle {
        height: px(20),
        ..BoxStyle::default()
    });
    tree.append_child(root, text).unwrap();
    let fixed = tree.insert(BoxStyle {
        position: Position::Fixed,
        height: px(100),
        inset: Edges {
            top: px(5),
            right: Dimension::Auto,
            bottom: Dimension::Auto,
            left: px(7),
        },
        ..BoxStyle::default()
    });
    tree.append_child(root, fixed).unwrap();
    // The fixed box takes no space: the root is only as tall as the in-flow child.
    assert_eq!(layout(&mut tree, root), 20.0);
    let placed = tree.get(fixed).unwrap().layout;
    assert_eq!((placed.x, placed.y), (7.0, 5.0));
}

#[test]
fn sticky_offsets_like_relative() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let first = tree.insert(BoxStyle {
        height: px(20),
        ..BoxStyle::default()
    });
    let sticky = tree.insert(BoxStyle {
        position: Position::Sticky,
        height: px(20),
        inset: Edges {
            top: px(10),
            right: Dimension::Auto,
            bottom: Dimension::Auto,
            left: Dimension::Auto,
        },
        ..BoxStyle::default()
    });
    tree.append_child(root, first).unwrap();
    tree.append_child(root, sticky).unwrap();
    // The sticky box keeps its flow position (y = 20) and is offset down by its top inset.
    layout(&mut tree, root);
    assert_eq!(tree.get(sticky).unwrap().layout.y, 30.0);
}
