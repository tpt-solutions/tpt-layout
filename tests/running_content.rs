//! Running headers and footers: the same nodes are placed at the top and bottom of every page.

use tpt_layout::{
    layout_document, paginate, BoxStyle, BoxTree, Dimension, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, PageConstraints, Px,
};

#[test]
fn header_and_footer_repeat_on_every_page_in_the_reserved_space() {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();

    // Body: three 150 px boxes. A page's content area is 200 - 20 - 20 = 160 px, so each box
    // fills a page of its own.
    let body = tree.insert(BoxStyle::default());
    for _ in 0..3 {
        let block = tree.insert(BoxStyle {
            height: Dimension::Length(Px::from_px(150)),
            ..BoxStyle::default()
        });
        tree.append_child(body, block).unwrap();
    }
    let header = tree.insert(BoxStyle {
        height: Dimension::Length(Px::from_px(20)),
        ..BoxStyle::default()
    });
    let footer = tree.insert(BoxStyle {
        height: Dimension::Length(Px::from_px(20)),
        ..BoxStyle::default()
    });
    let constraints = LayoutConstraints {
        width: Px::from_px(200),
        height: None,
    };
    layout_document(&mut tree, body, constraints, &ctx).unwrap();
    layout_document(&mut tree, header, constraints, &ctx).unwrap();
    layout_document(&mut tree, footer, constraints, &ctx).unwrap();

    let page = PageConstraints {
        header_height: Px::from_px(20),
        footer_height: Px::from_px(20),
        ..PageConstraints::new(Px::from_px(200), Px::from_px(200))
    };
    let mut layout = paginate(&tree, body, &page);
    assert_eq!(layout.pages.len(), 3);

    layout.place_running(&page, Some(header), Some(footer));

    let headers: Vec<_> = layout
        .fragments
        .iter()
        .filter(|f| f.node == header)
        .collect();
    let footers: Vec<_> = layout
        .fragments
        .iter()
        .filter(|f| f.node == footer)
        .collect();
    assert_eq!(headers.len(), 3);
    assert_eq!(footers.len(), 3);
    for (index, (h, f)) in headers.iter().zip(&footers).enumerate() {
        assert_eq!(h.page, index);
        assert_eq!(h.y, 0.0);
        assert_eq!(h.height, 20.0);
        // The footer sits below the header and the 160 px of body content.
        assert_eq!(f.y, 180.0);
        assert_eq!(f.height, 20.0);
    }
    for page in &layout.pages {
        assert!(page.boxes.contains(&header));
        assert!(page.boxes.contains(&footer));
    }
}

#[test]
fn no_header_or_footer_adds_nothing() {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let body = tree.insert(BoxStyle::default());
    let page = PageConstraints::new(Px::from_px(200), Px::from_px(200));
    layout_document(
        &mut tree,
        body,
        LayoutConstraints {
            width: Px::from_px(200),
            height: None,
        },
        &ctx,
    )
    .unwrap();
    let mut layout = paginate(&tree, body, &page);
    let before = layout.fragments.len();
    layout.place_running(&page, None, None);
    assert_eq!(layout.fragments.len(), before);
}
