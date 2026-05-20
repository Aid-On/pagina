pub mod css;
pub mod dom;
pub mod font;
pub mod js;
pub mod layout;
pub mod mathml;
pub mod pdf;
pub mod pdfa;
pub mod pdfua;
pub mod style;
pub mod svg;

use css::PageStyleSet;
use font::FontManager;

/// Conversion options.
#[derive(Default)]
pub struct ConvertOptions<'a> {
    pub font_paths: &'a [&'a str],
    pub pdfa: Option<pdfa::PdfAOptions>,
    pub tagged: bool,
}

/// Convert HTML (with embedded CSS) to PDF bytes.
pub fn convert(html: &str) -> Vec<u8> {
    convert_with_options(html, &ConvertOptions::default())
}

/// Convert HTML to PDF with full options.
pub fn convert_with_options(html: &str, opts: &ConvertOptions) -> Vec<u8> {
    let mut fm = FontManager::new();

    for path in opts.font_paths {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("warning: failed to load font {path}: {e}");
                continue;
            }
        };
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
    let js_writes = js::run_scripts(&dom);

    let effective_dom = if js_writes.is_empty() {
        dom
    } else {
        let extra_html = js_writes.join("");
        let mut modified = html.to_string();
        if let Some(pos) = modified.rfind("</body>") {
            modified.insert_str(pos, &extra_html);
        } else {
            modified.push_str(&extra_html);
        }
        dom::parse_html(&modified)
    };

    let styles = dom::extract_styles(&effective_dom.document);

    let mut page_styles = PageStyleSet::default();
    let mut rules = Vec::new();
    for css_text in &styles {
        css::parser::parse_stylesheet(css_text, &mut page_styles, &mut rules);
    }

    let styled_tree = style::build_styled_tree(&effective_dom.document, &rules)
        .expect("failed to build styled tree");

    let (pages, images) = layout::lay_out(&page_styles, &styled_tree, &fm);
    let mut pdf_bytes = pdf::render(&page_styles.base, &pages, &images, &mut fm);

    // PDF/A post-processing
    if let Some(pdfa_opts) = &opts.pdfa {
        pdf_bytes = pdfa::make_pdfa(&pdf_bytes, pdfa_opts);
    }

    // PDF/UA tagged structure
    if opts.tagged {
        let structure = pdfua::build_structure(&styled_tree);
        pdf_bytes = pdfua::make_tagged_pdf(&pdf_bytes, &structure);
    }

    pdf_bytes
}

/// Convenience: convert with font paths only.
pub fn convert_with_fonts(html: &str, font_paths: &[&str]) -> Vec<u8> {
    convert_with_options(
        html,
        &ConvertOptions { font_paths, ..Default::default() },
    )
}
