//! Flex `min-width: auto` is the item's min-content width, so a long word stops it shrinking.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Display, FixedAdvanceMeasurer, LayoutConstraints,
    LayoutContext, Overflow, Px,
};

/// Two flex items in a 60 px row, each holding one paragraph. Each item's text is
/// `"hello wonderful"`, whose widest word ("wonderful", 9 chars) is 72 px at 8 px per char.
fn item_widths(overflow: Overflow) -> (f32, f32) {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        display: Display::Flex,
        ..BoxStyle::default()
    });
    let mut items = Vec::new();
    for _ in 0..2 {
        let item = tree.insert(BoxStyle {
            flex_shrink: 1.0,
            overflow,
            ..BoxStyle::default()
        });
        let text = tree.insert_text(
            BoxStyle {
                display: Display::Inline,
                ..BoxStyle::default()
            },
            "hello wonderful",
        );
        tree.append_child(item, text).unwrap();
        tree.append_child(root, item).unwrap();
        items.push(item);
    }
    layout_document(
        &mut tree,
        root,
        LayoutConstraints {
            width: Px::from_px(60),
            height: None,
        },
        &ctx,
    )
    .unwrap();
    (
        tree.get(items[0]).unwrap().layout.width,
        tree.get(items[1]).unwrap().layout.width,
    )
}

#[test]
fn auto_min_keeps_items_at_least_their_longest_word() {
    let (a, b) = item_widths(Overflow::Visible);
    assert!(a >= 72.0, "first item {a} shrank below its longest word");
    assert!(b >= 72.0, "second item {b} shrank below its longest word");
}

#[test]
fn overflow_hidden_drops_the_auto_minimum() {
    let (a, b) = item_widths(Overflow::Hidden);
    assert!(
        a < 72.0 && b < 72.0,
        "items should shrink with min 0: {a}, {b}"
    );
}
