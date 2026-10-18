//! Text measurement boundary. The real implementation lives in `tpt-text`; this crate only
//! depends on the trait, plus a fixed-advance mock for tests.

use alloc::vec::Vec;
use core::ops::Range;

use crate::px::Px;

/// One laid-out line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineBox {
    pub width: Px,
    pub height: Px,
    /// Distance from the top of the line to its baseline.
    pub baseline: Px,
    /// Byte range of the source text this line shows, without the break space that ends it.
    pub text: Range<usize>,
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

    fn line(&self, chars: i32, text: Range<usize>) -> LineBox {
        LineBox {
            width: self.advance * chars,
            height: self.line_height,
            baseline: self.ascent,
            text,
        }
    }
}

impl TextMeasurer for FixedAdvanceMeasurer {
    fn line_boxes(&self, text: &str, max_width: Px) -> Vec<LineBox> {
        let mut lines = Vec::new();
        if text.is_empty() {
            return lines;
        }
        // Byte offset of the start of the current paragraph within `text`.
        let mut paragraph_start = 0;
        for paragraph in text.split('\n') {
            // The line being filled: characters, and its byte range within `text`. `None` until
            // the first word is placed.
            let mut line: Option<(i32, Range<usize>)> = None;
            // Byte offset of the current word within `text`.
            let mut word_start = paragraph_start;
            for word in paragraph.split(' ') {
                let word_end = word_start + word.len();
                let word_chars = word.chars().count() as i32;
                line = Some(match line {
                    None => (word_chars, word_start..word_end),
                    Some((current, range)) => {
                        let candidate = current + 1 + word_chars;
                        if self.advance * candidate > max_width {
                            lines.push(self.line(current, range));
                            (word_chars, word_start..word_end)
                        } else {
                            (candidate, range.start..word_end)
                        }
                    }
                });
                // Skip the space that separated this word from the next.
                word_start = word_end + 1;
            }
            if let Some((chars, range)) = line {
                lines.push(self.line(chars, range));
            }
            // Skip the newline that ended this paragraph.
            paragraph_start += paragraph.len() + 1;
        }
        lines
    }
}
