//! The paginated report template: paragraphs split across pages with contiguous line ranges.

use std::collections::BTreeMap;

use tpt_layout::{templates, NodeId, PageConstraints, Px};

fn long_paragraph(words: usize, seed: char) -> String {
    (0..words)
        .map(|i| format!("{seed}{}", "x".repeat(3 + i % 5)))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn report_spans_pages_and_paragraph_lines_stay_contiguous() {
    let a = long_paragraph(30, 'a');
    let b = long_paragraph(25, 'b');
    let report = templates::report("Quarterly report", &[&a, &b]);
    // 60 px pages hold three 16 px lines.
    let page = PageConstraints::new(Px::from_px(400), Px::from_px(60));
    let layout = report.paginate(&page);
    assert!(layout.pages.len() >= 4, "pages: {}", layout.pages.len());

    // For each block with text, its line ranges across all pages, in order, must follow on from
    // one another without gaps or overlaps.
    let mut by_node: BTreeMap<NodeId, Vec<(usize, std::ops::Range<usize>)>> = BTreeMap::new();
    for fragment in &layout.fragments {
        if let Some(lines) = &fragment.lines {
            by_node
                .entry(fragment.node)
                .or_default()
                .push((fragment.page, lines.clone()));
        }
    }
    assert!(!by_node.is_empty());
    for (node, mut parts) in by_node {
        parts.sort_by_key(|(page, range)| (*page, range.start));
        let mut next = 0;
        for (_, range) in &parts {
            assert_eq!(
                range.start, next,
                "node {node:?} lines out of order: {parts:?}"
            );
            next = range.end;
        }
    }
}

#[test]
fn no_page_content_runs_past_the_page() {
    let a = long_paragraph(30, 'a');
    let report = templates::report("Title", &[&a]);
    let page = PageConstraints::new(Px::from_px(400), Px::from_px(60));
    let layout = report.paginate(&page);
    for fragment in &layout.fragments {
        assert!(
            fragment.y + fragment.height <= 60.0 + 0.5,
            "fragment past the page: {fragment:?}"
        );
    }
}
