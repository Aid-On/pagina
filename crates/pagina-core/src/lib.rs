pub mod css;
pub mod dom;
pub mod layout;
pub mod pdf;

/// Convert HTML (with embedded CSS) to PDF bytes.
pub fn convert(html: &str) -> Vec<u8> {
    let dom = dom::parse_html(html);
    let styles = dom::extract_styles(&dom.document);
    let blocks = dom::extract_text_blocks(&dom.document);

    let mut page_style = css::PageStyle::default();
    for css_text in &styles {
        css::parser::apply_page_rules(css_text, &mut page_style);
    }

    let pages = layout::lay_out(&page_style, &blocks);
    pdf::render(&page_style, &pages)
}
