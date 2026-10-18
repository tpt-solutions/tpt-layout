//! Property tests over generated documents: layout is idempotent, and pagination splits each
//! text box's lines into contiguous, non-overlapping ranges starting at line 0.

use tpt_layout::{
    layout_document, paginate, snapshot, BoxStyle, BoxTree, Dimension, Display,
    FixedAdvanceMeasurer, LayoutConstraints, LayoutContext, NodeId, PageConstraints, Px,
};

/// A small deterministic generator (xorshift), so failures reproduce from the seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// A root with a random number of blocks, each either a text paragraph or a fixed-height box.
/// Returns the tree, the root, and the text paragraphs.
fn random_document(seed: u64) -> (BoxTree, NodeId, Vec<NodeId>) {
    let mut rng = Rng(seed | 1);
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    let mut texts = Vec::new();
    for _ in 0..(1 + rng.below(40)) {
        if rng.below(3) == 0 {
            let block = tree.insert(BoxStyle {
                height: Dimension::Length(Px::from_px(10 + rng.below(200) as i32)),
                ..BoxStyle::default()
            });
            tree.append_child(root, block).unwrap();
        } else {
            let words = 1 + rng.below(120);
            let text: String = (0..words)
                .map(|_| "lorem ".repeat(1 + rng.below(3) as usize))
                .collect();
            let para = tree.insert_text(
                BoxStyle {
                    display: Display::Block,
                    ..BoxStyle::default()
                },
                text,
            );
            tree.append_child(root, para).unwrap();
            texts.push(para);
        }
    }
    (tree, root, texts)
}

fn measurer() -> FixedAdvanceMeasurer {
    FixedAdvanceMeasurer::new(Px::from_px(7), Px::from_px(14), Px::from_px(11))
}

#[test]
fn layout_is_idempotent() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let constraints = LayoutConstraints {
        width: Px::from_px(320),
        height: None,
    };
    for seed in 1..=60 {
        let (mut tree, root, _) = random_document(seed);
        layout_document(&mut tree, root, constraints, &ctx).unwrap();
        let first = snapshot::fingerprint(&tree, root);
        layout_document(&mut tree, root, constraints, &ctx).unwrap();
        let second = snapshot::fingerprint(&tree, root);
        assert_eq!(
            first, second,
            "seed {seed}: second layout differs from first"
        );
    }
}

#[test]
fn pagination_covers_each_text_line_exactly_once() {
    let m = measurer();
    let ctx = LayoutContext::new(&m);
    let constraints = LayoutConstraints {
        width: Px::from_px(320),
        height: None,
    };
    let page = PageConstraints::a4();
    let mut checked = 0;
    for seed in 1..=60 {
        let (mut tree, root, texts) = random_document(seed);
        layout_document(&mut tree, root, constraints, &ctx).unwrap();
        let layout = paginate(&tree, root, &page);
        for text in texts {
            let mut ranges: Vec<_> = layout
                .fragments
                .iter()
                .filter(|f| f.node == text)
                .filter_map(|f| f.lines.clone())
                .collect();
            if ranges.is_empty() {
                continue;
            }
            ranges.sort_by_key(|r| r.start);
            checked += 1;
            assert_eq!(ranges[0].start, 0, "seed {seed}: lines must start at 0");
            for pair in ranges.windows(2) {
                assert_eq!(
                    pair[0].end, pair[1].start,
                    "seed {seed}: line ranges must be contiguous and disjoint"
                );
            }
        }
    }
    assert!(
        checked > 0,
        "no paragraph produced line fragments; the test checked nothing"
    );
}
