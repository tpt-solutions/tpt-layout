//! Table `row_span`: a cell covers several rows, and later rows skip the columns it holds.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Px,
};

/// A cell holding a fixed `width` x `height` box, spanning `row_span` rows.
fn cell(tree: &mut BoxTree, width: i32, height: i32, row_span: u16) -> NodeId {
    let cell = tree.insert(BoxStyle {
        display: Display::TableCell,
        row_span,
        ..BoxStyle::default()
    });
    let inner = tree.insert(BoxStyle {
        width: Dimension::Length(Px::from_px(width)),
        height: Dimension::Length(Px::from_px(height)),
        ..BoxStyle::default()
    });
    tree.append_child(cell, inner).unwrap();
    cell
}

fn row(tree: &mut BoxTree, table: NodeId, cells: &[NodeId]) {
    let row = tree.insert(BoxStyle {
        display: Display::TableRow,
        ..BoxStyle::default()
    });
    tree.append_child(table, row).unwrap();
    for c in cells {
        tree.append_child(row, *c).unwrap();
    }
}

#[test]
fn row_spanning_cell_covers_its_rows_and_later_rows_skip_its_column() {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let table = tree.insert(BoxStyle {
        display: Display::Table,
        ..BoxStyle::default()
    });
    tree.append_child(root, table).unwrap();

    // Row one: A spans two rows (40 x 60), B is 40 x 20.
    let a = cell(&mut tree, 40, 60, 2);
    let b = cell(&mut tree, 40, 20, 1);
    row(&mut tree, table, &[a, b]);
    // Row two: C (40 x 20) skips column one, which A holds, so it lands in column two.
    let c = cell(&mut tree, 40, 20, 1);
    row(&mut tree, table, &[c]);

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

    // Rows: one is 20 tall from B and two is 20 from C. A needs 60, so row two grows to 40.
    let a_layout = tree.get(a).unwrap().layout;
    assert_eq!((a_layout.x, a_layout.y), (0.0, 0.0));
    assert_eq!(a_layout.height, 60.0);

    let b_layout = tree.get(b).unwrap().layout;
    let c_layout = tree.get(c).unwrap().layout;
    assert_eq!(c_layout.x, b_layout.x, "C should sit in B's column");
    assert_eq!(c_layout.y, 20.0);
    assert_eq!(c_layout.height, 40.0, "C is stretched across row two");

    assert_eq!(tree.get(table).unwrap().layout.height, 60.0);
}

#[test]
fn row_span_zero_counts_as_one() {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let table = tree.insert(BoxStyle {
        display: Display::Table,
        ..BoxStyle::default()
    });
    tree.append_child(root, table).unwrap();
    let a = cell(&mut tree, 40, 20, 0);
    let b = cell(&mut tree, 40, 20, 1);
    row(&mut tree, table, &[a, b]);
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
    assert_eq!(tree.get(table).unwrap().layout.height, 20.0);
}
