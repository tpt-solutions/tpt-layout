//! A JSON description of a tpt-layout box tree, and the output the CLI and playground produce.
//!
//! A document has a root width, optional text metrics, and a tree of nodes:
//!
//! ```json
//! {
//!   "width": 400,
//!   "measure": { "advance": 8, "line_height": 16, "ascent": 12 },
//!   "root": {
//!     "display": "flex",
//!     "children": [
//!       { "width": 120, "padding": 8, "children": [{ "text": "Left" }] },
//!       { "flex_grow": 1, "children": [{ "text": "Right side" }] }
//!     ]
//!   }
//! }
//! ```
//!
//! Sizes are numbers (px), `"auto"`, or a percentage string such as `"50%"`. Enum values are
//! lowercase names, listed on [`Node`]. Unknown fields are rejected, so a typo is an error.

use serde::Deserialize;
use serde_json::{json, Value};
use tpt_layout::{
    layout_document, svg, BoxStyle, BoxTree, Dimension, Direction, Display, Edges,
    FixedAdvanceMeasurer, FlexDirection, JustifyContent, LayoutConstraints, LayoutContext, NodeId,
    Overflow, Px, TextAlign, WritingMode,
};

/// A whole document.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    /// Width of the root's containing block, in px.
    pub width: f32,
    /// Height of the containing block, in px. Omit to let the content set the height.
    #[serde(default)]
    pub height: Option<f32>,
    /// Text metrics. The defaults are 8 px per character, 16 px lines, 12 px ascent.
    #[serde(default)]
    pub measure: Measure,
    /// The root box.
    pub root: Node,
}

/// Fixed-advance text metrics, in px.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measure {
    #[serde(default = "default_advance")]
    pub advance: f32,
    #[serde(default = "default_line_height")]
    pub line_height: f32,
    #[serde(default = "default_ascent")]
    pub ascent: f32,
}

impl Default for Measure {
    fn default() -> Self {
        Measure {
            advance: default_advance(),
            line_height: default_line_height(),
            ascent: default_ascent(),
        }
    }
}

fn default_advance() -> f32 {
    8.0
}
fn default_line_height() -> f32 {
    16.0
}
fn default_ascent() -> f32 {
    12.0
}

/// A size: px, `"auto"`, or a percentage such as `"50%"`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Size {
    Px(f32),
    Text(String),
}

/// A box, or a text leaf when `text` is set.
///
/// `display` is one of `block` (default), `inline`, `inline-block`, `flex`, `grid`, `table`,
/// `table-row`, `table-cell` or `none`. `text_align` is `start`, `center`, `end` or `justify`.
/// `direction` is `ltr` or `rtl`. `writing_mode` is `horizontal-tb`, `vertical-rl` or `vertical-lr`.
/// `flex_direction` is `row`, `row-reverse`, `column` or `column-reverse`. `justify_content` is
/// `flex-start`, `flex-end`, `center`, `space-between`, `space-around` or `space-evenly`.
/// `overflow` is `visible`, `hidden` or `scroll`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    #[serde(default)]
    pub display: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub width: Option<Size>,
    #[serde(default)]
    pub height: Option<Size>,
    #[serde(default)]
    pub min_width: Option<Size>,
    #[serde(default)]
    pub max_width: Option<Size>,
    #[serde(default)]
    pub min_height: Option<Size>,
    #[serde(default)]
    pub max_height: Option<Size>,
    /// Padding on every side, in px.
    #[serde(default)]
    pub padding: Option<f32>,
    /// Margin on every side, in px.
    #[serde(default)]
    pub margin: Option<f32>,
    /// Border width on every side, in px.
    #[serde(default)]
    pub border: Option<f32>,
    #[serde(default)]
    pub flex_grow: Option<f32>,
    #[serde(default)]
    pub flex_direction: Option<String>,
    #[serde(default)]
    pub justify_content: Option<String>,
    #[serde(default)]
    pub column_count: Option<u16>,
    #[serde(default)]
    pub column_gap: Option<f32>,
    #[serde(default)]
    pub text_align: Option<String>,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub writing_mode: Option<String>,
    #[serde(default)]
    pub overflow: Option<String>,
    #[serde(default)]
    pub column_span: Option<u16>,
    #[serde(default)]
    pub row_span: Option<u16>,
    #[serde(default)]
    pub children: Vec<Node>,
}

/// A built document, ready to lay out.
pub struct Built {
    pub tree: BoxTree,
    pub root: NodeId,
    pub width: Px,
    pub height: Option<Px>,
    pub measure: Measure,
}

