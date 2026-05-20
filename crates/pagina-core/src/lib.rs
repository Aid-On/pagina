pub mod css;
pub mod dom;
pub mod font;
pub mod layout;
pub mod pdf;
pub mod style;

use css::PageStyleSet;
use font::FontManager;

/// Convert HTML (with embedded CSS) to PDF bytes.
pub fn convert(html: &str) -> Vec<u8> {
    convert_with_fonts(html, &[])
}

/// Convert HTML to PDF, optionally loading external font files.
pub fn convert_with_fonts(html: &str, font_paths: &[&str]) -> Vec<u8> {
    let mut fm = FontManager::new();

    // Load external fonts
    for path in font_paths {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("warning: failed to load font {path}: {e}");
                continue;
            }
        };
        // Derive family name from filename
        let family = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("CustomFont")
            .to_string();
        if !fm.load_font(bytes, &family) {
            eprintln!("warning: failed to parse font {path}");
        }
    }

    let dom = dom::parse_html(html);
    let styles = dom::extract_styles(&dom.document);

    let mut page_styles = PageStyleSet::default();
    let mut rules = Vec::new();
    for css_text in &styles {
        css::parser::parse_stylesheet(css_text, &mut page_styles, &mut rules);
    }

    let styled_tree = style::build_styled_tree(&dom.document, &rules)
        .expect("failed to build styled tree");

    let (pages, images) = layout::lay_out(&page_styles, &styled_tree, &fm);

    pdf::render(&page_styles.base, &pages, &images, &mut fm)
}
