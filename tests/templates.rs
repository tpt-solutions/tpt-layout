//! Starter templates: each renders to an SVG that is pinned by a golden file.
//! Regenerate with `UPDATE_GOLDEN=1 cargo test --test templates`.

use std::path::PathBuf;

use tpt_layout::templates;

fn check_golden(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/templates")
        .join(format!("{name}.svg"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing golden file {}; run with UPDATE_GOLDEN=1",
            path.display()
        )
    });
    let normalize = |s: &str| s.replace("\r\n", "\n");
    assert_eq!(
        normalize(actual),
        normalize(&expected),
        "golden mismatch for {name}"
    );
}

#[test]
fn invoice_matches_golden() {
    let t = templates::invoice(
        "Acme Ltd",
        &[("Widgets", 3, 12), ("Gears", 2, 7), ("Bolts", 10, 1)],
    );
    let svg = t.to_svg();
    // Each word is its own text element. The grand total is 3*12 + 2*7 + 10*1 = 60.
    assert!(svg.contains(">60</text>"), "{svg}");
    assert!(svg.contains(">Widgets</text>"), "{svg}");
    check_golden("invoice", &svg);
}

#[test]
fn dashboard_matches_golden() {
    let t = templates::dashboard(&[
        ("Users", "1,204"),
        ("Revenue", "$9.8k"),
        ("Churn", "2.1%"),
        ("Tickets", "37"),
        ("Uptime", "99.9%"),
        ("Latency", "120 ms"),
    ]);
    check_golden("dashboard", &t.to_svg());
}

#[test]
fn navbar_matches_golden() {
    let t = templates::navbar("Brand", &["Docs", "Pricing", "Login"]);
    check_golden("navbar", &t.to_svg());
}

#[test]
fn resume_matches_golden() {
    let t = templates::resume(
        "Ada Lovelace",
        &["Email: ada@example.com", "Skills: maths", "Languages: EN"],
        &["Analyst at Babbage Co.", "Wrote the first algorithm."],
    );
    check_golden("resume", &t.to_svg());
}

#[test]
fn rtl_page_matches_golden() {
    let t = templates::rtl_page(&["مرحبا بالعالم", "Second line"]);
    check_golden("rtl_page", &t.to_svg());
}

#[test]
fn vertical_page_matches_golden() {
    let t = templates::vertical_page(&["縦書き", "テキスト"]);
    check_golden("vertical_page", &t.to_svg());
}

#[test]
fn newsletter_matches_golden() {
    let t = templates::newsletter(&[
        "First story about layout engines.",
        "Second story, a little longer than the first one.",
        "Third story wraps onto its own lines too.",
    ]);
    check_golden("newsletter", &t.to_svg());
}

#[test]
fn rtl_lines_end_at_the_right_edge() {
    // "Second line" is 88 px at 8 px per character, so in a 400 px page its last character ends at
    // 400: the words start at 400 - 88 = 312.
    let svg = templates::rtl_page(&["Second line"]).to_svg();
    assert!(svg.contains("<text x=\"312\" y=\"16\""), "{svg}");
}

#[test]
fn vertical_characters_stack_down_one_column() {
    let svg = templates::vertical_page(&["縦書き"]).to_svg();
    // The three characters share x and step down by one 8 px cell each.
    assert!(svg.contains("<text x=\"304\" y=\"16\""), "{svg}");
    assert!(svg.contains("<text x=\"304\" y=\"24\""), "{svg}");
    assert!(svg.contains("<text x=\"304\" y=\"32\""), "{svg}");
}
