//! Block flow, margin collapsing, and sizing.

use tpt_layout::{
    layout_flow, BoxSizing, BoxStyle, BoxTree, Dimension, Edges, FixedAdvanceMeasurer, NodeId, Px,
};

fn measurer() -> FixedAdvanceMeasurer {
    // 8 px per character, 20 px lines, baseline 15 px below the top of each line.
    FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(20), Px::from_px(15))
}

fn px(v: i32) -> Px {
    Px::from_px(v)
}

fn margins(top: i32, bottom: i32) -> BoxStyle {
    BoxStyle {
        margin: Edges {
            top: Dimension::px(top),
            right: Dimension::ZERO,
            bottom: Dimension::px(bottom),
            left: Dimension::ZERO,
        },
        ..BoxStyle::default()
    }
}

/// The root establishes a block formatting context, so its children's margins never escape it.
fn new_root(tree: &mut BoxTree) -> NodeId {
    tree.insert(BoxStyle::default())
}

fn run(tree: &mut BoxTree, root: NodeId, width: i32) {
    layout_flow(tree, root, px(width), &measurer()).expect("layout");
}

fn text(tree: &mut BoxTree, parent: NodeId, style: BoxStyle, s: &str) -> NodeId {
    let id = tree.insert_text(style, s);
    tree.append_child(parent, id).unwrap();
    id
}

fn block(tree: &mut BoxTree, parent: NodeId, style: BoxStyle) -> NodeId {
    let id = tree.insert(style);
    tree.append_child(parent, id).unwrap();
    id
}

#[test]
fn headings_and_paragraphs_lay_out_with_collapsed_margins() {
    let mut tree = BoxTree::new();
    let root = new_root(&mut tree);

    // h1: 21 px margins, one line of text.
    let h1 = text(&mut tree, root, margins(21, 21), "Title");
    // p: 16 px margins, three words that fit on one 400 px line.
    let p = text(&mut tree, root, margins(16, 16), "aaa bbb ccc ddd");
    run(&mut tree, root, 400);

    let h1 = tree.get(h1).unwrap().layout;
    let p = tree.get(p).unwrap().layout;
    let root = tree.get(root).unwrap().layout;

    assert_eq!((h1.x, h1.y, h1.width, h1.height), (0.0, 21.0, 400.0, 20.0));
    // Adjacent margins collapse to max(21, 16) = 21: 21 + 20 + 21 = 62.
    assert_eq!((p.y, p.height), (62.0, 20.0));
    // Root is a block formatting context, so the paragraph's bottom margin stays inside it.
    assert_eq!(root.height, 82.0 + 16.0);
    assert_eq!(h1.baseline, 15.0);
}

#[test]
fn parent_and_first_child_margins_collapse() {
    let mut tree = BoxTree::new();
    let root = new_root(&mut tree);
    let parent = block(&mut tree, root, BoxStyle::default());
    let child_style = margins(30, 0);
    let _child = text(&mut tree, parent, child_style, "x");
    run(&mut tree, root, 400);

    // The child's 30 px top margin escapes the parent: both start at y = 30.
    let parent_layout = tree.get(parent).unwrap().layout;
    assert_eq!(parent_layout.y, 30.0);
    assert_eq!(parent_layout.height, 20.0);
    let child_layout = tree.get(tree.children(parent)[0]).unwrap().layout;
    assert_eq!(child_layout.y, 30.0);
    assert_eq!(tree.get(root).unwrap().layout.height, 50.0);
}

#[test]
fn padding_prevents_parent_child_collapse() {
    let mut tree = BoxTree::new();
    let root = new_root(&mut tree);
    let parent_style = BoxStyle {
        padding: Edges {
            top: Dimension::px(1),
            ..Edges::all(Dimension::ZERO)
        },
        ..BoxStyle::default()
    };
    let parent = block(&mut tree, root, parent_style);
    let _child = text(&mut tree, parent, margins(30, 0), "x");
    run(&mut tree, root, 400);

    assert_eq!(tree.get(parent).unwrap().layout.y, 0.0);
    let child = tree.get(tree.children(parent)[0]).unwrap().layout;
    assert_eq!(child.y, 31.0);
}

#[test]
fn adjacent_margins_with_negative_value_subtract() {
    let mut tree = BoxTree::new();
    let root = new_root(&mut tree);
    let a = text(&mut tree, root, margins(0, 10), "a");
    let b = text(&mut tree, root, margins(-4, 0), "b");
    run(&mut tree, root, 400);

    // Positive 10 and negative -4 combine to 10 + (-4) = 6 px between the boxes.
    assert_eq!(tree.get(a).unwrap().layout.y, 0.0);
    assert_eq!(tree.get(b).unwrap().layout.y, 26.0);
}