/// Parses a document and builds its box tree. Errors name the offending field or value.
pub fn build(json: &str) -> Result<Built, String> {
    let doc: Document = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut tree = BoxTree::new();
    let root = build_node(&mut tree, &doc.root)?;
    Ok(Built {
        tree,
        root,
        width: Px::from_f32(doc.width),
        height: doc.height.map(Px::from_f32),
        measure: doc.measure,
    })
}

fn build_node(tree: &mut BoxTree, node: &Node) -> Result<NodeId, String> {
    let style = style_of(node)?;
    let id = match &node.text {
        Some(text) => {
            if !node.children.is_empty() {
                return Err("a text node cannot also have children".into());
            }
            let text_style = BoxStyle {
                display: match &node.display {
                    Some(_) => style.display,
                    None => Display::Inline,
                },
                ..style
            };
            tree.insert_text(text_style, text.clone())
        }
        None => tree.insert(style),
    };
    for child in &node.children {
        let child_id = build_node(tree, child)?;
        tree.append_child(id, child_id).map_err(|e| format!("{e:?}"))?;
    }
    Ok(id)
}

fn style_of(node: &Node) -> Result<BoxStyle, String> {
    let mut style = BoxStyle::default();
    if let Some(name) = &node.display {
        style.display = match name.as_str() {
            "block" => Display::Block,
            "inline" => Display::Inline,
            "inline-block" => Display::InlineBlock,
            "flex" => Display::Flex,
            "grid" => Display::Grid,
            "table" => Display::Table,
            "table-row" => Display::TableRow,
            "table-cell" => Display::TableCell,
            "none" => Display::None,
            other => return Err(format!("unknown display \"{other}\"")),
        };
    }
    if let Some(size) = &node.width {
        style.width = dimension(size, "width")?;
    }
    if let Some(size) = &node.height {
        style.height = dimension(size, "height")?;
    }
    if let Some(size) = &node.min_width {
        style.min_width = dimension(size, "min_width")?;
    }
    if let Some(size) = &node.max_width {
        style.max_width = dimension(size, "max_width")?;
    }
    if let Some(size) = &node.min_height {
        style.min_height = dimension(size, "min_height")?;
    }
    if let Some(size) = &node.max_height {
        style.max_height = dimension(size, "max_height")?;
    }
    if let Some(px) = node.padding {
        let side = Dimension::Length(Px::from_f32(px));
        style.padding = Edges {
            top: side,
            right: side,
            bottom: side,
            left: side,
        };
    }
    if let Some(px) = node.margin {
        let side = Dimension::Length(Px::from_f32(px));
        style.margin = Edges {
            top: side,
            right: side,
            bottom: side,
            left: side,
        };
    }
    if let Some(px) = node.border {
        let side = Px::from_f32(px);
        style.border = Edges {
            top: side,
            right: side,
            bottom: side,
            left: side,
        };
    }
    if let Some(grow) = node.flex_grow {
        style.flex_grow = grow;
    }
    if let Some(name) = &node.flex_direction {
        style.flex_direction = match name.as_str() {
            "row" => FlexDirection::Row,
            "row-reverse" => FlexDirection::RowReverse,
            "column" => FlexDirection::Column,
            "column-reverse" => FlexDirection::ColumnReverse,
            other => return Err(format!("unknown flex_direction \"{other}\"")),
        };
    }
    if let Some(name) = &node.justify_content {
        style.justify_content = match name.as_str() {
            "flex-start" => JustifyContent::FlexStart,
            "flex-end" => JustifyContent::FlexEnd,
            "center" => JustifyContent::Center,
            "space-between" => JustifyContent::SpaceBetween,
            "space-around" => JustifyContent::SpaceAround,
            "space-evenly" => JustifyContent::SpaceEvenly,
            other => return Err(format!("unknown justify_content \"{other}\"")),
        };
    }
    if let Some(count) = node.column_count {
        style.column_count = count;
    }
    if let Some(px) = node.column_gap {
        style.column_gap = Dimension::Length(Px::from_f32(px));
    }
    if let Some(name) = &node.text_align {
        style.text_align = match name.as_str() {
            "start" => TextAlign::Start,
            "center" => TextAlign::Center,
            "end" => TextAlign::End,
            "justify" => TextAlign::Justify,
            other => return Err(format!("unknown text_align \"{other}\"")),
        };
    }
    if let Some(name) = &node.direction {
        style.direction = match name.as_str() {
            "ltr" => Direction::Ltr,
            "rtl" => Direction::Rtl,
            other => return Err(format!("unknown direction \"{other}\"")),
        };
    }
    if let Some(name) = &node.writing_mode {
        style.writing_mode = match name.as_str() {
            "horizontal-tb" => WritingMode::HorizontalTb,
            "vertical-rl" => WritingMode::VerticalRl,
            "vertical-lr" => WritingMode::VerticalLr,
            other => return Err(format!("unknown writing_mode \"{other}\"")),
        };
    }
    if let Some(name) = &node.overflow {
        style.overflow = match name.as_str() {
            "visible" => Overflow::Visible,
            "hidden" => Overflow::Hidden,
            "scroll" => Overflow::Scroll,
            other => return Err(format!("unknown overflow \"{other}\"")),
        };
    }
    if let Some(span) = node.column_span {
        style.column_span = span;
    }
    if let Some(span) = node.row_span {
        style.row_span = span;
    }
    Ok(style)
}

