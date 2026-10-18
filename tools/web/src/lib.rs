//! WebAssembly bindings for the layout playground. The functions take a JSON document (see the
//! `tpt-layout-spec` crate) and return SVG or layout JSON.

use wasm_bindgen::prelude::*;

/// Lays out `json` and returns it as SVG. A document that fails to parse or lay out returns an SVG
/// that shows the error, so the playground always has something to display.
#[wasm_bindgen]
pub fn render_svg(json: &str) -> String {
    match tpt_layout_spec::render_svg(json) {
        Ok(svg) => svg,
        Err(message) => error_svg(&message),
    }
}

/// Lays out `json` and returns every box's geometry as JSON. Errors are thrown to JavaScript.
#[wasm_bindgen]
pub fn layout_json(json: &str) -> Result<String, JsValue> {
    tpt_layout_spec::layout_json(json).map_err(|message| JsValue::from_str(&message))
}

fn error_svg(message: &str) -> String {
    let escaped = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"600\" height=\"80\">\
         <rect width=\"100%\" height=\"100%\" fill=\"#fff0f0\"/>\
         <text x=\"12\" y=\"30\" font-family=\"sans-serif\" font-size=\"14\" fill=\"#a00\">{escaped}</text>\
         </svg>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_documents_render() {
        let svg = render_svg(r#"{"width": 100, "root": {"children": [{"text": "hi"}]}}"#);
        assert!(svg.contains(">hi</text>"), "{svg}");
    }

    #[test]
    fn errors_are_shown_as_escaped_svg() {
        let svg = render_svg(r#"{"width": 100, "root": {"display": "<b>"}}"#);
        assert!(svg.contains("&lt;b&gt;"), "{svg}");
        assert!(!svg.contains("<b>"), "{svg}");
    }
}
