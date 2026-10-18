//! Percentage sizes of writing-mode roots. A root whose caller is horizontal resolves its
//! percentages against that caller. A nested root (its caller is also a vertical root) has no
//! containing block, so its percentages do not resolve, as documented in `src/writing.rs`.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, FixedAdvanceMeasurer, LayoutConstraints,
    LayoutContext, Px, WritingMode,
};

fn measurer() -> FixedAdvanceMeasurer {
    FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12))
}

fn layout(tree: &mut BoxTree, root: tpt_layout::NodeId, width: i32, height: i32) {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    layout_document(
        tree,
        root,
        LayoutConstraints {
            width: Px::from_px(width),
            height: Some(Px::from_px(height)),
        },
        &ctx,
    )
    .unwrap();
}

#[test]
fn percent_size_of_a_vertical_root_resolves_against_a_horizontal_caller() {
    let mut tree = BoxTree::new();
    let outer = tree.insert(BoxStyle {
        width: Dimension::px(200),
        height: Dimension::px(200),
        ..BoxStyle::default()
    });
    let vertical = tree.insert(BoxStyle {
        writing_mode: WritingMode::VerticalRl,
        width: Dimension::percent(50.0),
        height: Dimension::percent(50.0),
        ..BoxStyle::default()
    });
    tree.append_child(outer, vertical).unwrap();

    layout(&mut tree, outer, 200, 200);

    let l = tree.get(vertical).unwrap().layout;
    assert_eq!((l.width, l.height), (100.0, 100.0));
}

#[test]
fn percent_size_inside_a_root_resolves_against_that_root() {
    let mut tree = BoxTree::new();
    let outer = tree.insert(BoxStyle {
        width: Dimension::px(200),
        height: Dimension::px(200),
        ..BoxStyle::default()
    });
    let vertical = tree.insert(BoxStyle {
        writing_mode: WritingMode::VerticalRl,
        width: Dimension::px(100),
        height: Dimension::px(100),
        ..BoxStyle::default()
    });
    let child = tree.insert(BoxStyle {
        width: Dimension::percent(50.0),
        height: Dimension::px(10),
        ..BoxStyle::default()
    });
    tree.append_child(outer, vertical).unwrap();
    tree.append_child(vertical, child).unwrap();

    layout(&mut tree, outer, 200, 200);

    let l = tree.get(child).unwrap().layout;
    assert_eq!(l.width, 50.0);
}

#[test]
fn percent_size_of_a_nested_root_does_not_resolve_against_a_vertical_caller() {
    let mut tree = BoxTree::new();
    let outer = tree.insert(BoxStyle {
        writing_mode: WritingMode::VerticalRl,
        width: Dimension::px(200),
        height: Dimension::px(200),
        ..BoxStyle::default()
    });
    // A horizontal root inside a vertical one: its caller is vertical, so it is nested.
    let nested = tree.insert(BoxStyle {
        writing_mode: WritingMode::HorizontalTb,
        width: Dimension::percent(50.0),
        height: Dimension::percent(50.0),
        ..BoxStyle::default()
    });
    let child = tree.insert(BoxStyle {
        height: Dimension::px(10),
        ..BoxStyle::default()
    });
    tree.append_child(outer, nested).unwrap();
    tree.append_child(nested, child).unwrap();

    layout(&mut tree, outer, 200, 200);

    // Had 50% resolved against the 200 px outer box, the nested root would be 100 px tall. It
    // takes its height from its content instead.
    let l = tree.get(nested).unwrap().layout;
    assert_eq!(l.height, 10.0);
}
