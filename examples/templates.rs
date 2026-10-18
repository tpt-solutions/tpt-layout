//! Writes every starter template to an output directory as SVG.
//!
//! Usage: `cargo run --example templates -- <output-dir>` (defaults to `templates-out`).

use std::path::PathBuf;

use tpt_layout::templates;

fn main() -> std::io::Result<()> {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "templates-out".into()),
    );
    std::fs::create_dir_all(&dir)?;
    let outputs = [
        (
            "invoice",
            templates::invoice("Acme Ltd", &[("Widgets", 3, 12), ("Gears", 2, 7)]),
        ),
        (
            "dashboard",
            templates::dashboard(&[("Users", "1,204"), ("Revenue", "$9.8k"), ("Churn", "2.1%")]),
        ),
        (
            "navbar",
            templates::navbar("Brand", &["Docs", "Pricing", "Login"]),
        ),
        (
            "resume",
            templates::resume(
                "Ada Lovelace",
                &["Email: ada@example.com", "Skills: maths"],
                &["Analyst at Babbage Co."],
            ),
        ),
    ];
    for (name, template) in outputs {
        let path = dir.join(format!("{name}.svg"));
        std::fs::write(&path, template.to_svg())?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
