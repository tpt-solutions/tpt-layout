//! Paint metadata, golden layouts, a pagination hash, and randomized robustness checks.

mod common;

use common::*;
use std::fmt::Write as _;
use tpt_layout::{
    layout_document, paginate, paint_order, relayout, BoxStyle, BoxTree, Dimension, Display, Edges,
    FlexDirection, LayoutConstraints, LayoutContext, NodeId, Overflow, PageConstraints, Position,
    Px, TrackSize, Tracks,
};

// --- paint -----------------------------------------------------------------------------------

#[test]
fn paint_order_sorts_by_z_index_then_document_order() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let a = child(
        &mut tree,
        root,
        BoxStyle {
            z_index: Some(2),
            ..boxed(10, 10)
        },
    );
    let b = child(&mut tree, root, boxed(10, 10));
    let c = child(
        &mut tree,
        root,
        BoxStyle {
            z_index: Some(1),
            ..boxed(10, 10)
        },
    );
    layout(&mut tree, root, 100);
    let order: Vec<NodeId> = paint_order(&tree, root).iter().map(|i| i.node).collect();
    assert_eq!(order, [root, b, c, a]);
}

#[test]
fn overflow_clips_descendants_to_the_ancestor() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        overflow: Overflow::Hidden,
        ..boxed(100, 50)
    });
    let inside = child(&mut tree, root, boxed(10, 200));
    layout(&mut tree, root, 100);
    let items = paint_order(&tree, root);
    let clip = items
        .iter()
        .find(|i| i.node == inside)
        .unwrap()
        .clip
        .unwrap();
    assert_eq!(
        (clip.x, clip.y, clip.width, clip.height),
        (px(0), px(0), px(100), px(50))
    );
    assert!(items
        .iter()
        .find(|i| i.node == root)
        .unwrap()
        .clip
        .is_none());
}

// --- golden layouts --------------------------------------------------------------------------

/// Serializes every node's frame as integer px units (1/64 px), so the text is exact on every
/// platform.
fn dump(tree: &BoxTree, root: NodeId) -> String {
    let mut out = String::from("[\n");
    for id in tree_order(tree, root) {
        let node = tree.get(id).unwrap();
        let l = node.layout;
        let raw = |v: f32| (v * 64.0) as i64;
        writeln!(
            out,
            "  {{\"node\": {}, \"x\": {}, \"y\": {}, \"w\": {}, \"h\": {}, \"baseline\": {}}},",
            id.index(),
            raw(l.x),
            raw(l.y),
            raw(l.width),
            raw(l.height),
            raw(l.baseline)
        )
        .unwrap();
    }
    if out.ends_with(",\n") {
        out.truncate(out.len() - 2);
        out.push('\n');
    }
    out.push_str("]\n");
    out
}

fn tree_order(tree: &BoxTree, root: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        out.push(id);
        stack.extend(tree.children(id).iter().rev().copied());
    }
    out
}

fn check_golden(name: &str, actual: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.json"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing golden file {}; run with UPDATE_GOLDEN=1",
            path.display()
        )
    });
    assert_eq!(actual, expected, "golden mismatch for {name}");
}

fn golden_flow() -> String {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let h1 = text_child(
        &mut tree,
        root,
        BoxStyle {
            margin: Edges::all(Dimension::ZERO),
            ..BoxStyle::default()
        },
        "Heading",
    );
    let _ = h1;
    let p = child(
        &mut tree,
        root,
        BoxStyle {
            margin: Edges {
                top: Dimension::px(16),
                bottom: Dimension::px(16),
                ..Edges::all(Dimension::ZERO)
            },
            ..BoxStyle::default()
        },
    );
    text_child(
        &mut tree,
        p,
        BoxStyle::default(),
        "aaa bbb ccc ddd eee fff ggg hhh iii jjj",
    );
    layout(&mut tree, root, 200);
    dump(&tree, root)
}

fn golden_flex_grid() -> String {
    let mut tree = BoxTree::new();
    let flex = tree.insert(BoxStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        ..BoxStyle::default()
    });
    for grow in [1.0, 2.0] {
        child(
            &mut tree,
            flex,
            BoxStyle {
                flex_grow: grow,
                flex_basis: Dimension::px(0),
                height: Dimension::px(20),
                ..BoxStyle::default()
            },
        );
    }
    let grid = tree.insert(BoxStyle {
        display: Display::Grid,
        ..BoxStyle::default()
    });
    tree.set_grid_template(
        grid,
        tpt_layout::GridTemplate {
            columns: Tracks::new(&[TrackSize::Fr(100), TrackSize::Fr(100)]),
            ..Default::default()
        },
    );
    child(&mut tree, grid, boxed(10, 30));
    child(&mut tree, grid, boxed(10, 10));
    let root = tree.insert(BoxStyle::default());
    tree.append_child(root, flex).unwrap();
    tree.append_child(root, grid).unwrap();
    layout(&mut tree, root, 300);
    dump(&tree, root)
}