#[test]
fn empty_block_collapses_its_margins_through() {
    let mut tree = BoxTree::new();
    let root = new_root(&mut tree);
    let _a = text(&mut tree, root, margins(0, 10), "a");
    let empty = block(&mut tree, root, margins(20, 5));
    let b = text(&mut tree, root, BoxStyle::default(), "b");
    run(&mut tree, root, 400);

    // All of 10, 20 and 5 collapse into one margin: max = 20.
    assert_eq!(tree.get(empty).unwrap().layout.height, 0.0);
    assert_eq!(tree.get(b).unwrap().layout.y, 20.0 + 20.0);
}

#[test]
fn explicit_width_with_auto_margins_centers() {
    let mut tree = BoxTree::new();
    let root = new_root(&mut tree);
    let style = BoxStyle {
        width: Dimension::px(100),
        margin: Edges {
            top: Dimension::ZERO,
            right: Dimension::Auto,
            bottom: Dimension::ZERO,
            left: Dimension::Auto,
        },
        ..BoxStyle::default()
    };
    let child = block(&mut tree, root, style);
    run(&mut tree, root, 400);

    let layout = tree.get(child).unwrap().layout;
    assert_eq!((layout.x, layout.width), (150.0, 100.0));
}

#[test]
fn percentage_width_resolves_against_containing_block() {
    let mut tree = BoxTree::new();
    let root = new_root(&mut tree);
    let style = BoxStyle {
        width: Dimension::percent(50.0),
        ..BoxStyle::default()
    };
    let child = block(&mut tree, root, style);
    run(&mut tree, root, 400);

    assert_eq!(tree.get(child).unwrap().layout.width, 200.0);
}

#[test]
fn max_width_clamps_auto_width() {
    let mut tree = BoxTree::new();
    let root = new_root(&mut tree);
    let style = BoxStyle {
        max_width: Dimension::px(200),
        ..BoxStyle::default()
    };
    let child = block(&mut tree, root, style);
    run(&mut tree, root, 400);

    assert_eq!(tree.get(child).unwrap().layout.width, 200.0);
}

#[test]
fn min_width_wins_over_max_width() {
    let mut tree = BoxTree::new();
    let root = new_root(&mut tree);
    let style = BoxStyle {
        width: Dimension::px(300),
        min_width: Dimension::px(250),
        max_width: Dimension::px(200),
        ..BoxStyle::default()
    };
    let child = block(&mut tree, root, style);
    run(&mut tree, root, 400);

    assert_eq!(tree.get(child).unwrap().layout.width, 250.0);
}

#[test]
fn border_box_sizing_includes_padding_and_border() {
    let mut tree = BoxTree::new();
    let root = new_root(&mut tree);
    let style = BoxStyle {
        width: Dimension::px(100),
        box_sizing: BoxSizing::BorderBox,
        padding: Edges::all(Dimension::px(10)),
        ..BoxStyle::default()
    };
    let child = block(&mut tree, root, style);
    run(&mut tree, root, 400);

    assert_eq!(tree.get(child).unwrap().layout.width, 100.0);
}

#[test]
fn explicit_and_min_heights_apply() {
    let mut tree = BoxTree::new();
    let root = new_root(&mut tree);
    let tall = block(
        &mut tree,
        root,
        BoxStyle {
            height: Dimension::px(50),
            ..BoxStyle::default()
        },
    );
    let min = block(
        &mut tree,
        root,
        BoxStyle {
            height: Dimension::px(10),
            min_height: Dimension::px(80),
            ..BoxStyle::default()
        },
    );
    run(&mut tree, root, 400);

    assert_eq!(tree.get(tall).unwrap().layout.height, 50.0);
    assert_eq!(tree.get(min).unwrap().layout.y, 50.0);
    assert_eq!(tree.get(min).unwrap().layout.height, 80.0);
}

#[test]
fn layout_is_deterministic() {
    let build = || {
        let mut tree = BoxTree::new();
        let root = new_root(&mut tree);
        let _ = text(
            &mut tree,
            root,
            margins(7, 9),
            "one two three four five six seven",
        );
        let _ = text(&mut tree, root, margins(3, 11), "eight nine ten");
        run(&mut tree, root, 120);
        tree
    };
    assert_eq!(build(), build());
}
