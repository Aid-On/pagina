/// MathML rendering to layout items.
///
/// Supports basic MathML elements:
/// - <math> container
/// - <mrow> horizontal grouping
/// - <mi> identifier (variable)
/// - <mn> number
/// - <mo> operator
/// - <mfrac> fraction (numerator/denominator)
/// - <msup> superscript
/// - <msub> subscript
/// - <msqrt> square root

use crate::css::values::*;
use crate::font::FontManager;
use crate::layout::LayoutItem;
use crate::layout::ItemKind;
use crate::style::{StyledContent, StyledNode};

/// Render a <math> element into positioned layout items.
pub fn render_math(
    node: &StyledNode,
    base_font_size: f64,
    fm: &FontManager,
) -> Vec<LayoutItem> {
    let mut items = Vec::new();
    let mut x = 0.0;
    let style = MathStyle {
        font_size: base_font_size,
        color: node.style.color,
        font_family: node.style.font_family.clone(),
    };
    render_math_node(node, &mut items, &mut x, 0.0, &style, fm);
    items
}

/// Width of a <math> element in mm.
pub fn math_width(node: &StyledNode, base_font_size: f64, fm: &FontManager) -> f64 {
    let items = render_math(node, base_font_size, fm);
    items
        .iter()
        .map(|item| item.x_mm + measure_item_width(item, fm))
        .fold(0.0_f64, f64::max)
}

struct MathStyle {
    font_size: f64,
    color: Color,
    font_family: String,
}

fn render_math_node(
    node: &StyledNode,
    items: &mut Vec<LayoutItem>,
    x: &mut f64,
    y_offset: f64,
    style: &MathStyle,
    fm: &FontManager,
) {
    match node.tag.as_str() {
        "math" | "mrow" => {
            for child in &node.children {
                match child {
                    StyledContent::Element(child_node) => {
                        render_math_node(child_node, items, x, y_offset, style, fm);
                    }
                    StyledContent::Text(text) => {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            emit_math_text(items, x, y_offset, trimmed, style, fm);
                        }
                    }
                }
            }
        }
        "mi" => {
            // Identifier: render in italic
            let text = collect_math_text(node);
            let italic_style = MathStyle {
                font_size: style.font_size,
                color: style.color,
                font_family: style.font_family.clone(),
            };
            items.push(LayoutItem {
                x_mm: *x,
                y_mm: y_offset,
                font_size_pt: italic_style.font_size,
                font_weight: FontWeight::Normal,
                font_style: FontStyle::Italic,
                font_family: italic_style.font_family.clone(),
                color: italic_style.color,
                text: text.clone(),
                kind: ItemKind::Text,
            });
            let w = fm.measure_text(&text, &style.font_family, FontWeight::Normal, FontStyle::Italic, style.font_size);
            *x += w;
        }
        "mn" | "mo" => {
            let text = collect_math_text(node);
            emit_math_text(items, x, y_offset, &text, style, fm);
        }
        "mfrac" => {
            // Fraction: numerator over denominator with a line
            let children: Vec<&StyledNode> = node.children.iter().filter_map(|c| {
                if let StyledContent::Element(n) = c { Some(n) } else { None }
            }).collect();

            if children.len() >= 2 {
                let small_style = MathStyle {
                    font_size: style.font_size * 0.75,
                    color: style.color,
                    font_family: style.font_family.clone(),
                };

                // Measure numerator and denominator
                let num_text = collect_all_text(children[0]);
                let den_text = collect_all_text(children[1]);
                let num_w = fm.measure_text(&num_text, &style.font_family, FontWeight::Normal, FontStyle::Normal, small_style.font_size);
                let den_w = fm.measure_text(&den_text, &style.font_family, FontWeight::Normal, FontStyle::Normal, small_style.font_size);
                let frac_w = num_w.max(den_w) + 1.0; // padding

                let frac_x = *x;
                let lh = style.font_size * 25.4 / 72.0;

                // Numerator (above the line)
                let num_x = frac_x + (frac_w - num_w) / 2.0;
                items.push(LayoutItem {
                    x_mm: num_x, y_mm: y_offset - lh * 0.4,
                    font_size_pt: small_style.font_size,
                    font_weight: FontWeight::Normal, font_style: FontStyle::Normal,
                    font_family: small_style.font_family.clone(),
                    color: small_style.color, text: num_text,
                    kind: ItemKind::Text,
                });

                // Fraction line
                items.push(LayoutItem {
                    x_mm: frac_x, y_mm: y_offset,
                    font_size_pt: 0.0, font_weight: FontWeight::Normal,
                    font_style: FontStyle::Normal, font_family: String::new(),
                    color: style.color, text: String::new(),
                    kind: ItemKind::HorizontalRule {
                        width_mm: frac_w,
                        thickness_mm: 0.15,
                        color: style.color,
                    },
                });

                // Denominator (below the line)
                let den_x = frac_x + (frac_w - den_w) / 2.0;
                items.push(LayoutItem {
                    x_mm: den_x, y_mm: y_offset + lh * 0.45,
                    font_size_pt: small_style.font_size,
                    font_weight: FontWeight::Normal, font_style: FontStyle::Normal,
                    font_family: small_style.font_family.clone(),
                    color: small_style.color, text: den_text,
                    kind: ItemKind::Text,
                });

                *x += frac_w + 0.5;
            }
        }
        "msup" => {
            // Base + superscript
            let children: Vec<&StyledNode> = node.children.iter().filter_map(|c| {
                if let StyledContent::Element(n) = c { Some(n) } else { None }
            }).collect();

            if children.len() >= 2 {
                // Base
                render_math_node(children[0], items, x, y_offset, style, fm);
                // Superscript (smaller, raised)
                let sup_style = MathStyle {
                    font_size: style.font_size * 0.7,
                    color: style.color,
                    font_family: style.font_family.clone(),
                };
                let lh = style.font_size * 25.4 / 72.0;
                render_math_node(children[1], items, x, y_offset - lh * 0.35, &sup_style, fm);
            }
        }
        "msub" => {
            let children: Vec<&StyledNode> = node.children.iter().filter_map(|c| {
                if let StyledContent::Element(n) = c { Some(n) } else { None }
            }).collect();

            if children.len() >= 2 {
                render_math_node(children[0], items, x, y_offset, style, fm);
                let sub_style = MathStyle {
                    font_size: style.font_size * 0.7,
                    color: style.color,
                    font_family: style.font_family.clone(),
                };
                let lh = style.font_size * 25.4 / 72.0;
                render_math_node(children[1], items, x, y_offset + lh * 0.25, &sub_style, fm);
            }
        }
        "msqrt" => {
            // Square root: radical sign + content
            emit_math_text(items, x, y_offset, "V/", style, fm); // simplified radical
            // Overline over the content
            let content_start = *x;
            for child in &node.children {
                match child {
                    StyledContent::Element(n) => render_math_node(n, items, x, y_offset, style, fm),
                    StyledContent::Text(t) => {
                        let trimmed = t.trim();
                        if !trimmed.is_empty() {
                            emit_math_text(items, x, y_offset, trimmed, style, fm);
                        }
                    }
                }
            }
            let content_end = *x;
            // Top bar
            let lh = style.font_size * 25.4 / 72.0;
            items.push(LayoutItem {
                x_mm: content_start, y_mm: y_offset - lh * 0.5,
                font_size_pt: 0.0, font_weight: FontWeight::Normal,
                font_style: FontStyle::Normal, font_family: String::new(),
                color: style.color, text: String::new(),
                kind: ItemKind::HorizontalRule {
                    width_mm: content_end - content_start,
                    thickness_mm: 0.15,
                    color: style.color,
                },
            });
        }
        _ => {
            // Unknown element: render children
            for child in &node.children {
                match child {
                    StyledContent::Element(n) => render_math_node(n, items, x, y_offset, style, fm),
                    StyledContent::Text(t) => {
                        let trimmed = t.trim();
                        if !trimmed.is_empty() {
                            emit_math_text(items, x, y_offset, trimmed, style, fm);
                        }
                    }
                }
            }
        }
    }
}

