//! Measures full layout against incremental re-layout after a single text change.
//!
//! Run with `cargo run --release --example incremental_bench`. The document is made of
//! positioned, fixed-size sections, so each one is a layout boundary (see `engine.rs`). Results
//! depend on the machine; the ratio is what the TODO's 10x target refers to.

use std::time::Instant;

use tpt_layout::{
    layout_document, relayout, BoxStyle, BoxTree, Dimension, FixedAdvanceMeasurer,
    LayoutConstraints, LayoutContext, NodeId, Position, Px,
};

const SECTIONS: usize = 2000;
const ROUNDS: usize = 15;

fn build() -> (BoxTree, NodeId, NodeId) {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let mut target = None;
    for index in 0..SECTIONS {
        let section = tree.insert(BoxStyle {
            position: Position::Relative,
            overflow: tpt_layout::Overflow::Hidden,
            width: Dimension::px(400),
            height: Dimension::px(40),
            ..BoxStyle::default()
        });
        tree.append_child(root, section).unwrap();
        let text = tree.insert_text(BoxStyle::default(), format!("section {index} text"));
        tree.append_child(section, text).unwrap();
        if index == SECTIONS / 2 {
            target = Some(text);
        }
    }
    (tree, root, target.unwrap())
}

fn median(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    samples[samples.len() / 2]
}

fn main() {
    let measurer = FixedAdvanceMeasurer::new(Px::from_px(8), Px::from_px(20), Px::from_px(15));
    let ctx = LayoutContext::new(&measurer);
    let constraints = LayoutConstraints {
        width: Px::from_px(400),
        height: None,
    };

    let (mut tree, root, target) = build();
    layout_document(&mut tree, root, constraints, &ctx).unwrap();

    let mut full = Vec::new();
    for _ in 0..ROUNDS {
        let mut fresh = tree.clone();
        fresh.get_mut(target).unwrap().text = Some("changed".into());
        let start = Instant::now();
        layout_document(&mut fresh, root, constraints, &ctx).unwrap();
        full.push(start.elapsed().as_secs_f64() * 1e6);
    }

    let mut incremental = Vec::new();
    for round in 0..ROUNDS {
        let text = if round % 2 == 0 {
            "changed text"
        } else {
            "another change"
        };
        tree.get_mut(target).unwrap().text = Some(text.into());
        let start = Instant::now();
        relayout(&mut tree, root, constraints, &ctx).unwrap();
        incremental.push(start.elapsed().as_secs_f64() * 1e6);
    }

    let full_us = median(full);
    let incremental_us = median(incremental);
    println!("sections:            {SECTIONS}");
    println!("full layout:         {full_us:>10.1} us (median of {ROUNDS})");
    println!("single-box relayout: {incremental_us:>10.1} us (median of {ROUNDS})");
    println!("speed-up:            {:>10.1}x", full_us / incremental_us);
}
