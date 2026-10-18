//! Text measurement boundary. The real implementation lives in `tpt-text`; this crate only
//! depends on the trait, plus a fixed-advance mock for tests.

use alloc::vec::Vec;

use crate::px::Px;

/// One laid-out line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineBox {
    pub width: Px,
    pub height: Px,
    /// Distance from the top of the line to its baseline.
    pub baseline: Px,
}

pub trait TextMeasurer {
    /// Breaks `text` into lines no wider than `max_width` and reports each line's box.
    /// Must be deterministic: identical inputs produce identical output on every platform.
    /// A line wider than `max_width` (a single long word) is allowed to overflow.
    fn line_boxes(&self, text: &str, max_width: Px) -> Vec<LineBox>;
}

/// Monospace-style mock: every character advances by `advance`. Breaks at spaces and at `\n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedAdvanceMeasurer {
    pub advance: Px,
    pub line_height: Px,
    pub ascent: Px,
}

impl FixedAdvanceMeasurer {
    pub fn new(advance: Px, line_height: Px, ascent: Px) -> Self {
        FixedAdvanceMeasurer {
            advance,
            line_height,
            ascent,
        }
    }

    fn line(&self, chars: i32) -> LineBox {
        LineBox {
            width: self.advance * chars,
            height: self.line_height,
            baseline: self.ascent,
        }
    }
}

impl TextMeasurer for FixedAdvanceMeasurer {
    fn line_boxes(&self, text: &str, max_width: Px) -> Vec<LineBox> {
        let mut lines = Vec::new();
        if text.is_empty() {
            return lines;
        }
        for paragraph in text.split('\n') {
            // Characters on the line being filled; `None` until the first word is placed.
            let mut line_chars: Option<i32> = None;
            for word in paragraph.split(' ') {
                let word_chars = word.chars().count() as i32;
                line_chars = Some(match line_chars {
                    None => word_chars,
                    Some(current) => {
                        let candidate = current + 1 + word_chars;
                        if self.advance * candidate > max_width {
                            lines.push(self.line(current));
                            word_chars
                        } else {
                            candidate
                        }
                    }
                });
            }
            if let Some(chars) = line_chars {
                lines.push(self.line(chars));
            }
        }
        lines
    }
}