fn emit_math_text(
    items: &mut Vec<LayoutItem>,
    x: &mut f64,
    y_offset: f64,
    text: &str,
    style: &MathStyle,
    fm: &FontManager,
) {
    let w = fm.measure_text(text, &style.font_family, FontWeight::Normal, FontStyle::Normal, style.font_size);
    items.push(LayoutItem {
        x_mm: *x,
        y_mm: y_offset,
        font_size_pt: style.font_size,
        font_weight: FontWeight::Normal,
        font_style: FontStyle::Normal,
        font_family: style.font_family.clone(),
        color: style.color,
        text: text.to_string(),
        kind: ItemKind::Text,
    });
    *x += w;
}

fn collect_math_text(node: &StyledNode) -> String {
    let mut s = String::new();
    for child in &node.children {
        if let StyledContent::Text(t) = child {
            s.push_str(t.trim());
        }
    }
    s
}

fn collect_all_text(node: &StyledNode) -> String {
    let mut s = String::new();
    for child in &node.children {
        match child {
            StyledContent::Text(t) => s.push_str(t.trim()),
            StyledContent::Element(n) => s.push_str(&collect_all_text(n)),
        }
    }
    s
}

fn measure_item_width(item: &LayoutItem, fm: &FontManager) -> f64 {
    if item.text.is_empty() {
        if let ItemKind::HorizontalRule { width_mm, .. } = &item.kind {
            return *width_mm;
        }
        return 0.0;
    }
    fm.measure_text(&item.text, &item.font_family, item.font_weight, item.font_style, item.font_size_pt)
}
