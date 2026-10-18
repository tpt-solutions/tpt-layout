//! The nesting limit: deep trees lay out up to `MAX_DEPTH` and report `TooDeep` past it.

mod common;

use common::*;
use tpt_layout::{layout_document, BoxStyle, BoxTree, LayoutConstraints, LayoutContext, TreeError};

fn chain(depth: usize) -> (BoxTree, tpt_layout::NodeId) {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let mut parent = root;
    for _ in 0..depth {
        let wrapper = tree.insert(BoxStyle {
            padding: tpt_layout::Edges::all(tpt_layout::Dimension::px(1)),
            ..BoxStyle::default()
        });
        tree.append_child(parent, wrapper).unwrap();
        parent = wrapper;
    }
    let leaf = tree.insert_text(BoxStyle::default(), "leaf");
    tree.append_child(parent, leaf).unwrap();
    (tree, root)
}

fn run(depth: usize) -> Result<tpt_layout::LayoutResult, TreeError> {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let (mut tree, root) = chain(depth);
    layout_document(
        &mut tree,
        root,
        LayoutConstraints {
            width: px(400),
            height: None,
        },
        &ctx,
    )
}

#[test]
fn a_deep_but_supported_tree_lays_out() {
    let depth = tpt_layout::flow::MAX_DEPTH as usize - 4;
    assert!(run(depth).is_ok());
}

#[test]
fn a_tree_past_the_limit_reports_too_deep() {
    let depth = tpt_layout::flow::MAX_DEPTH as usize + 8;
    assert_eq!(run(depth), Err(TreeError::TooDeep));
}
