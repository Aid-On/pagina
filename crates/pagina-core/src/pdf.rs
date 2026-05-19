use printpdf::{
    BuiltinFont, Line, LinePoint, Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions,
    Point, Pt, Rgb, TextItem,
};

use crate::css::values::{Color, FontStyle, FontWeight, MarginBoxPosition, TextAlign};
use crate::css::PageStyle;
use crate::layout::{ItemKind, Page, ResolvedMarginBox};

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

fn resolve_font(weight: FontWeight, style: FontStyle, family: &str) -> BuiltinFont {
    let is_courier = family.to_ascii_lowercase().contains("courier")
        || family.to_ascii_lowercase().contains("mono");

    match (is_courier, weight, style) {
        (true, FontWeight::Bold, FontStyle::Italic) => BuiltinFont::CourierBoldOblique,
        (true, FontWeight::Bold, _) => BuiltinFont::CourierBold,
        (true, _, FontStyle::Italic) => BuiltinFont::CourierOblique,
        (true, _, _) => BuiltinFont::Courier,
        (false, FontWeight::Bold, FontStyle::Italic) => BuiltinFont::HelveticaBoldOblique,
        (false, FontWeight::Bold, _) => BuiltinFont::HelveticaBold,
        (false, _, FontStyle::Italic) => BuiltinFont::HelveticaOblique,
        (false, _, _) => BuiltinFont::Helvetica,
    }
}

fn color_to_printpdf(c: &Color) -> printpdf::Color {
    printpdf::Color::Rgb(Rgb {
        r: c.r as f32 / 255.0,
        g: c.g as f32 / 255.0,
        b: c.b as f32 / 255.0,
        icc_profile: None,
    })
}

fn build_page_ops(style: &PageStyle, page: &Page) -> Vec<Op> {
    let mut ops = Vec::new();
    render_items(&mut ops, style, &page.items);
    render_items(&mut ops, style, &page.footnotes);
    render_margin_boxes(&mut ops, style, &page.margin_boxes);
    ops
}

fn render_items(ops: &mut Vec<Op>, style: &PageStyle, items: &[crate::layout::LayoutItem]) {
    for item in items {
        match &item.kind {
            ItemKind::Text => render_text_item(ops, style, item),
            ItemKind::HorizontalRule { width_mm, thickness_mm, color } => {
                render_hr(ops, style, item, *width_mm, *thickness_mm, color);
            }
            ItemKind::FootnoteMarker(_) | ItemKind::FootnoteRef(_) => {
                render_text_item(ops, style, item);
            }
        }
    }
}

fn render_text_item(ops: &mut Vec<Op>, style: &PageStyle, item: &crate::layout::LayoutItem) {
    let x = (style.margin_left_mm + item.x_mm) as f32;
    let y = (style.height_mm - style.margin_top_mm - item.y_mm
        - item.font_size_pt * 25.4 / 72.0) as f32;

    let font = resolve_font(item.font_weight, item.font_style, &item.font_family);

    ops.push(Op::StartTextSection);
    ops.push(Op::SetFillColor {
        col: color_to_printpdf(&item.color),
    });
    ops.push(Op::SetFont {
        font: PdfFontHandle::Builtin(font),
        size: Pt(item.font_size_pt as f32),
    });
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(x), Mm(y)),
    });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(item.text.clone())],
    });
    ops.push(Op::EndTextSection);
}

fn render_hr(
    ops: &mut Vec<Op>,
    style: &PageStyle,
    item: &crate::layout::LayoutItem,
    width_mm: f64,
    thickness_mm: f64,
    color: &Color,
) {
    let x_start = style.margin_left_mm + item.x_mm;
    let y = style.height_mm - style.margin_top_mm - item.y_mm;

    ops.push(Op::SaveGraphicsState);
    ops.push(Op::SetOutlineColor {
        col: color_to_printpdf(color),
    });
    ops.push(Op::SetOutlineThickness {
        pt: Pt(thickness_mm as f32 * 72.0 / 25.4),
    });
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![
                LinePoint { p: Point::new(Mm(x_start as f32), Mm(y as f32)), bezier: false },
                LinePoint { p: Point::new(Mm((x_start + width_mm) as f32), Mm(y as f32)), bezier: false },
            ],
            is_closed: false,
        },
    });
    ops.push(Op::RestoreGraphicsState);
}

fn render_margin_boxes(ops: &mut Vec<Op>, style: &PageStyle, boxes: &[ResolvedMarginBox]) {
    for mb in boxes {
        let text_width_mm = mb.text.len() as f64 * mb.font_size_pt * 0.5 * 25.4 / 72.0;
        let area_width = margin_box_area_width(style, &mb.position);
        let area_x = margin_box_area_x(style, &mb.position);

        let x = match mb.text_align {
            TextAlign::Center => area_x + (area_width - text_width_mm).max(0.0) / 2.0,
            TextAlign::Right => area_x + (area_width - text_width_mm).max(0.0),
            _ => area_x,
        };

        let font_height_mm = mb.font_size_pt * 25.4 / 72.0;
        let y = if mb.position.is_top() {
            style.height_mm - style.margin_top_mm / 2.0 - font_height_mm / 2.0
        } else if mb.position.is_bottom() {
            style.margin_bottom_mm / 2.0 - font_height_mm / 2.0
        } else {
            style.height_mm / 2.0
        };

        ops.push(Op::StartTextSection);
        ops.push(Op::SetFillColor {
            col: color_to_printpdf(&mb.color),
        });
        ops.push(Op::SetFont {
            font: PdfFontHandle::Builtin(BuiltinFont::Helvetica),
            size: Pt(mb.font_size_pt as f32),
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(x as f32), Mm(y as f32)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(mb.text.clone())],
        });
        ops.push(Op::EndTextSection);
    }
}

fn margin_box_area_x(style: &PageStyle, pos: &MarginBoxPosition) -> f64 {
    match pos {
        MarginBoxPosition::LeftTop | MarginBoxPosition::LeftMiddle | MarginBoxPosition::LeftBottom => 0.0,
        MarginBoxPosition::RightTop | MarginBoxPosition::RightMiddle | MarginBoxPosition::RightBottom => {
            style.width_mm - style.margin_right_mm
        }
        _ => style.margin_left_mm, // top/bottom boxes span the content area
    }
}

fn margin_box_area_width(style: &PageStyle, pos: &MarginBoxPosition) -> f64 {
    match pos {
        MarginBoxPosition::TopLeft | MarginBoxPosition::BottomLeft => style.content_width_mm() / 3.0,
        MarginBoxPosition::TopCenter | MarginBoxPosition::BottomCenter => style.content_width_mm(),
        MarginBoxPosition::TopRight | MarginBoxPosition::BottomRight => style.content_width_mm() / 3.0,
        MarginBoxPosition::LeftTop | MarginBoxPosition::LeftMiddle | MarginBoxPosition::LeftBottom => {
            style.margin_left_mm
        }
        MarginBoxPosition::RightTop | MarginBoxPosition::RightMiddle | MarginBoxPosition::RightBottom => {
            style.margin_right_mm
        }
        _ => style.content_width_mm(),
    }
}
