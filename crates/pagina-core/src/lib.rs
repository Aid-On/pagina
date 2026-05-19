pub mod css;
pub mod dom;
pub mod layout;
pub mod pdf;
pub mod style;

use css::PageStyleSet;

/// Convert HTML (with embedded CSS) to PDF bytes.
pub fn convert(html: &str) -> Vec<u8> {
    let dom = dom::parse_html(html);
    let styles = dom::extract_styles(&dom.document);

    // Parse CSS: page styles + regular rules
    let mut page_styles = PageStyleSet::default();
    let mut rules = Vec::new();
    for css_text in &styles {
        css::parser::parse_stylesheet(css_text, &mut page_styles, &mut rules);
    }

    // Build styled tree
    let styled_tree = style::build_styled_tree(&dom.document, &rules)
        .expect("failed to build styled tree");

    // Layout
    let pages = layout::lay_out(&page_styles, &styled_tree);

    // Render PDF
    pdf::render(&page_styles.base, &pages)
}
