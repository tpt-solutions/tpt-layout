//! `direction` and `text-align` are inherited from the containing block unless a box sets them.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Direction, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Px, TextAlign,
};

fn px(v: i32) -> Dimension {
    Dimension::Length(Px::from_px(v))
}

fn layout(tree: &mut BoxTree, root: NodeId, width: i32) {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    layout_document(
        tree,
        root,
        LayoutConstraints {
            width: Px::from_px(width),
            height: None,
        },
        &ctx,
    )
    .unwrap();
}

/// A paragraph block with inline `text` inside `parent`. Returns the paragraph and its text.
fn paragraph(tree: &mut BoxTree, parent: NodeId, style: BoxStyle, text: &str) -> (NodeId, NodeId) {
    let block = tree.insert(style);
    tree.append_child(parent, block).unwrap();
    let inline = tree.insert_text(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        text,
    );
    tree.append_child(block, inline).unwrap();
    (block, inline)
}

#[test]
fn nested_block_inherits_rtl_from_the_root() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: px(100),
        direction: Direction::Rtl,
        ..BoxStyle::default()
    });
    // A nested block sets nothing, so it is right-to-left: "ab" (16 px) sits at the right edge.
    let (_, text) = paragraph(&mut tree, root, BoxStyle::default(), "ab");
    layout(&mut tree, root, 100);
    assert_eq!(tree.get(text).unwrap().fragments[0].x, Px::from_px(84));
}

#[test]
fn a_block_can_override_the_inherited_direction() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: px(100),
        direction: Direction::Rtl,
        ..BoxStyle::default()
    });
    let (_, text) = paragraph(
        &mut tree,
        root,
        BoxStyle {
            direction: Direction::Ltr,
            ..BoxStyle::default()
        },
        "ab",
    );
    layout(&mut tree, root, 100);
    assert_eq!(tree.get(text).unwrap().fragments[0].x, Px::ZERO);
}

#[test]
fn text_align_is_inherited_by_nested_blocks() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: px(100),
        text_align: TextAlign::Center,
        ..BoxStyle::default()
    });
    let (_, text) = paragraph(&mut tree, root, BoxStyle::default(), "ab");
    layout(&mut tree, root, 100);
    // Centred: (100 - 16) / 2 = 42.
    assert_eq!(tree.get(text).unwrap().fragments[0].x, Px::from_px(42));
}

#[test]
fn a_flex_row_inherits_rtl_and_starts_on_the_right() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: px(100),
        direction: Direction::Rtl,
        ..BoxStyle::default()
    });
    let row = tree.insert(BoxStyle {
        display: Display::Flex,
        ..BoxStyle::default()
    });
    tree.append_child(root, row).unwrap();
    let mut items = Vec::new();
    for _ in 0..2 {
        let item = tree.insert(BoxStyle {
            width: px(10),
            height: px(10),
            ..BoxStyle::default()
        });
        tree.append_child(row, item).unwrap();
        items.push(item);
    }
    layout(&mut tree, root, 100);
    // The first item starts at the right edge; the second is to its left.
    assert_eq!(tree.get(items[0]).unwrap().layout.x, 90.0);
    assert_eq!(tree.get(items[1]).unwrap().layout.x, 80.0);
}