fn dimension(size: &Size, field: &str) -> Result<Dimension, String> {
    match size {
        Size::Px(px) => Ok(Dimension::Length(Px::from_f32(*px))),
        Size::Text(text) if text == "auto" => Ok(Dimension::Auto),
        Size::Text(text) => match text.strip_suffix('%').and_then(|n| n.trim().parse::<f32>().ok()) {
            Some(percent) => Ok(Dimension::percent(percent / 100.0)),
            None => Err(format!("{field}: expected a number, \"auto\" or a percentage, got \"{text}\"")),
        },
    }
}

/// Lays out a built document with its text metrics.
pub fn layout(built: &mut Built) -> Result<(), String> {
    let measurer = FixedAdvanceMeasurer::new(
        Px::from_f32(built.measure.advance),
        Px::from_f32(built.measure.line_height),
        Px::from_f32(built.measure.ascent),
    );
    let ctx = LayoutContext::new(&measurer);
    layout_document(
        &mut built.tree,
        built.root,
        LayoutConstraints {
            width: built.width,
            height: built.height,
        },
        &ctx,
    )
    .map(|_| ())
    .map_err(|e| format!("{e:?}"))
}

/// Parses, lays out and draws a document as SVG.
pub fn render_svg(json: &str) -> Result<String, String> {
    let mut built = build(json)?;
    layout(&mut built)?;
    let root = built.tree.get(built.root).ok_or("no root")?.layout;
    Ok(svg::render_svg(
        &built.tree,
        built.root,
        root.width,
        root.height,
    ))
}

/// Parses and lays out a document, and describes every box as JSON: its id, its depth, its text
/// (if any) and its border-box geometry.
pub fn layout_json(json: &str) -> Result<String, String> {
    let mut built = build(json)?;
    layout(&mut built)?;
    let mut boxes = Vec::new();
    collect(&built.tree, built.root, 0, &mut boxes);
    let value = json!({ "boxes": boxes });
    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
}

fn collect(tree: &BoxTree, id: NodeId, depth: usize, out: &mut Vec<Value>) {
    let Some(node) = tree.get(id) else {
        return;
    };
    let l = node.layout;
    out.push(json!({
        "id": id.index(),
        "depth": depth,
        "text": node.text,
        "x": l.x,
        "y": l.y,
        "width": l.width,
        "height": l.height,
    }));
    for child in tree.children(id) {
        collect(tree, *child, depth + 1, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_document_lays_out() {
        let json = r#"{"width": 200, "root": {"children": [{"height": 40}]}}"#;
        let out = layout_json(json).unwrap();
        assert!(out.contains("\"height\": 40.0"), "{out}");
    }

    #[test]
    fn unknown_fields_are_errors() {
        let json = r#"{"width": 200, "root": {"widht": 10}}"#;
        assert!(build(json).is_err());
    }

    #[test]
    fn unknown_enum_values_are_errors() {
        let json = r#"{"width": 200, "root": {"display": "florp"}}"#;
        let err = build(json).err().expect("error");
        assert!(err.contains("florp"), "{err}");
    }

    #[test]
    fn percentages_and_auto_parse() {
        let json = r#"{"width": 200, "root": {"width": "50%", "height": "auto"}}"#;
        let built = build(json).unwrap();
        let style = built.tree.get(built.root).unwrap().style;
        assert_eq!(style.width, Dimension::percent(0.5));
        assert_eq!(style.height, Dimension::Auto);
    }

    #[test]
    fn svg_output_contains_the_text() {
        let json = r#"{"width": 200, "root": {"children": [{"text": "hello"}]}}"#;
        let svg = render_svg(json).unwrap();
        assert!(svg.contains(">hello</text>"), "{svg}");
    }
}
