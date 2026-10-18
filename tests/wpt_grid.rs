//! Adapted from the official CSS Grid tests in web-platform-tests (`css/css-grid`). Each test names
//! its source file. The official tests are pixel reftests; these check the geometry each test's
//! pass condition describes. Tests that need `max-content` sizing keywords, scrolling or fonts are
//! not included.

mod common;

use common::*;
use tpt_layout::{BoxStyle, BoxTree, Dimension, Display, GridTemplate, TrackSize, Tracks};

fn grid_style(width: i32) -> BoxStyle {
    BoxStyle {
        display: Display::Grid,
        width: Dimension::px(width),
        ..BoxStyle::default()
    }
}

#[test]
fn fractional_fr_factors_do_not_lose_space() {
    // grid-flexible-track-fractional-flex-factor.html: 1.01fr 0.99fr 1fr in 600 px.
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        GridTemplate {
            columns: Tracks::new(&[TrackSize::Fr(101), TrackSize::Fr(99), TrackSize::Fr(100)]),
            rows: Tracks::new(&[TrackSize::Fixed(Dimension::px(100))]),
            ..GridTemplate::default()
        },
        grid_style(600),
    );
    let a = child(&mut tree, root, height_only(100));
    let b = child(&mut tree, root, height_only(100));
    let c = child(&mut tree, root, height_only(100));
    layout(&mut tree, root, 600);
    assert_eq!(rect(&tree, a).2, 202.0);
    assert_eq!(rect(&tree, b), (202.0, 0.0, 198.0, 100.0));
    assert_eq!(rect(&tree, c), (400.0, 0.0, 200.0, 100.0));
}

#[test]
fn free_space_is_shared_evenly_among_fr_tracks() {
    // grid-flexible-track-free-space-distribution.html, with 16 tracks (the supported maximum)
    // instead of 100: 100 px divides into 6.25 px per track.
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        GridTemplate {
            columns: Tracks::new(&[TrackSize::Fr(100); 16]),
            rows: Tracks::new(&[TrackSize::Fixed(Dimension::px(100))]),
            ..GridTemplate::default()
        },
        grid_style(100),
    );
    let items: Vec<_> = (0..16)
        .map(|_| child(&mut tree, root, height_only(100)))
        .collect();
    layout(&mut tree, root, 100);
    assert_eq!(rect(&tree, items[15]), (93.75, 0.0, 6.25, 100.0));
    let total: f32 = items.iter().map(|id| rect(&tree, *id).2).sum();
    assert_eq!(total, 100.0);
}

#[test]
fn an_item_wider_than_its_zero_width_grid_sizes_the_auto_track() {
    // grid-overflowing-item-right-of-container-is-painted.html: an `auto` track grows to the
    // item even when the grid itself is 0 px wide, and the item overflows to the right.
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        GridTemplate {
            columns: Tracks::new(&[TrackSize::Auto]),
            rows: Tracks::new(&[TrackSize::Auto]),
            ..GridTemplate::default()
        },
        grid_style(0),
    );
    let item = child(&mut tree, root, boxed(100, 100));
    layout(&mut tree, root, 800);
    assert_eq!(rect(&tree, item), (0.0, 0.0, 100.0, 100.0));
}

#[test]
fn flexible_length_fills_the_remaining_track_space() {
    // grid-definition/fr-unit.html: 100px and 1fr columns, 30px and 1fr rows, in 400 x 100.
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        GridTemplate {
            columns: Tracks::new(&[TrackSize::Fixed(Dimension::px(100)), TrackSize::Fr(100)]),
            rows: Tracks::new(&[TrackSize::Fixed(Dimension::px(30)), TrackSize::Fr(100)]),
            ..GridTemplate::default()
        },
        BoxStyle {
            width: Dimension::px(400),
            height: Dimension::px(100),
            ..BoxStyle::default()
        },
    );
    let _cell1 = child(&mut tree, root, height_only(30));
    let cell2 = child(&mut tree, root, height_only(30));
    let _cell3 = child(&mut tree, root, height_only(30));
    // cell4 has an auto height, so it stretches to fill its 1fr row.
    let cell4 = child(&mut tree, root, BoxStyle::default());
    layout(&mut tree, root, 400);
    assert_eq!(rect(&tree, cell2), (100.0, 0.0, 300.0, 30.0));
    assert_eq!(rect(&tree, cell4), (100.0, 30.0, 300.0, 70.0));
}

#[test]
fn fr_factors_summing_to_less_than_one_take_only_their_fraction() {
    // grid-definition/flex-factor-sum-less-than-1-001.html (plain fr; minmax(0, fr) is not
    // supported): 0.1fr 0.2fr 0.3fr in 100 px take 10, 20 and 30 px. The rest stays empty.
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        GridTemplate {
            columns: Tracks::new(&[TrackSize::Fr(10), TrackSize::Fr(20), TrackSize::Fr(30)]),
            rows: Tracks::new(&[TrackSize::Fixed(Dimension::px(50))]),
            ..GridTemplate::default()
        },
        BoxStyle {
            width: Dimension::px(100),
            ..BoxStyle::default()
        },
    );
    let a = child(&mut tree, root, height_only(50));
    let b = child(&mut tree, root, height_only(50));
    let c = child(&mut tree, root, height_only(50));
    layout(&mut tree, root, 100);
    assert_eq!(rect(&tree, a), (0.0, 0.0, 10.0, 50.0));
    assert_eq!(rect(&tree, b), (10.0, 0.0, 20.0, 50.0));
    assert_eq!(rect(&tree, c), (30.0, 0.0, 30.0, 50.0));
}
