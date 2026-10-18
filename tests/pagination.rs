//! Pagination: page breaks, forced breaks, orphans and widows, oversized boxes, table headers.

mod common;

use common::*;
use tpt_layout::{paginate, BoxStyle, BoxTree, Break, Dimension, Display, PageConstraints};

/// A 100 px wide root, paginated into 100 px tall pages.
fn page_100() -> PageConstraints {
    PageConstraints::new(px(100), px(100))
}

fn doc(tree: &mut BoxTree) -> tpt_layout::NodeId {
    tree.insert(BoxStyle {
        width: Dimension::px(100),
        ..BoxStyle::default()
    })
}

#[test]
fn blocks_fill_pages_and_overflow_to_the_next() {
    let mut tree = BoxTree::new();
    let root = doc(&mut tree);
    let ids: Vec<_> = (0..4)
        .map(|_| child(&mut tree, root, boxed(0, 30)))
        .collect();
    layout(&mut tree, root, 100);
    let layout = paginate(&tree, root, &page_100());
    assert_eq!(layout.pages.len(), 2);
    assert_eq!(layout.pages[1].content_offset_y, 90.0);
    let on_first = layout.fragments.iter().filter(|f| f.page == 0).count();
    assert_eq!(on_first, 3);
    let last = layout.fragments.iter().find(|f| f.node == ids[3]).unwrap();
    assert_eq!((last.page, last.y), (1, 0.0));
    assert_eq!(layout.total_height, 120.0);
}

#[test]
fn break_before_always_starts_a_new_page() {
    let mut tree = BoxTree::new();
    let root = doc(&mut tree);
    let _first = child(&mut tree, root, boxed(0, 30));
    let forced = child(
        &mut tree,
        root,
        BoxStyle {
            break_before: Break::Always,
            ..boxed(0, 30)
        },
    );
    layout(&mut tree, root, 100);
    let layout = paginate(&tree, root, &page_100());
    assert_eq!(layout.pages.len(), 2);
    assert_eq!(
        layout
            .fragments
            .iter()
            .find(|f| f.node == forced)
            .unwrap()
            .page,
        1
    );
}

#[test]
fn break_inside_avoid_keeps_a_box_whole() {
    let mut tree = BoxTree::new();
    let root = doc(&mut tree);
    let _filler = child(&mut tree, root, boxed(0, 60));
    let keep = child(
        &mut tree,
        root,
        BoxStyle {
            break_inside: Break::Avoid,
            ..BoxStyle::default()
        },
    );
    child(&mut tree, keep, boxed(0, 30));
    child(&mut tree, keep, boxed(0, 30));
    layout(&mut tree, root, 100);
    let layout = paginate(&tree, root, &page_100());
    // 60 + 60 > 100, and the box may not split, so it moves to page 2 as a whole.
    let fragment = layout.fragments.iter().find(|f| f.node == keep).unwrap();
    assert_eq!((fragment.page, fragment.y), (1, 0.0));
}

#[test]
fn text_splits_between_lines_when_orphans_and_widows_allow() {
    // Five 20 px lines (one 10-char word each). 20 px used, so four lines fit on the page.
    // That leaves one line on the next page, below the widow minimum of 2, so the split moves
    // to three lines here and two there.
    let mut tree = BoxTree::new();
    let root = doc(&mut tree);
    let _before = child(&mut tree, root, boxed(0, 20));
    let para = text_child(
        &mut tree,
        root,
        BoxStyle::default(),
        "aaaaaaaaaa bbbbbbbbbb cccccccccc dddddddddd eeeeeeeeee",
    );
    layout(&mut tree, root, 100);
    let layout = paginate(&tree, root, &page_100());
    let parts: Vec<_> = layout.fragments.iter().filter(|f| f.node == para).collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].lines, Some(0..3));
    assert_eq!(parts[1].lines, Some(3..5));
    assert_eq!(parts[1].page, 1);
}

#[test]
fn orphan_rule_moves_a_paragraph_with_one_line_on_the_page() {
    let mut tree = BoxTree::new();
    let root = doc(&mut tree);
    let _before = child(&mut tree, root, boxed(0, 80));
    let para = text_child(
        &mut tree,
        root,
        BoxStyle::default(),
        "aaaaaaaaaa bbbbbbbbbb cccccccccc",
    );
    layout(&mut tree, root, 100);
    let layout = paginate(&tree, root, &page_100());
    // Only one line would fit, which is below the orphan minimum of 2, so the paragraph moves.
    let fragment = layout.fragments.iter().find(|f| f.node == para).unwrap();
    // The whole paragraph (all three lines) lands on page 2.
    assert_eq!((fragment.page, fragment.lines.clone()), (1, Some(0..3)));
}

#[test]
fn oversized_box_gets_its_own_page() {
    let mut tree = BoxTree::new();
    let root = doc(&mut tree);
    let big = child(&mut tree, root, boxed(0, 300));
    layout(&mut tree, root, 100);
    let layout = paginate(&tree, root, &page_100());
    assert_eq!(
        layout
            .fragments
            .iter()
            .find(|f| f.node == big)
            .unwrap()
            .page,
        0
    );
    assert_eq!(layout.pages.len(), 1);
}

#[test]
fn left_and_right_breaks_respect_page_parity() {
    // Page 1 is a right page, so `left` lands on page 2 and `right` needs a blank page first.
    for (kind, expected_page) in [(Break::Left, 1), (Break::Right, 2)] {
        let mut tree = BoxTree::new();
        let root = doc(&mut tree);
        let _first = child(&mut tree, root, boxed(0, 30));
        let target = child(
            &mut tree,
            root,
            BoxStyle {
                break_before: kind,
                ..boxed(0, 30)
            },
        );
        layout(&mut tree, root, 100);
        let layout = paginate(&tree, root, &page_100());
        assert_eq!(
            layout
                .fragments
                .iter()
                .find(|f| f.node == target)
                .unwrap()
                .page,
            expected_page,
            "{kind:?}"
        );
    }
}

#[test]
fn table_header_repeats_on_continuation_pages() {
    let mut tree = BoxTree::new();
    let root = doc(&mut tree);
    let table = child(
        &mut tree,
        root,
        BoxStyle {
            display: Display::Table,
            ..BoxStyle::default()
        },
    );
    let mut header_cell = None;
    for index in 0..4 {
        let row = child(
            &mut tree,
            table,
            BoxStyle {
                display: Display::TableRow,
                ..BoxStyle::default()
            },
        );
        let style = BoxStyle {
            display: Display::TableCell,
            table_header: index == 0,
            ..boxed(0, 30)
        };
        let cell = child(&mut tree, row, style);
        if index == 0 {
            header_cell = Some(cell);
        }
    }
    layout(&mut tree, root, 100);
    let page_70 = PageConstraints::new(px(100), px(70));
    let layout = paginate(&tree, root, &page_70);
    // Page 1: header + row 1. Page 2: header + row 2. Page 3: header + row 3.
    assert_eq!(layout.pages.len(), 3);
    let header = header_cell.unwrap();
    let repeated_on_page_2 = layout
        .fragments
        .iter()
        .any(|f| f.node == header && f.page == 1);
    assert!(
        repeated_on_page_2,
        "header row should repeat on the second page"
    );
}
