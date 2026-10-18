//! A long invoice paginated: the header row repeats at the top of each page after the first.

use tpt_layout::{templates, NodeId, PageConstraints, Px};

#[test]
fn invoice_header_row_repeats_on_each_page() {
    let items: Vec<(String, i32, i32)> = (0..12).map(|i| (format!("Item {i}"), i + 1, 3)).collect();
    let borrowed: Vec<(&str, i32, i32)> =
        items.iter().map(|(d, q, p)| (d.as_str(), *q, *p)).collect();
    let invoice = templates::invoice("Acme Ltd", &borrowed);

    // Find the header row: the only row marked `table_header`.
    let header_row = invoice
        .tree
        .subtree_ids(invoice.root)
        .into_iter()
        .find(|id| invoice.tree.get(*id).is_some_and(|n| n.style.table_header))
        .expect("header row");
    let header_cells: Vec<NodeId> = invoice.tree.children(header_row).to_vec();
    assert_eq!(header_cells.len(), 3);

    // Short pages: a few rows each.
    let page = PageConstraints::new(Px::from_px(400), Px::from_px(96));
    let layout = invoice.paginate(&page);
    assert!(layout.pages.len() >= 3, "pages: {}", layout.pages.len());

    for index in 0..layout.pages.len() {
        for cell in &header_cells {
            let placed = layout
                .fragments
                .iter()
                .any(|f| f.node == *cell && f.page == index);
            assert!(placed, "header cell {cell:?} missing on page {index}");
        }
    }
}
