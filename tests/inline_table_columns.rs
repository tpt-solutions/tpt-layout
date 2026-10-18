//! Inline formatting, floats, tables, and multi-column layout.

mod common;

use common::*;
use tpt_layout::{BoxStyle, BoxTree, Clear, Dimension, Display, Float};

#[test]
fn text_wraps_at_word_boundaries() {
    // Block of 100 px. "aaaa bbbb cccc" at 10 px/char: "aaaa bbbb" is 90 px and fits, "cccc"
    // wraps onto a second 20 px line.
    let mut tree = BoxTree::new();
    let root = tree.insert(boxed(100, 0));
    let text = text_child(
        &mut tree,
        root,
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        "aaaa bbbb cccc",
    );
    layout(&mut tree, root, 100);
    let fragments = &tree.get(text).unwrap().fragments;
    let positions: Vec<(f32, f32)> = fragments
        .iter()
        .map(|f| (f.x.to_f32(), f.y.to_f32()))
        .collect();
    assert_eq!(positions, [(0.0, 0.0), (50.0, 0.0), (0.0, 20.0)]);
    assert_eq!(rect(&tree, text).3, 40.0);
}

#[test]
fn inline_block_sits_on_the_text_baseline() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let text = text_child(
        &mut tree,
        root,
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        "aa",
    );
    let block = child(
        &mut tree,
        root,
        BoxStyle {
            display: Display::InlineBlock,
            ..boxed(30, 20)
        },
    );
    // The text is measured inside the same run, so the inline-block follows it.
    let _ = text;
    layout(&mut tree, root, 200);
    let (x, y, w, h) = rect(&tree, block);
    assert_eq!((x, y, w, h), (20.0, 0.0, 30.0, 20.0));
}

#[test]
fn left_float_shortens_lines_beside_it() {
    let mut tree = BoxTree::new();
    let root = tree.insert(boxed(100, 0));
    let float = child(
        &mut tree,
        root,
        BoxStyle {
            float: Float::Left,
            ..boxed(50, 40)
        },
    );
    let text = text_child(
        &mut tree,
        root,
        BoxStyle {
            display: Display::Inline,
            ..BoxStyle::default()
        },
        "aaaa bbbb",
    );
    layout(&mut tree, root, 100);
    assert_eq!(rect(&tree, float).0, 0.0);
    let fragments = &tree.get(text).unwrap().fragments;
    let positions: Vec<(f32, f32)> = fragments
        .iter()
        .map(|f| (f.x.to_f32(), f.y.to_f32()))
        .collect();
    // Both lines start at the float's right edge.
    assert_eq!(positions, [(50.0, 0.0), (50.0, 20.0)]);
}

#[test]
fn clear_both_moves_block_below_float() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let _float = child(
        &mut tree,
        root,
        BoxStyle {
            float: Float::Left,
            ..boxed(50, 40)
        },
    );
    let after = child(
        &mut tree,
        root,
        BoxStyle {
            clear: Clear::Both,
            ..boxed(10, 10)
        },
    );
    layout(&mut tree, root, 100);
    assert_eq!(rect(&tree, after).1, 40.0);
}

#[test]
fn table_cells_share_width_and_rows_stack() {
    let mut tree = BoxTree::new();
    let table = tree.insert(BoxStyle {
        display: Display::Table,
        ..BoxStyle::default()
    });
    for _ in 0..2 {
        let row = child(
            &mut tree,
            table,
            BoxStyle {
                display: Display::TableRow,
                ..BoxStyle::default()
            },
        );
        for _ in 0..2 {
            child(
                &mut tree,
                row,
                BoxStyle {
                    display: Display::TableCell,
                    ..boxed(0, 20)
                },
            );
        }
    }
    layout(&mut tree, table, 300);
    let cells: Vec<(f32, f32, f32, f32)> = (0..4)
        .map(|i| {
            let row = tree.children(table)[i / 2];
            rect(&tree, tree.children(row)[i % 2])
        })
        .collect();
    assert_eq!(cells[0], (0.0, 0.0, 150.0, 20.0));
    assert_eq!(cells[1], (150.0, 0.0, 150.0, 20.0));
    assert_eq!(cells[2], (0.0, 20.0, 150.0, 20.0));
    assert_eq!(tree.get(table).unwrap().layout.height, 40.0);
}

#[test]
fn multi_column_balances_children() {
    // Four 20 px blocks in two 100 px columns: target 40 px, so two per column.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        column_count: 2,
        column_gap: Dimension::ZERO,
        ..BoxStyle::default()
    });
    let ids: Vec<_> = (0..4)
        .map(|_| child(&mut tree, root, boxed(0, 20)))
        .collect();
    layout(&mut tree, root, 200);
    let placed: Vec<(f32, f32)> = ids
        .iter()
        .map(|id| (rect(&tree, *id).0, rect(&tree, *id).1))
        .collect();
    assert_eq!(
        placed,
        [(0.0, 0.0), (0.0, 20.0), (100.0, 0.0), (100.0, 20.0)]
    );
    assert_eq!(tree.get(root).unwrap().layout.height, 40.0);
}
