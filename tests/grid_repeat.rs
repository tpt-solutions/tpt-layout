//! Grid `repeat()`: a fixed repeat written out with `Tracks::repeat`, and `auto-fill` columns.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer, GridTemplate,
    LayoutConstraints, LayoutContext, NodeId, Px, TrackSize, Tracks,
};

fn fixed(px: i32) -> TrackSize {
    TrackSize::Fixed(Dimension::Length(Px::from_px(px)))
}

#[test]
fn repeat_writes_the_pattern_out() {
    let tracks = Tracks::repeat(2, &[fixed(10), fixed(20)]).unwrap();
    assert_eq!(
        tracks.as_slice(),
        &[fixed(10), fixed(20), fixed(10), fixed(20)]
    );
}

#[test]
fn repeat_beyond_the_track_limit_is_none() {
    // 9 repeats of 2 tracks is 18, over the 16-track limit.
    assert!(Tracks::repeat(9, &[fixed(10), fixed(20)]).is_none());
}

/// Lays out a 200 px grid of six 10 px items in the given columns, and returns the x of each.
fn item_xs(template: GridTemplate, column_gap: i32) -> Vec<f32> {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let grid = tree.insert(BoxStyle {
        display: Display::Grid,
        width: Dimension::Length(Px::from_px(200)),
        column_gap: Dimension::Length(Px::from_px(column_gap)),
        ..BoxStyle::default()
    });
    tree.set_grid_template(grid, template);
    tree.append_child(root, grid).unwrap();
    let mut items: Vec<NodeId> = Vec::new();
    for _ in 0..6 {
        let item = tree.insert(BoxStyle {
            height: Dimension::Length(Px::from_px(10)),
            ..BoxStyle::default()
        });
        tree.append_child(grid, item).unwrap();
        items.push(item);
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
    items
        .iter()
        .map(|id| tree.get(*id).unwrap().layout.x)
        .collect()
}

#[test]
fn auto_fill_makes_as_many_columns_as_fit() {
    // 200 px with 40 px columns and no gap: five columns, so the sixth item wraps to the start.
    let template = GridTemplate {
        auto_fill_columns: Some(Px::from_px(40)),
        ..GridTemplate::default()
    };
    assert_eq!(
        item_xs(template, 0),
        vec![0.0, 40.0, 80.0, 120.0, 160.0, 0.0]
    );
}

#[test]
fn auto_fill_counts_the_gap_between_columns() {
    // 40 px columns with 10 px gaps: (200 + 10) / (40 + 10) is four columns.
    let template = GridTemplate {
        auto_fill_columns: Some(Px::from_px(40)),
        ..GridTemplate::default()
    };
    assert_eq!(
        item_xs(template, 10),
        vec![0.0, 50.0, 100.0, 150.0, 0.0, 50.0]
    );
}

#[test]
fn auto_fill_keeps_at_least_one_column() {
    // A column wider than the container still gets one column.
    let template = GridTemplate {
        auto_fill_columns: Some(Px::from_px(300)),
        ..GridTemplate::default()
    };
    assert_eq!(item_xs(template, 0), vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
}

/// Three items in a 200 px grid with `auto-fit` or `auto-fill` 40 px columns, centred. Returns
/// the x of the first item.
fn centred_first_x(auto_fit: bool) -> f32 {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let grid = tree.insert(BoxStyle {
        display: Display::Grid,
        width: Dimension::Length(Px::from_px(200)),
        justify_content: tpt_layout::JustifyContent::Center,
        ..BoxStyle::default()
    });
    tree.set_grid_template(
        grid,
        GridTemplate {
            auto_fill_columns: Some(Px::from_px(40)),
            auto_fit,
            ..GridTemplate::default()
        },
    );
    tree.append_child(root, grid).unwrap();
    let mut first = None;
    for _ in 0..3 {
        let item = tree.insert(BoxStyle {
            height: Dimension::Length(Px::from_px(10)),
            ..BoxStyle::default()
        });
        tree.append_child(grid, item).unwrap();
        first.get_or_insert(item);
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
    tree.get(first.unwrap()).unwrap().layout.x
}

#[test]
fn auto_fill_keeps_empty_columns_so_centring_has_no_room() {
    // Five 40 px columns fill the 200 px grid, so the three items are already at the start.
    assert_eq!(centred_first_x(false), 0.0);
}

#[test]
fn auto_fit_collapses_empty_columns_so_the_items_centre() {
    // Only three columns are used; the two empty ones collapse. The 120 px of items centre at
    // (200 - 120) / 2 = 40.
    assert_eq!(centred_first_x(true), 40.0);
}
