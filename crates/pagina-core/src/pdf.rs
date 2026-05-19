use printpdf::*;

use crate::css::PageStyle;
use crate::layout::Page;

/// Render laid-out pages to PDF bytes.
pub fn render(style: &PageStyle, pages: &[Page]) -> Vec<u8> {
    let w = Mm(style.width_mm as f32);
    let h = Mm(style.height_mm as f32);

    let mut doc = PdfDocument::new("pagina output");

    let pdf_pages: Vec<PdfPage> = pages
        .iter()
        .map(|page| {
            let ops = build_page_ops(style, page);
            PdfPage::new(w, h, ops)
        })
        .collect();

    doc.with_pages(pdf_pages);

    let mut warnings = Vec::new();
    doc.save(&PdfSaveOptions::default(), &mut warnings)
}

fn build_page_ops(style: &PageStyle, page: &Page) -> Vec<Op> {
    let font = PdfFontHandle::Builtin(BuiltinFont::Helvetica);
    let mut ops = Vec::new();

    for item in &page.items {
        let fs = item.font_size_pt as f32;

        // printpdf Y is bottom-up; our layout Y grows downward from top margin.
        let x = (style.margin_left_mm + item.x_mm) as f32;
        let y = (style.height_mm - style.margin_top_mm - item.y_mm
            - item.font_size_pt * 25.4 / 72.0) as f32;

        // Each item gets its own BT/ET so SetTextCursor is always absolute (from origin).
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFont {
            font: font.clone(),
            size: Pt(fs),
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(x), Mm(y)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(item.text.clone())],
        });
        ops.push(Op::EndTextSection);
    }

    ops
}
