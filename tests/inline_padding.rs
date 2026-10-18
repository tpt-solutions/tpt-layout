//! Margin, border and padding on inline elements take horizontal space on each side.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, Edges, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Px,
};

/// A 100 px block holding a padded span around "ab", followed by the text "cd". Returns the
/// tree and the span, the text inside the span, and the trailing text.
fn build(span_style: BoxStyle) -> (BoxTree, NodeId, NodeId, NodeId) {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let block = tree.insert(BoxStyle {
        width: Dimension::Length(Px::from_px(100)),
        ..BoxStyle::default()
    });
    tree.append_child(root, block).unwrap();
    let span = tree.insert(span_style);
    tree.append_child(block, span).unwrap();
    let inside = tree.insert_text(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        "ab",
    );
    tree.append_child(span, inside).unwrap();
    let after = tree.insert_text(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        " cd",
    );
    tree.append_child(block, after).unwrap();
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
    (tree, span, inside, after)
}

fn px(v: i32) -> Dimension {
    Dimension::Length(Px::from_px(v))
}

#[test]
fn padding_moves_the_text_and_the_next_word() {
    let (tree, span, inside, after) = build(BoxStyle {
        display: Display::Inline,
        padding: Edges {
            top: Dimension::Auto,
            right: px(3),
            bottom: Dimension::Auto,
            left: px(5),
        },
        ..BoxStyle::default()
    });
    // "ab" starts after the 5 px left padding. The span is 5 + 16 + 3 = 24 px wide, and the
    // leading space of " cd" (8 px) sits after it, so "cd" starts at 24 + 8 = 32.
    assert_eq!(tree.get(inside).unwrap().fragments[0].x, Px::from_px(5));
    // The space is not a fragment, so "cd" is the first fragment of its text.
    assert_eq!(tree.get(after).unwrap().fragments[0].x, Px::from_px(32));
    let frame = tree.get(span).unwrap().layout;
    assert_eq!((frame.x, frame.width), (0.0, 24.0));
}

#[test]
fn border_counts_like_padding() {
    let (tree, span, inside, _) = build(BoxStyle {
        display: Display::Inline,
        border: Edges {
            top: Px::ZERO,
            right: Px::ZERO,
            bottom: Px::ZERO,
            left: Px::from_px(2),
        },
        ..BoxStyle::default()
    });
    assert_eq!(tree.get(inside).unwrap().fragments[0].x, Px::from_px(2));
    assert_eq!(tree.get(span).unwrap().layout.width, 18.0);
}

#[test]
fn margin_on_an_inline_element_counts_too() {
    let (tree, span, inside, _) = build(BoxStyle {
        display: Display::Inline,
        margin: Edges {
            top: Dimension::Auto,
            right: Dimension::Auto,
            bottom: Dimension::Auto,
            left: px(4),
        },
        ..BoxStyle::default()
    });
    assert_eq!(tree.get(inside).unwrap().fragments[0].x, Px::from_px(4));
    assert_eq!(tree.get(span).unwrap().layout.width, 20.0);
}

#[test]
fn padding_never_causes_a_line_break() {
    // "abcd" is 32 px in a 36 px block. The span's 6 px padding then overflows the line. A break
    // is not allowed at padding, so the padding stays on line one, and only "x" wraps to line
    // two. Two lines in total, so the second word sits one line (16 px) below the first.
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let block = tree.insert(BoxStyle {
        width: px(36),
        ..BoxStyle::default()
    });
    tree.append_child(root, block).unwrap();
    let word = tree.insert_text(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        "abcd",
    );
    tree.append_child(block, word).unwrap();
    let span = tree.insert(BoxStyle {
        display: Display::Inline,
        padding: Edges {
            top: Dimension::Auto,
            right: Dimension::Auto,
            bottom: Dimension::Auto,
            left: px(6),
        },
        ..BoxStyle::default()
    });
    tree.append_child(block, span).unwrap();
    let inner = tree.insert_text(
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        "x",
    );
    tree.append_child(span, inner).unwrap();
    layout_document(
        &mut tree,
        root,
        LayoutConstraints {
            width: Px::from_px(36),
            height: None,
        },
        &ctx,
    )
    .unwrap();
    let y_word = tree.get(word).unwrap().fragments[0].y;
    let y_inner = tree.get(inner).unwrap().fragments[0].y;
    assert_eq!(y_inner.to_f32() - y_word.to_f32(), 16.0);
}
