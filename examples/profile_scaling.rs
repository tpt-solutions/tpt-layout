//! Measures full-layout time against document size for each layout mode. Superlinear growth
//! points at an algorithm to fix. Run with `cargo run --release --example profile_scaling`.

use std::time::Instant;

use tpt_layout::{
    layout_document, BoxStyle, BoxTree, Dimension, Display, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Px, TrackSize, Tracks,
};

fn measurer() -> FixedAdvanceMeasurer {
    FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(20), Px::from_px(15))
}

/// A flat block list of `n` paragraphs.
fn blocks(n: usize) -> (BoxTree, NodeId) {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    for i in 0..n {
        let p = tree.insert_text(
            BoxStyle::default(),
            format!("paragraph {i} with some words in it"),
        );
        tree.append_child(root, p).unwrap();
    }
    (tree, root)
}

/// A flex row of `n` cards, each with a text child.
fn flex(n: usize) -> (BoxTree, NodeId) {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        display: Display::Flex,
        flex_wrap: tpt_layout::FlexWrap::Wrap,
        ..BoxStyle::default()
    });
    for i in 0..n {
        let card = tree.insert(BoxStyle {
            flex_basis: Dimension::px(60),
            ..BoxStyle::default()
        });
        tree.append_child(root, card).unwrap();
        let t = tree.insert_text(BoxStyle::default(), format!("card {i}"));
        tree.append_child(card, t).unwrap();
    }
    (tree, root)
}

/// A grid of `n` cells in four columns.
fn grid(n: usize) -> (BoxTree, NodeId) {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle {
        display: Display::Grid,
        ..BoxStyle::default()
    });
    tree.set_grid_template(
        root,
        tpt_layout::GridTemplate {
            columns: Tracks::new(&[TrackSize::Fr(100); 4]),
            ..Default::default()
        },
    );
    for i in 0..n {
        let cell = tree.insert_text(BoxStyle::default(), format!("cell {i}"));
        tree.append_child(root, cell).unwrap();
    }
    (tree, root)
}

/// Nested blocks `depth` levels deep, `n` boxes in total along a single chain of wrappers.
fn nested(n: usize) -> (BoxTree, NodeId) {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let mut parent = root;
    for i in 0..n {
        let wrapper = tree.insert(BoxStyle {
            padding: tpt_layout::Edges::all(Dimension::px(1)),
            ..BoxStyle::default()
        });
        tree.append_child(parent, wrapper).unwrap();
        let t = tree.insert_text(BoxStyle::default(), format!("level {i}"));
        tree.append_child(wrapper, t).unwrap();
        parent = wrapper;
    }
    (tree, root)
}

type Builder = fn(usize) -> (BoxTree, NodeId);

fn time(build: Builder, n: usize) -> f64 {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let constraints = LayoutConstraints {
        width: Px::from_px(600),
        height: None,
    };
    let mut samples = Vec::new();
    for _ in 0..5 {
        let (mut tree, root) = build(n);
        let start = Instant::now();
        let result = layout_document(&mut tree, root, constraints, &ctx);
        if result.is_err() {
            return f64::NAN;
        }
        samples.push(start.elapsed().as_secs_f64() * 1e9 / n as f64);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    samples[2]
}

fn main() {
    let sizes = [1_000usize, 4_000, 16_000];
    let depths = [25usize, 50, 100, 128, 129, 300];
    for d in depths {
        let t = time(nested, d);
        if t.is_nan() {
            println!("nested depth {d}: TooDeep");
        } else {
            println!("nested depth {d}: {t:.0} ns per box");
        }
    }
    let modes: [(&str, Builder); 3] = [("blocks", blocks), ("flex", flex), ("grid", grid)];
    println!("ns per box, median of 5 (flat sizes {:?})", sizes);
    if std::env::var_os("ONLY_DEPTH").is_some() {
        return;
    }
    for (name, build) in modes {
        let row: Vec<String> = sizes
            .iter()
            .map(|&n| format!("{:>8.0}", time(build, n)))
            .collect();
        println!("{name:>8}: {}", row.join(" "));
    }
}