#[test]
fn golden_flow_layout() {
    check_golden("flow", &golden_flow());
}

#[test]
fn golden_flex_and_grid_layout() {
    check_golden("flex_grid", &golden_flex_grid());
}

#[test]
fn pagination_hash_is_stable() {
    // A long document over several A4-like pages. The hash covers every fragment's position.
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    for index in 0..40 {
        let style = BoxStyle {
            margin: Edges {
                top: Dimension::px(index % 3),
                bottom: Dimension::px(4),
                ..Edges::all(Dimension::ZERO)
            },
            break_before: if index == 17 {
                tpt_layout::Break::Always
            } else {
                tpt_layout::Break::Auto
            },
            ..boxed(0, 60 + (index * 7) % 50)
        };
        child(&mut tree, root, style);
    }
    layout(&mut tree, root, 200);
    let pages = paginate(&tree, root, &PageConstraints::new(px(200), px(297)));
    let mut text = String::new();
    for f in &pages.fragments {
        writeln!(
            text,
            "{} {} {} {} {:?}",
            f.node.index(),
            f.page,
            f.y,
            f.height,
            f.lines
        )
        .unwrap();
    }
    for page in &pages.pages {
        writeln!(
            text,
            "page {} {:?}",
            page.content_offset_y,
            page.boxes.len()
        )
        .unwrap();
    }
    // FNV-1a over the text: integer-only and platform independent.
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    assert_eq!(
        hash, PAGINATION_HASH,
        "pagination hash is {hash:#x}; output was:\n{text}"
    );
}

const PAGINATION_HASH: u64 = 0x2fa336a19d0bf49b;

// --- randomized robustness -------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn dim(&mut self) -> Dimension {
        match self.below(4) {
            0 => Dimension::Auto,
            1 => Dimension::px(self.below(200) as i32),
            2 => Dimension::percent(self.below(120) as f32),
            _ => Dimension::px(0),
        }
    }
}

fn random_style(rng: &mut Rng) -> (BoxStyle, Option<tpt_layout::GridTemplate>) {
    let mut template = None;
    let displays = [
        Display::Block,
        Display::Block,
        Display::Inline,
        Display::InlineBlock,
        Display::Flex,
        Display::Grid,
    ];
    let mut style = BoxStyle {
        display: displays[rng.below(displays.len() as u64) as usize],
        width: rng.dim(),
        height: rng.dim(),
        min_width: if rng.below(4) == 0 {
            Dimension::px(20)
        } else {
            Dimension::Auto
        },
        max_height: if rng.below(4) == 0 {
            Dimension::px(60)
        } else {
            Dimension::Auto
        },
        padding: Edges::all(Dimension::px(rng.below(4) as i32)),
        margin: Edges {
            top: Dimension::px(rng.below(30) as i32 - 10),
            bottom: Dimension::px(rng.below(30) as i32),
            left: if rng.below(5) == 0 {
                Dimension::Auto
            } else {
                Dimension::px(rng.below(10) as i32)
            },
            right: Dimension::px(rng.below(10) as i32),
        },
        flex_grow: rng.below(3) as f32,
        flex_shrink: rng.below(3) as f32,
        flex_basis: rng.dim(),
        order: rng.below(5) as i32 - 2,
        ..BoxStyle::default()
    };
    style.flex_direction = [
        FlexDirection::Row,
        FlexDirection::Column,
        FlexDirection::RowReverse,
    ][rng.below(3) as usize];
    if rng.below(6) == 0 {
        style.position = Position::Relative;
        style.inset.left = Dimension::px(rng.below(10) as i32);
    }
    if rng.below(8) == 0 {
        style.writing_mode = [
            tpt_layout::WritingMode::VerticalRl,
            tpt_layout::WritingMode::VerticalLr,
            tpt_layout::WritingMode::SidewaysRl,
            tpt_layout::WritingMode::SidewaysLr,
        ][rng.below(4) as usize];
    }
    if rng.below(8) == 0 {
        style.position = Position::Absolute;
        style.inset.top = Dimension::px(rng.below(50) as i32);
    }
    if rng.below(8) == 0 {
        style.float = tpt_layout::Float::Left;
    }
    if rng.below(4) == 0 {
        template = Some(tpt_layout::GridTemplate {
            columns: Tracks::new(&[TrackSize::Fr(100), TrackSize::Fixed(Dimension::px(40))]),
            ..Default::default()
        });
    }
    (style, template)
}

