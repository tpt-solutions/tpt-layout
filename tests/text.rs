//! Text measurement: line breaking and baselines through the mock measurer.

use tpt_layout::{layout_flow, BoxStyle, BoxTree, FixedAdvanceMeasurer, LineBox, Px, TextMeasurer};

fn mock() -> FixedAdvanceMeasurer {
    FixedAdvanceMeasurer::new(Px::from_px(10), Px::from_px(20), Px::from_px(15))
}

#[test]
fn breaks_on_word_boundaries_within_max_width() {
    let lines = mock().line_boxes("aaa bbb ccc", Px::from_px(70));
    assert_eq!(
        lines,
        vec![
            LineBox {
                width: Px::from_px(70),
                height: Px::from_px(20),
                baseline: Px::from_px(15)
            },
            LineBox {
                width: Px::from_px(30),
                height: Px::from_px(20),
                baseline: Px::from_px(15)
            },
        ]
    );
}

#[test]
fn newline_forces_a_break_and_blank_lines_keep_their_height() {
    let lines = mock().line_boxes("a\n\nb", Px::from_px(1000));
    let heights: Vec<Px> = lines.iter().map(|l| l.height).collect();
    let widths: Vec<Px> = lines.iter().map(|l| l.width).collect();
    assert_eq!(heights.len(), 3);
    assert_eq!(widths, vec![Px::from_px(10), Px::ZERO, Px::from_px(10)]);
}

#[test]
fn empty_text_has_no_lines() {
    assert!(mock().line_boxes("", Px::from_px(100)).is_empty());
}

#[test]
fn overlong_word_overflows_its_own_line() {
    let lines = mock().line_boxes("supercalifragilistic", Px::from_px(50));
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].width, Px::from_px(200));
}

#[test]
fn text_box_height_and_baseline_come_from_its_lines() {
    let mut tree = BoxTree::new();
    let root = tree.insert(BoxStyle::default());
    // 40 px wide at 10 px per char: "aaa bbb" (70 px) does not fit, so "aaa" and "bbb" stack.
    let text = tree.insert_text(BoxStyle::default(), "aaa bbb");
    tree.append_child(root, text).unwrap();
    layout_flow(&mut tree, root, Px::from_px(40), &mock()).unwrap();

    let node = tree.get(text).unwrap().layout;
    assert_eq!(node.height, 40.0);
    assert_eq!(node.baseline, 15.0);
}
