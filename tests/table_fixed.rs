//! `table-layout: fixed`: the first row's declared cell widths set the columns.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Px, TableLayout,
};

/// Lays out a 200 px fixed-layout table. Each entry of `rows` lists the declared width of each
/// cell in that row (`None` for an auto width). Returns the tree and the cells, row by row.
fn layout(rows: &[&[Option<i32>]]) -> (BoxTree, Vec<Vec<NodeId>>) {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let table = tree.insert(BoxStyle {
        display: Display::Table,
        table_layout: TableLayout::Fixed,
        ..BoxStyle::default()
    });
    tree.append_child(root, table).unwrap();
    let mut cells = Vec::new();
    for row_widths in rows {
        let row = tree.insert(BoxStyle {
            display: Display::TableRow,
            ..BoxStyle::default()
        });
        tree.append_child(table, row).unwrap();
        let mut row_cells = Vec::new();
        for width in row_widths.iter() {
            let cell = tree.insert(BoxStyle {
                display: Display::TableCell,
                width: width.map_or(Dimension::Auto, |w| Dimension::Length(Px::from_px(w))),
                height: Dimension::Length(Px::from_px(10)),
                ..BoxStyle::default()
            });
            tree.append_child(row, cell).unwrap();
            row_cells.push(cell);
        }
        cells.push(row_cells);
    }
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
    (tree, cells)
}

#[test]
fn declared_width_sets_its_column_and_the_rest_takes_the_remainder() {
    let (tree, cells) = layout(&[&[Some(50), None], &[None, None]]);
    assert_eq!(tree.get(cells[0][0]).unwrap().layout.width, 50.0);
    assert_eq!(tree.get(cells[0][1]).unwrap().layout.width, 150.0);
    assert_eq!(tree.get(cells[0][1]).unwrap().layout.x, 50.0);
}

#[test]
fn undeclared_columns_share_the_remainder_equally() {
    let (tree, cells) = layout(&[&[None, None, Some(40)]]);
    assert_eq!(tree.get(cells[0][0]).unwrap().layout.width, 80.0);
    assert_eq!(tree.get(cells[0][1]).unwrap().layout.width, 80.0);
    assert_eq!(tree.get(cells[0][2]).unwrap().layout.width, 40.0);
}

#[test]
fn declared_widths_are_kept_even_when_they_overflow() {
    let (tree, cells) = layout(&[&[Some(150), Some(100)]]);
    assert_eq!(tree.get(cells[0][0]).unwrap().layout.width, 150.0);
    assert_eq!(tree.get(cells[0][1]).unwrap().layout.x, 150.0);
    assert_eq!(tree.get(cells[0][1]).unwrap().layout.width, 100.0);
}
