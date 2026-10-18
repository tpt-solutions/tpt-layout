//! CSS Grid. Expected values follow the spec's examples for track sizing and placement.

mod common;

use common::*;
use tpt_layout::{
    AlignItems, BoxStyle, BoxTree, Dimension, Direction, GridAutoFlow, GridLine, GridPlacement,
    GridTemplate, TrackSize, Tracks,
};

fn cols(sizes: &[TrackSize]) -> Tracks {
    Tracks::new(sizes)
}

fn columns(sizes: &[TrackSize]) -> GridTemplate {
    GridTemplate {
        columns: cols(sizes),
        ..GridTemplate::default()
    }
}

#[test]
fn fixed_tracks_and_auto_placement_fill_rows() {
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        columns(&[TrackSize::Fixed(Dimension::px(100)); 2]),
        BoxStyle::default(),
    );
    let ids: Vec<_> = (0..4)
        .map(|_| child(&mut tree, root, height_only(20)))
        .collect();
    layout(&mut tree, root, 300);
    let positions: Vec<(f32, f32)> = ids
        .iter()
        .map(|id| (rect(&tree, *id).0, rect(&tree, *id).1))
        .collect();
    assert_eq!(
        positions,
        [(0.0, 0.0), (100.0, 0.0), (0.0, 20.0), (100.0, 20.0)]
    );
}

#[test]
fn fr_tracks_share_leftover_space() {
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        columns(&[TrackSize::Fr(100), TrackSize::Fr(200)]),
        BoxStyle::default(),
    );
    let a = child(&mut tree, root, height_only(20));
    let b = child(&mut tree, root, height_only(20));
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, a).2, 100.0);
    assert_eq!(rect(&tree, b).0, 100.0);
    assert_eq!(rect(&tree, b).2, 200.0);
}

#[test]
fn explicit_span_covers_two_tracks() {
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        columns(&[TrackSize::Fixed(Dimension::px(100)); 3]),
        BoxStyle::default(),
    );
    let style = BoxStyle {
        grid_column: GridPlacement {
            start: GridLine::Index(1),
            end: GridLine::Span(2),
        },
        ..height_only(20)
    };
    let spanning = child(&mut tree, root, style);
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, spanning).2, 200.0);
}

#[test]
fn named_areas_place_items_by_name() {
    let mut areas = [[0u8; 12]; 12];
    areas[0][0] = 1;
    areas[0][1] = 1;
    areas[1][0] = 2;
    areas[1][1] = 3;
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        GridTemplate {
            columns: cols(&[TrackSize::Fr(100), TrackSize::Fr(100)]),
            areas,
            ..GridTemplate::default()
        },
        BoxStyle::default(),
    );
    let header = child(
        &mut tree,
        root,
        BoxStyle {
            grid_area: 1,
            ..height_only(20)
        },
    );
    let left = child(
        &mut tree,
        root,
        BoxStyle {
            grid_area: 2,
            ..height_only(20)
        },
    );
    let right = child(
        &mut tree,
        root,
        BoxStyle {
            grid_area: 3,
            ..height_only(20)
        },
    );
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, header).2, 300.0);
    assert_eq!(rect(&tree, left).1, 20.0);
    assert_eq!(rect(&tree, right).0, 150.0);
}

#[test]
fn column_flow_fills_columns_first() {
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        GridTemplate {
            rows: cols(&[TrackSize::Fixed(Dimension::px(40)); 2]),
            ..GridTemplate::default()
        },
        BoxStyle {
            grid_auto_columns: TrackSize::Fixed(Dimension::px(100)),
            grid_auto_flow: GridAutoFlow::Column,
            ..BoxStyle::default()
        },
    );
    let ids: Vec<_> = (0..4)
        .map(|_| child(&mut tree, root, height_only(20)))
        .collect();
    layout(&mut tree, root, 300);
    assert_eq!((rect(&tree, ids[1]).0, rect(&tree, ids[1]).1), (0.0, 40.0));
    assert_eq!((rect(&tree, ids[2]).0, rect(&tree, ids[2]).1), (100.0, 0.0));
}

#[test]
fn column_gap_offsets_second_track() {
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        columns(&[TrackSize::Fixed(Dimension::px(100)); 2]),
        BoxStyle {
            column_gap: Dimension::px(10),
            ..BoxStyle::default()
        },
    );
    let _first = child(&mut tree, root, height_only(20));
    let second = child(&mut tree, root, height_only(20));
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, second).0, 110.0);
}

#[test]
fn justify_self_centers_within_cell() {
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        columns(&[TrackSize::Fixed(Dimension::px(100))]),
        BoxStyle::default(),
    );
    let centered = child(
        &mut tree,
        root,
        BoxStyle {
            justify_self: Some(AlignItems::Center),
            ..boxed(50, 20)
        },
    );
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, centered).0, 25.0);
}

#[test]
fn rtl_grid_starts_columns_from_the_right() {
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        columns(&[TrackSize::Fixed(Dimension::px(100)); 2]),
        BoxStyle {
            direction: Direction::Rtl,
            ..BoxStyle::default()
        },
    );
    let first = child(&mut tree, root, height_only(20));
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, first).0, 200.0);
}

#[test]
fn minmax_track_keeps_content_size_above_minimum() {
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        columns(&[TrackSize::MinMax {
            min: Dimension::px(50),
            max: Dimension::Auto,
        }]),
        BoxStyle::default(),
    );
    let id = child(&mut tree, root, boxed(80, 20));
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, id).2, 80.0);
}

#[test]
fn row_heights_follow_tallest_item() {
    let mut tree = BoxTree::new();
    let root = grid(
        &mut tree,
        columns(&[TrackSize::Fixed(Dimension::px(100)); 2]),
        BoxStyle::default(),
    );
    // Row 1 holds a 50 px item and a 10 px item; row 2 starts below the tallest of row 1.
    let _tall = child(&mut tree, root, boxed(10, 50));
    let _short = child(&mut tree, root, height_only(10));
    let next_row = child(&mut tree, root, height_only(10));
    layout(&mut tree, root, 300);
    assert_eq!(rect(&tree, next_row).1, 50.0);
}
