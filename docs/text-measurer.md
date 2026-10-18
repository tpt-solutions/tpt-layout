# Writing a TextMeasurer

The layout engine calls one method on your measurer:

```rust
fn line_boxes(&self, text: &str, max_width: Px) -> Vec<LineBox>;
```

It breaks `text` into lines no wider than `max_width` and reports each line:

- `width`: the width of the line's text, not counting the break space.
- `height`: the line's height. Use your font's line height.
- `baseline`: the distance from the top of the line to its baseline.
- `text`: the byte range of `text` that the line shows. Leave out the space the line breaks at.

## A ready-made implementation

`tools/text` (`tpt-layout-text`) implements this trait with tpt-text, which does shaping, bidi and
UAX #14 line breaking. Its `TextEngine` is the reference to read when writing your own.

## Rules the engine relies on

- **Deterministic.** The same input must give the same output on every platform. The determinism
  tests compare hashes of layouts across operating systems.
- **Overflow is allowed.** A single word wider than `max_width` should become its own line, even
  though it overflows.
- **Ranges are exact.** Each line's `text` range must lie inside `text`, and the ranges of
  consecutive lines should follow one another without overlap. Renderers and pagination read them.
- **Newlines.** A `\n` in the text ends a line. Empty text gives no lines.

## A minimal sketch

```rust
struct FontMeasurer { /* your font */ }

impl TextMeasurer for FontMeasurer {
    fn line_boxes(&self, text: &str, max_width: Px) -> Vec<LineBox> {
        // Split at spaces and newlines, measure each word, and fill lines greedily up to
        // `max_width`. Track byte offsets so each line's `text` range is exact.
        todo!()
    }
}
```

`FixedAdvanceMeasurer` in `src/text.rs` is a complete, short example. It fills lines greedily by
character count and tracks byte ranges, so it is a good model for a real implementation.

## Checking your measurer

- Lay out a paragraph at a few widths, and check that no line is wider than `max_width` unless
  it holds a single word.
- Check that the text ranges cover the words in order, with no gaps other than break spaces.
- Run the same layout on two machines, or compare the output of
  `snapshot::fingerprint` against a value recorded on another platform.
