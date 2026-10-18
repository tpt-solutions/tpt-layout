//! Deeper writing-mode nesting, and right-to-left inside a vertical box. Exact positions are checked
//! in the focused writing-mode tests; here the check is that every inner box lies inside its
//! ancestors, and that layout completes.

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Direction, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Px, WritingMode,
};

fn sized(writing_mode: WritingMode, direction: Direction, width: i32, height: i32) -> BoxStyle {
    BoxStyle {
        writing_mode,
        direction,
        width: Dimension::Length(Px::from_px(width)),
        height: Dimension::Length(Px::from_px(height)),
        ..BoxStyle::default()
    }
}

/// Lays out `root` and returns the layout rectangle of every node under it, as (x, y, w, h).
fn rects(tree: &BoxTree, root: NodeId) -> Vec<(NodeId, (f32, f32, f32, f32))> {
    tree.subtree_ids(root)
        .into_iter()
        .map(|id| {
            let l = tree.get(id).unwrap().layout;
            (id, (l.x, l.y, l.width, l.height))
        })
        .collect()
}

fn inside(inner: (f32, f32, f32, f32), outer: (f32, f32, f32, f32)) -> bool {
    let (ix, iy, iw, ih) = inner;
    let (ox, oy, ow, oh) = outer;
    // A small tolerance covers the 1/64 px rounding of the frames.
    let eps = 0.5;
    ix >= ox - eps && iy >= oy - eps && ix + iw <= ox + ow + eps && iy + ih <= oy + oh + eps
}

#[test]
fn horizontal_box_inside_vertical_lr_inside_vertical_rl_stays_in_its_ancestors() {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let outer = tree.insert(sized(WritingMode::VerticalRl, Direction::Ltr, 200, 200));
    let middle = tree.insert(sized(WritingMode::VerticalLr, Direction::Ltr, 120, 120));
    let inner = tree.insert(sized(WritingMode::HorizontalTb, Direction::Ltr, 40, 20));
    tree.append_child(outer, middle).unwrap();
    tree.append_child(middle, inner).unwrap();

    layout_document(
        &mut tree,
        outer,
        LayoutConstraints {
            width: Px::from_px(200),
            height: Some(Px::from_px(200)),
        },
        &ctx,
    )
    .unwrap();

    let all = rects(&tree, outer);
    let rect = |id: NodeId| all.iter().find(|(n, _)| *n == id).unwrap().1;
    assert!(
        inside(rect(middle), rect(outer)),
        "middle {:?}",
        rect(middle)
    );
    assert!(inside(rect(inner), rect(middle)), "inner {:?}", rect(inner));
}

#[test]
fn right_to_left_text_inside_a_vertical_box_stays_inside_it() {
    let m = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(16), Px::from_px(12));
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let outer = tree.insert(sized(WritingMode::VerticalRl, Direction::Rtl, 160, 160));
    let child = tree.insert(sized(WritingMode::Inherit, Direction::Inherit, 50, 50));
    tree.append_child(outer, child).unwrap();

    layout_document(
        &mut tree,
        outer,
        LayoutConstraints {
            width: Px::from_px(160),
            height: Some(Px::from_px(160)),
        },
        &ctx,
    )
    .unwrap();

    let all = rects(&tree, outer);
    let rect = |id: NodeId| all.iter().find(|(n, _)| *n == id).unwrap().1;
    assert!(inside(rect(child), rect(outer)), "child {:?}", rect(child));
}
