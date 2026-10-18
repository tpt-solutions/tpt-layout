//! Margins collapse through an empty first child: a parent's top margin and the empty child's top
//! margin collapse to the larger of the two, not their sum.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Edges, FixedAdvanceMeasurer, LayoutConstraints,
    LayoutContext, Px,
};

fn margin_top(px: i32) -> Edges<Dimension> {
    Edges {
        top: Dimension::Length(Px::from_px(px)),
        right: Dimension::Auto,
        bottom: Dimension::Auto,
        left: Dimension::Auto,
    }
}

#[test]
fn parent_and_empty_first_child_margins_collapse_to_the_larger() {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    // Parent margin 10, empty first child margin 20: they collapse to 20, not 30.
    let parent = tree.insert(BoxStyle {
        margin: margin_top(10),
        ..BoxStyle::default()
    });
    tree.append_child(root, parent).unwrap();
    let empty = tree.insert(BoxStyle {
        margin: margin_top(20),
        ..BoxStyle::default()
    });
    tree.append_child(parent, empty).unwrap();
    let after = tree.insert(BoxStyle {
        height: Dimension::Length(Px::from_px(10)),
        ..BoxStyle::default()
    });
    tree.append_child(root, after).unwrap();

    layout_document(
        &mut tree,
        root,
        LayoutConstraints {
            width: Px::from_px(100),
            height: None,
        },
        &ctx,
    )
    .unwrap();
    assert_eq!(tree.get(parent).unwrap().layout.y, 20.0);
    assert_eq!(tree.get(after).unwrap().layout.y, 20.0);
}

#[test]
fn empty_first_child_margin_alone_moves_the_parent() {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let parent = tree.insert(BoxStyle::default());
    tree.append_child(root, parent).unwrap();
    let empty = tree.insert(BoxStyle {
        margin: margin_top(20),
        ..BoxStyle::default()
    });
    tree.append_child(parent, empty).unwrap();
    let after = tree.insert(BoxStyle {
        height: Dimension::Length(Px::from_px(10)),
        ..BoxStyle::default()
    });
    tree.append_child(root, after).unwrap();

    layout_document(
        &mut tree,
        root,
        LayoutConstraints {
            width: Px::from_px(100),
            height: None,
        },
        &ctx,
    )
    .unwrap();
    // The child's 20 px margin collapses out through the parent and separates it from `after`.
    assert_eq!(tree.get(parent).unwrap().layout.y, 20.0);
    assert_eq!(tree.get(after).unwrap().layout.y, 20.0);
}
