//! Chainable `BoxStyle` setters and the closure-based `BoxTree::build`.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, Edges, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, Px, WritingMode,
};

#[test]
fn setters_set_only_their_field() {
    let style = BoxStyle::default()
        .width(Dimension::px(200))
        .display(Display::Flex)
        .padding(Edges::all(Dimension::px(4)));
    let plain = BoxStyle::default();

    assert_eq!(style.width, Dimension::px(200));
    assert_eq!(style.display, Display::Flex);
    assert_eq!(style.padding, Edges::all(Dimension::px(4)));
    // Fields that were not set keep their defaults.
    assert_eq!(style.height, plain.height);
    assert_eq!(style.writing_mode, WritingMode::Inherit);
}

#[test]
fn build_appends_children_in_order() {
    let mut tree = BoxTree::new();
    let root = tree.build(BoxStyle::default(), |scope| {
        scope.text(BoxStyle::default(), "first");
        scope.leaf(BoxStyle::default());
        scope.element(BoxStyle::default().display(Display::Flex), |inner| {
            inner.text(BoxStyle::default(), "nested");
        });
    });

    let children = tree.children(root).to_vec();
    assert_eq!(children.len(), 3);
    assert_eq!(
        tree.get(children[0]).unwrap().text.as_deref(),
        Some("first")
    );
    assert!(tree.children(children[1]).is_empty());
    let nested = tree.children(children[2]);
    assert_eq!(nested.len(), 1);
    assert_eq!(tree.get(nested[0]).unwrap().text.as_deref(), Some("nested"));
}

#[test]
fn built_tree_lays_out_like_a_hand_built_one() {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let card = || BoxStyle::default().height(Dimension::px(30));

    let mut built = BoxTree::new();
    let root_built = built.build(BoxStyle::default().width(Dimension::px(100)), |scope| {
        scope.leaf(card());
        scope.leaf(card());
    });

    let mut manual = BoxTree::new();
    let root_manual = manual.insert(BoxStyle::default().width(Dimension::px(100)));
    for _ in 0..2 {
        let child = manual.insert(card());
        manual.append_child(root_manual, child).unwrap();
    }

    let constraints = LayoutConstraints {
        width: Px::from_px(100),
        height: None,
    };
    layout_document(&mut built, root_built, constraints, &ctx).unwrap();
    layout_document(&mut manual, root_manual, constraints, &ctx).unwrap();

    let a = built.get(root_built).unwrap().layout;
    let b = manual.get(root_manual).unwrap().layout;
    assert_eq!((a.width, a.height), (b.width, b.height));
    assert_eq!(a.height, 60.0);
}
