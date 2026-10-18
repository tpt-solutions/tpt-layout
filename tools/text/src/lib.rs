//! Real text measurement for tpt-layout, using tpt-text.
//!
//! [`TextEngine`] implements tpt-layout's [`TextMeasurer`]. Each line that tpt-text breaks is
//! reported with its width (the sum of its glyph advances), its height and baseline, and the
//! bytes of text it covers. The layout engine then breaks and positions text with the metrics of
//! the font you supply.
//!
//! ```no_run
//! use tpt_layout::{layout_document, BoxStyle, BoxTree, LayoutConstraints, LayoutContext, Px};
//! use tpt_layout_text::TextEngine;
//!
//! let font = std::fs::read("NotoSans-Regular.ttf")?;
//! let engine = TextEngine::new(font, Px::from_px(16))?;
//! let ctx = LayoutContext::new(&engine);
//! # let mut tree = BoxTree::new();
//! # let root = tree.insert(BoxStyle::default());
//! # let _ = layout_document(&mut tree, root, LayoutConstraints { width: Px::from_px(300), height: None }, &ctx);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::ops::Range;

use tpt_layout::{LineBox, Px, TextMeasurer};
use tpt_text::{layout_text, FontRegistry, Fx, TextStyle};

/// The widest line tpt-text is asked to break. Its fixed-point range is about 32,000 px, so wider
/// requests (such as the layout engine's unbounded measurements) are capped here.
const MAX_WIDTH_PX: f32 = 30_000.0;

/// Measures text in one font family at one size.
pub struct TextEngine {
    registry: FontRegistry,
    style: TextStyle,
}

impl TextEngine {
    /// Registers `font` (TrueType or OpenType data) and measures text in its family at `size`.
    pub fn new(font: Vec<u8>, size: Px) -> Result<Self, String> {
        let registry = FontRegistry::new();
        let face = registry
            .load_font(font, 0)
            .map_err(|error| format!("{error:?}"))?;
        let style = TextStyle::new(face.family(), Fx::from_f32(size.to_f32()));
        Ok(TextEngine { registry, style })
    }
}

impl TextMeasurer for TextEngine {
    /// Breaks `text` into lines no wider than `max_width`. If tpt-text cannot lay the text out (for
    /// example because the font is missing), there are no lines, so the text takes no space.
    fn line_boxes(&self, text: &str, max_width: Px) -> Vec<LineBox> {
        let width = Fx::from_f32(max_width.to_f32().clamp(0.0, MAX_WIDTH_PX));
        let Ok(layout) = layout_text(&self.registry, text, &self.style, width) else {
            return Vec::new();
        };
        layout
            .lines
            .iter()
            .map(|line| {
                let advance: f32 = line
                    .runs
                    .iter()
                    .flat_map(|run| run.glyphs.iter())
                    .map(|glyph| glyph.x_advance.to_f32())
                    .sum();
                LineBox {
                    width: Px::from_f32(advance),
                    height: Px::from_f32(line.height.to_f32()),
                    baseline: Px::from_f32(line.baseline_y.to_f32() - line.top.to_f32()),
                    text: trim_end(text, line.text_range.clone()),
                }
            })
            .collect()
    }
}

/// `range` without trailing whitespace. The break space at the end of a line is not part of the
/// line's text.
fn trim_end(text: &str, range: Range<usize>) -> Range<usize> {
    let covered = text.get(range.clone()).unwrap_or("");
    range.start..range.start + covered.trim_end().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &[u8] = include_bytes!("../tests/fonts/NotoSans-Regular.ttf");

    fn engine() -> TextEngine {
        TextEngine::new(FONT.to_vec(), Px::from_px(16)).expect("font loads")
    }

    #[test]
    fn short_text_is_one_line_of_real_width() {
        let lines = engine().line_boxes("hello", Px::from_px(1000));
        assert_eq!(lines.len(), 1);
        assert!(lines[0].width > Px::ZERO);
        assert!(lines[0].height > Px::ZERO);
        assert_eq!(lines[0].text, 0..5);
    }

    #[test]
    fn narrow_widths_break_at_spaces() {
        let text = "hello world";
        let wide = engine().line_boxes(text, Px::from_px(1000));
        assert_eq!(wide.len(), 1);
        // "hello" is about 41 px and "world" about 43 px in this font, so 60 px holds one word a line.
        let narrow = engine().line_boxes(text, Px::from_px(60));
        assert_eq!(narrow.len(), 2, "{narrow:?}");
        assert_eq!(narrow[0].text, 0..5);
        assert_eq!(narrow[1].text, 6..11);
    }

    #[test]
    fn a_word_wider_than_the_box_is_broken_inside_the_word() {
        // At 40 px "world" does not fit, so tpt-text breaks it (an emergency break). The layout
        // engine receives the pieces as separate lines.
        let lines = engine().line_boxes("hello world", Px::from_px(40));
        assert!(lines.len() >= 3, "{lines:?}");
        assert_eq!(lines.last().unwrap().text.end, 11);
    }

    #[test]
    fn the_break_space_is_not_part_of_the_line() {
        let lines = engine().line_boxes("hello ", Px::from_px(1000));
        assert_eq!(lines[0].text, 0..5);
    }

    #[test]
    fn measuring_is_deterministic() {
        let a = engine().line_boxes("the same text twice", Px::from_px(90));
        let b = engine().line_boxes("the same text twice", Px::from_px(90));
        assert_eq!(a, b);
    }

    #[test]
    fn the_layout_engine_uses_the_real_metrics() {
        use tpt_layout::{
            layout_document, BoxStyle, BoxTree, Dimension, Display, LayoutConstraints,
            LayoutContext,
        };
        let measurer = engine();
        let ctx = LayoutContext::new(&measurer);
        let mut tree = BoxTree::new();
        let root = tree.insert(BoxStyle {
            width: Dimension::Length(Px::from_px(60)),
            ..BoxStyle::default()
        });
        let paragraph = tree.insert(BoxStyle::default());
        tree.append_child(root, paragraph).unwrap();
        let text = tree.insert_text(
            BoxStyle {
                display: Display::Inline,
                ..BoxStyle::default()
            },
            "a longer sentence that must wrap",
        );
        tree.append_child(paragraph, text).unwrap();
        let result = layout_document(
            &mut tree,
            root,
            LayoutConstraints {
                width: Px::from_px(60),
                height: None,
            },
            &ctx,
        )
        .unwrap();
        assert!(result.height > Px::from_px(16), "text should wrap onto several lines");
    }
}
