//! SVG debug renderer: outlines for boxes and lines, inline text drawn and escaped.

use tpt_layout::{
    layout_document, svg::render_svg, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, Px,
};

/// Renders a 200 px root holding one paragraph with `text`. Inline text sits in an inline run
/// inside the paragraph, so it has fragments and is drawn with its characters.
fn render(text: &str, inline: bool) -> String {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: Dimension::Length(Px::from_px(200)),
        ..BoxStyle::default()
    });
    let para = tree.insert(BoxStyle::default());
    tree.append_child(root, para).unwrap();
    let leaf_display = if inline {
        Display::Inline
    } else {
        Display::Block
    };
    let leaf = tree.insert_text(
        BoxStyle {
            display: leaf_display,
            ..BoxStyle::default()
        },
        text,
    );
    tree.append_child(para, leaf).unwrap();
    layout_document(
        &mut tree,
        root,
        LayoutConstraints {
            width: Px::from_px(200),
            height: None,
        },
        &ctx,
    )
    .unwrap();
    render_svg(&tree, root, 200.0, 100.0)
}

#[test]
fn draws_an_outline_for_every_box_and_line() {
    let svg = render("hello", false);
    // Root box, paragraph, text box, and one line of "hello".
    assert_eq!(svg.matches("<rect data-node=").count(), 4, "{svg}");
    assert_eq!(svg.matches("data-line=").count(), 1, "{svg}");
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.trim_end().ends_with("</svg>"));
}

#[test]
fn inline_text_is_drawn_with_its_characters() {
    let svg = render("hello", true);
    assert!(svg.contains(">hello</text>"), "{svg}");
}

#[test]
fn block_text_is_drawn_with_its_characters() {
    let svg = render("hello", false);
    assert!(svg.contains(">hello</text>"), "{svg}");
}

#[test]
fn escapes_markup_in_inline_text() {
    let svg = render("<script>&\"'", true);
    assert!(!svg.contains("<script>"), "raw markup leaked: {svg}");
    assert!(svg.contains("&lt;script&gt;&amp;&quot;&apos;"), "{svg}");
}
