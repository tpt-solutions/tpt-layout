//! SVG debug renderer. Draws every box outline, clipped by its paint clip region, and every text
//! fragment and line. It is for looking at a layout, not for production output.
//!
//! Text is drawn with its characters, inline and block-level. Each block-level line is also drawn
//! as an outline. Vertical text has each character in its own cell. This is the first
//! implementation of [`Canvas`]. Text is escaped, so any source string is safe to embed.

use alloc::{format, string::String};

use crate::{
    canvas::Canvas,
    layout::ComputedLayout,
    paint::paint_order,
    px::Px,
    style::WritingMode,
    tree::{BoxTree, NodeId},
};

/// Collects SVG markup for boxes. Use [`render_svg`] unless you need to draw a subset.
#[derive(Default)]
pub struct SvgCanvas {
    body: String,
}

impl SvgCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    /// The markup drawn so far, without the `<svg>` wrapper.
    pub fn body(&self) -> &str {
        &self.body
    }
}

impl Canvas for SvgCanvas {
    fn draw_box(&mut self, node: NodeId, layout: &ComputedLayout) {
        self.body.push_str(&format!(
            "<rect data-node=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"#8888cc\" stroke-width=\"0.5\"/>\n",
            node.index(),
            layout.x,
            layout.y,
            layout.width,
            layout.height
        ));
    }
}

/// Renders the laid-out tree under `root` as a standalone SVG document of the given size in px.
pub fn render_svg(tree: &BoxTree, root: NodeId, width: f32, height: f32) -> String {
    let mut canvas = SvgCanvas::new();
    for item in paint_order(tree, root) {
        let Some(node) = tree.get(item.node) else {
            continue;
        };
        canvas.draw_box(item.node, &node.layout);
        // Block-level text has line boxes. Each line is drawn as an outline, stacked from the top of
        // the box, and its characters are drawn too. Vertical text is not drawn line by line.
        let vertical = is_vertical(tree, item.node);
        // Only text leaves have lines with characters; a block's inline lines have no origin here.
        let lines = if node.text.is_some() {
            &node.lines[..]
        } else {
            &node.lines[..0]
        };
        let mut line_top = node.layout.y;
        for (line, line_box) in lines.iter().enumerate() {
            canvas.body.push_str(&format!(
                "<rect data-node=\"{}\" data-line=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#eef\" stroke=\"#8888cc\" stroke-width=\"0.25\"/>\n",
                item.node.index(),
                line,
                node.layout.x,
                line_top,
                line_box.width.to_f32(),
                line_box.height.to_f32()
            ));
            if !vertical {
                if let Some(run) = node
                    .text
                    .as_deref()
                    .and_then(|t| t.get(line_box.text.clone()))
                {
                    canvas.body.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"{}\" fill=\"#222\">{}</text>\n",
                        node.layout.x,
                        line_top + line_box.baseline.to_f32(),
                        line_box.height.to_f32(),
                        escape(run)
                    ));
                }
            }
            line_top += line_box.height.to_f32();
        }
        for fragment in &node.fragments {
            let Some(text) = node.text.as_deref() else {
                continue;
            };
            let Some(run) = text.get(fragment.text_range.clone()) else {
                continue;
            };
            if is_vertical(tree, item.node) {
                // Vertical text: each character sits in its own cell down the fragment. The cell
                // size is the fragment's length divided by its character count. This is exact for
                // the fixed-advance measurer and an approximation for proportional fonts.
                let count = run.chars().count().max(1) as i32;
                let advance = Px(fragment.height.0 / count);
                let font_size = fragment.width.to_f32();
                for (index, ch) in run.chars().enumerate() {
                    let top = fragment.y + advance * index as i32;
                    let mut buf = [0u8; 4];
                    canvas.body.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"{}\" fill=\"#222\">{}</text>\n",
                        fragment.x.to_f32(),
                        top.to_f32() + font_size,
                        font_size,
                        escape(ch.encode_utf8(&mut buf))
                    ));
                }
                continue;
            }
            let baseline = fragment.y.to_f32() + fragment.height.to_f32();
            canvas.body.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"{}\" fill=\"#222\">{}</text>\n",
                fragment.x.to_f32(),
                baseline,
                fragment.height.to_f32(),
                escape(run)
            ));
        }
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n{}</svg>\n",
        canvas.body
    )
}

/// Whether text under `id` runs top to bottom. Uses the first writing mode that is not `Inherit`.
/// Sideways modes are not drawn specially, so they count as horizontal.
fn is_vertical(tree: &BoxTree, mut id: NodeId) -> bool {
    loop {
        let Some(node) = tree.get(id) else {
            return false;
        };
        match node.style.writing_mode {
            WritingMode::Inherit => match tree.parent(id) {
                Some(parent) => id = parent,
                None => return false,
            },
            mode => return matches!(mode, WritingMode::VerticalRl | WritingMode::VerticalLr),
        }
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}
