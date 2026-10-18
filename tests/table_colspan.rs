//! Table `column_span`: a spanning cell covers several columns and widens them when needed.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Px,
};

/// A cell whose max-content width is `width`, holding a fixed-width box, with `span` columns.
fn cell(tree: &mut BoxTree, width: i32, span: u16) -> NodeId {
    let cell = tree.insert(BoxStyle {
        display: Display::TableCell,
        column_span: span,
        ..BoxStyle::default()
    });
    let inner = tree.insert(BoxStyle {
        width: Dimension::Length(Px::from_px(width)),
        height: Dimension::Length(Px::from_px(10)),
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

/// Lays out a 200 px table and returns its tree plus the table node.
fn layout_table(build: impl FnOnce(&mut BoxTree, NodeId)) -> (BoxTree, NodeId) {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let table = tree.insert(BoxStyle {
        display: Display::Table,
        ..BoxStyle::default()
    });
    tree.append_child(root, table).unwrap();
    build(&mut tree, table);
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
    (tree, table)
}

#[test]
fn spanning_cell_widens_the_columns_it_covers() {
    // Row one: one cell spanning two columns, needing 100 px. Row two: two 40 px cells. The
    // columns start at 40 px each, so the spanning cell adds 10 px to each: 50 + 50 = 100. The
    // 200 px table then doubles that, so each column is 100 px.
    let mut spanner = None;
    let mut left = None;
    let mut right = None;
    let (tree, _) = layout_table(|tree, table| {
        spanner = Some(cell(tree, 100, 2));
        row(tree, table, &[spanner.unwrap()]);
        left = Some(cell(tree, 40, 1));
        right = Some(cell(tree, 40, 1));
        row(tree, table, &[left.unwrap(), right.unwrap()]);
    });
    let s = tree.get(spanner.unwrap()).unwrap().layout;
    assert_eq!((s.x, s.width), (0.0, 200.0));
    let l = tree.get(left.unwrap()).unwrap().layout;
    let r = tree.get(right.unwrap()).unwrap().layout;
    assert_eq!((l.x, l.width), (0.0, 100.0));
    assert_eq!((r.x, r.width), (100.0, 100.0));
}

#[test]
fn single_column_cells_share_the_spare_width_in_proportion() {
    let mut a = None;
    let mut b = None;
    let (tree, _) = layout_table(|tree, table| {
        a = Some(cell(tree, 40, 1));
        b = Some(cell(tree, 60, 1));
        row(tree, table, &[a.unwrap(), b.unwrap()]);
    });
    // 40 + 60 = 100 spreads over 200 px in a 2:3 ratio: 80 and 120.
    assert_eq!(tree.get(a.unwrap()).unwrap().layout.width, 80.0);
    assert_eq!(tree.get(b.unwrap()).unwrap().layout.width, 120.0);
}

#[test]
fn column_span_zero_counts_as_one() {
    let mut a = None;
    let mut b = None;
    let (tree, _) = layout_table(|tree, table| {
        a = Some(cell(tree, 40, 0));
        b = Some(cell(tree, 60, 1));
        row(tree, table, &[a.unwrap(), b.unwrap()]);
    });
    assert_eq!(tree.get(a.unwrap()).unwrap().layout.width, 80.0);
}