fn random_tree(rng: &mut Rng, tree: &mut BoxTree, parent: NodeId, depth: u32) {
    let children = if depth == 0 { 0 } else { rng.below(5) };
    for _ in 0..children {
        if rng.below(10) == 0 {
            let display = if rng.below(2) == 0 {
                Display::Inline
            } else {
                Display::Block
            };
            let style = BoxStyle {
                display,
                ..random_style(rng).0
            };
            let id = tree.insert_replaced(
                style,
                tpt_layout::Px::from_px(rng.below(60) as i32),
                tpt_layout::Px::from_px(rng.below(40) as i32),
            );
            tree.append_child(parent, id).unwrap();
        } else if rng.below(4) == 0 {
            let text = "lorem ipsum dolor sit amet consectetur"
                .split(' ')
                .take(1 + rng.below(6) as usize)
                .collect::<Vec<_>>()
                .join(" ");
            let style = BoxStyle {
                display: if rng.below(2) == 0 {
                    Display::Inline
                } else {
                    Display::Block
                },
                ..BoxStyle::default()
            };
            let id = tree.insert_text(style, text);
            tree.append_child(parent, id).unwrap();
        } else {
            let (style, template) = random_style(rng);
            let id = tree.insert(style);
            if let Some(template) = template {
                tree.set_grid_template(id, template);
            }
            tree.append_child(parent, id).unwrap();
            random_tree(rng, tree, id, depth - 1);
        }
    }
}

/// Describes the first node whose layout differs between two trees, or `None`.
fn first_difference(a: &BoxTree, b: &BoxTree, root: NodeId) -> Option<String> {
    for id in tree_order(a, root) {
        let (na, nb) = (a.get(id)?, b.get(id)?);
        if na.layout != nb.layout || na.fragments != nb.fragments || na.lines != nb.lines {
            let mut chain = String::new();
            let mut cur = Some(id);
            while let Some(p) = cur {
                let n = a.get(p).unwrap();
                chain.push_str(&format!(
                    " <{} {:?}/{:?} wm={:?} w={:?} h={:?}>",
                    p.index(),
                    n.style.display,
                    n.style.position,
                    n.style.writing_mode,
                    n.style.width,
                    n.style.height
                ));
                cur = a.parent(p);
            }
            return Some(format!(
                "node {} (incremental {:?}, full {:?}){chain}",
                id.index(),
                na.layout,
                nb.layout
            ));
        }
    }
    None
}

fn assert_sane(tree: &BoxTree, root: NodeId) {
    for id in tree_order(tree, root) {
        let l = tree.get(id).unwrap().layout;
        for v in [l.x, l.y, l.width, l.height, l.baseline] {
            assert!(
                v.is_finite(),
                "non-finite layout value at node {}",
                id.index()
            );
        }
        assert!(
            l.width >= 0.0 && l.height >= 0.0,
            "negative size at node {}",
            id.index()
        );
    }
}

/// Randomized documents: layout must terminate without panics, be a pure function of the tree
/// (a second pass over its own frames gives the same result), and relayout after an edit must
/// match a full pass. `FUZZ_CASES` overrides the number of cases.
#[test]
fn random_documents_lay_out_without_panics_and_relayout_matches() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let constraints = LayoutConstraints {
        width: px(240),
        height: None,
    };
    let cases: usize = std::env::var("FUZZ_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    let mut rng = Rng(0x9E3779B97F4A7C15);
    for case in 0..cases {
        let mut tree = BoxTree::new();
        let root = tree.insert(BoxStyle {
            width: Dimension::px(240),
            ..BoxStyle::default()
        });
        random_tree(&mut rng, &mut tree, root, 4);
        let result = layout_document(&mut tree, root, constraints, &ctx);
        assert!(result.is_ok(), "case {case}");
        assert_sane(&tree, root);

        // Laying out an already laid-out tree must not change it.
        let mut again = tree.clone();
        layout_document(&mut again, root, constraints, &ctx).unwrap();
        if let Some(diff) = first_difference(&tree, &again, root) {
            panic!("case {case}: layout is not idempotent; first difference {diff}");
        }

        // Dirty a random node, then check incremental matches a full pass.
        let nodes = tree_order(&tree, root);
        let target = nodes[rng.below(nodes.len() as u64) as usize];
        if let Some(node) = tree.get_mut(target) {
            node.style.height = Dimension::px(7);
        }
        let mut full = tree.clone();
        relayout(&mut tree, root, constraints, &ctx).unwrap();
        layout_document(&mut full, root, constraints, &ctx).unwrap();
        if let Some(diff) = first_difference(&tree, &full, root) {
            panic!("case {case}: incremental differs from full layout at {diff}");
        }
    }
}

#[test]
fn zero_and_huge_values_do_not_overflow() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        width: Dimension::px(100),
        ..BoxStyle::default()
    });
    let big = child(
        &mut tree,
        root,
        BoxStyle {
            width: Dimension::px(i32::MAX / 2),
            flex_basis: Dimension::px(i32::MAX / 2),
            ..BoxStyle::default()
        },
    );
    let _ = big;
    let result = layout_document(
        &mut tree,
        root,
        LayoutConstraints {
            width: Px::MAX,
            height: None,
        },
        &ctx,
    );
    assert!(result.is_ok());
    assert_sane(&tree, root);
}
