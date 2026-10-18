//! Paginates the report template onto small pages and prints where each block's lines fall.
//!
//! Usage: `cargo run --example report`

use tpt_layout::{templates, PageConstraints, Px};

fn main() {
    let text =
        "Layout engines split paragraphs between pages, keeping at least two lines at the top \
                and bottom of each page so that a single line never sits alone.";
    let report = templates::report("Quarterly report", &[text, text]);
    // Pages of 60 px: three 16 px lines each.
    let page = PageConstraints::new(Px::from_px(400), Px::from_px(60));
    let layout = report.paginate(&page);
    println!("{} pages", layout.pages.len());
    for fragment in &layout.fragments {
        if let Some(lines) = &fragment.lines {
            println!(
                "page {}: node {} lines {}..{}",
                fragment.page,
                fragment.node.index(),
                lines.start,
                lines.end
            );
        }
    }
}
