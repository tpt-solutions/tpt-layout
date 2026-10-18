//! Grid `justify-content` and `align-content` place fixed tracks in the free space.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer, GridTemplate,
    JustifyContent, LayoutConstraints, LayoutContext, Px, TrackSize, Tracks,
};

/// A 200 px grid with two 40 px columns, so 120 px are free. Returns the x of each item.
fn column_xs(justify: JustifyContent) -> Vec<f32> {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let grid = tree.insert(BoxStyle {
        display: Display::Grid,
        width: Dimension::Length(Px::from_px(200)),
        justify_content: justify,
        ..BoxStyle::default()
    });
    tree.set_grid_template(
        grid,
        GridTemplate {
            columns: Tracks::new(&[
                TrackSize::Fixed(Dimension::Length(Px::from_px(40))),
                TrackSize::Fixed(Dimension::Length(Px::from_px(40))),
            ]),
            ..Default::default()
        },
    );
    tree.append_child(root, grid).unwrap();
    let mut items = Vec::new();
    for _ in 0..2 {
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
fn flex_start_packs_tracks_at_the_start() {
    assert_eq!(column_xs(JustifyContent::FlexStart), vec![0.0, 40.0]);
}

#[test]
fn flex_end_packs_tracks_at_the_end() {
    assert_eq!(column_xs(JustifyContent::FlexEnd), vec![120.0, 160.0]);
}

#[test]
fn center_splits_the_free_space_evenly_at_each_end() {
    assert_eq!(column_xs(JustifyContent::Center), vec![60.0, 100.0]);
}

#[test]
fn space_between_puts_the_free_space_between_tracks() {
    assert_eq!(column_xs(JustifyContent::SpaceBetween), vec![0.0, 160.0]);
}

#[test]
fn space_evenly_gives_equal_space_at_both_ends_and_between() {
    // 120 px over three gaps (start, between, end) is 40 px each: the first track starts after
    // one gap, and the second after the first track plus the middle gap.
    assert_eq!(column_xs(JustifyContent::SpaceEvenly), vec![40.0, 120.0]);
}
