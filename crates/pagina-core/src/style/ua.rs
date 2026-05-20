//! User-agent default styles for HTML elements.

use crate::css::values::*;
use super::ComputedStyle;

/// UA defaults for common HTML tags.
pub(super) fn ua_style(tag: &str) -> ComputedStyle {
    let mut s = ComputedStyle::default();
    apply_ua_defaults(&mut s, tag);
    s
}

fn apply_ua_defaults(s: &mut ComputedStyle, tag: &str) {
    if !apply_ua_heading(s, tag) && !apply_ua_inline(s, tag) {
        apply_ua_block(s, tag);
    }
}

fn apply_ua_heading(s: &mut ComputedStyle, tag: &str) -> bool {
    let (size, mt, mb) = match tag {
        "h1" => (26.0, 6.0, 4.0),
        "h2" => (20.0, 5.0, 3.0),
        "h3" => (16.0, 4.0, 2.5),
        "h4" => (14.0, 3.0, 2.0),
        "h5" | "h6" => (12.0, 2.5, 1.5),
        _ => return false,
    };
    s.font_size_pt = size;
    s.font_weight = FontWeight::Bold;
    s.margin_top_mm = mt;
    s.margin_bottom_mm = mb;
    true
}

fn apply_ua_inline(s: &mut ComputedStyle, tag: &str) -> bool {
    match tag {
        "code" | "kbd" | "samp" => { s.font_family = "Courier".to_string(); s.display = Display::Inline; }
        "strong" | "b" => { s.font_weight = FontWeight::Bold; s.display = Display::Inline; }
        "em" | "i" => { s.font_style = FontStyle::Italic; s.display = Display::Inline; }
        "span" | "a" | "abbr" | "small" | "sub" | "sup" => { s.display = Display::Inline; }
        _ => return false,
    }
    true
}

fn apply_ua_block(s: &mut ComputedStyle, tag: &str) {
    match tag {
        "li" => apply_ua_li(s),
        "hr" => apply_ua_hr(s),
        "th" => apply_ua_th(s),
        _ => apply_ua_block_from_table(s, tag),
    }
}

fn apply_ua_li(s: &mut ComputedStyle) {
    s.display = Display::ListItem;
    s.margin_bottom_mm = 1.5;
}

fn apply_ua_hr(s: &mut ComputedStyle) {
    s.margin_top_mm = 4.0;
    s.margin_bottom_mm = 4.0;
    s.border_bottom_width_mm = 0.2;
}

fn apply_ua_th(s: &mut ComputedStyle) {
    s.padding_top_mm = 1.0;
    s.padding_bottom_mm = 1.0;
    s.font_weight = FontWeight::Bold;
}

/// Table-driven block defaults: (tag, margin_top, margin_bottom, padding_top, padding_bottom, font_family, font_size).
/// Zero values mean "keep default".
const BLOCK_DEFAULTS: &[(&str, f64, f64, f64, f64, &str, f64)] = &[
    ("p",          0.0, 3.5, 0.0, 0.0, "",        0.0),
    ("blockquote", 3.0, 3.0, 0.0, 0.0, "",        0.0),
    ("pre",        2.0, 2.0, 0.0, 0.0, "Courier", 10.0),
    ("table",      3.0, 3.0, 0.0, 0.0, "",        0.0),
    ("td",         0.0, 0.0, 1.0, 1.0, "",        0.0),
];

fn apply_ua_block_from_table(s: &mut ComputedStyle, tag: &str) {
    let Some(entry) = BLOCK_DEFAULTS.iter().find(|(t, ..)| *t == tag) else { return };
    apply_block_entry(s, entry);
}

fn apply_block_entry(s: &mut ComputedStyle, e: &(&str, f64, f64, f64, f64, &str, f64)) {
    if e.1 != 0.0 { s.margin_top_mm = e.1; }
    if e.2 != 0.0 { s.margin_bottom_mm = e.2; }
    if e.3 != 0.0 { s.padding_top_mm = e.3; }
    if e.4 != 0.0 { s.padding_bottom_mm = e.4; }
    if !e.5.is_empty() { s.font_family = e.5.to_string(); }
    if e.6 != 0.0 { s.font_size_pt = e.6; }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Headings ────────────────────────────────────────────────

    #[test]
    fn h1_font_size_and_weight() {
        let s = ua_style("h1");
        assert!((s.font_size_pt - 26.0).abs() < 1e-9);
        assert_eq!(s.font_weight, FontWeight::Bold);
    }

    #[test]
    fn h1_margins() {
        let s = ua_style("h1");
        assert!((s.margin_top_mm - 6.0).abs() < 1e-9);
        assert!((s.margin_bottom_mm - 4.0).abs() < 1e-9);
    }

    #[test]
    fn h2_font_size_and_weight() {
        let s = ua_style("h2");
        assert!((s.font_size_pt - 20.0).abs() < 1e-9);
        assert_eq!(s.font_weight, FontWeight::Bold);
    }

    #[test]
    fn h2_margins() {
        let s = ua_style("h2");
        assert!((s.margin_top_mm - 5.0).abs() < 1e-9);
        assert!((s.margin_bottom_mm - 3.0).abs() < 1e-9);
    }

    #[test]
    fn h3_font_size() {
        let s = ua_style("h3");
        assert!((s.font_size_pt - 16.0).abs() < 1e-9);
        assert_eq!(s.font_weight, FontWeight::Bold);
    }

    #[test]
    fn h4_font_size() {
        let s = ua_style("h4");
        assert!((s.font_size_pt - 14.0).abs() < 1e-9);
    }

    #[test]
    fn h5_h6_same_size() {
        let h5 = ua_style("h5");
        let h6 = ua_style("h6");
        assert!((h5.font_size_pt - 12.0).abs() < 1e-9);
        assert!((h6.font_size_pt - 12.0).abs() < 1e-9);
    }

    // ── Block elements ──────────────────────────────────────────

    #[test]
    fn p_margins() {
        let s = ua_style("p");
        assert!((s.margin_top_mm).abs() < 1e-9);
        assert!((s.margin_bottom_mm - 3.5).abs() < 1e-9);
    }

    #[test]
    fn p_default_display_block() {
        let s = ua_style("p");
        assert_eq!(s.display, Display::Block);
    }

    #[test]
    fn blockquote_margins() {
        let s = ua_style("blockquote");
        assert!((s.margin_top_mm - 3.0).abs() < 1e-9);
        assert!((s.margin_bottom_mm - 3.0).abs() < 1e-9);
    }

    #[test]
    fn pre_font_family_courier() {
        let s = ua_style("pre");
        assert_eq!(s.font_family, "Courier");
    }

    #[test]
    fn pre_font_size() {
        let s = ua_style("pre");
        assert!((s.font_size_pt - 10.0).abs() < 1e-9);
    }

    #[test]
    fn table_margins() {
        let s = ua_style("table");
        assert!((s.margin_top_mm - 3.0).abs() < 1e-9);
        assert!((s.margin_bottom_mm - 3.0).abs() < 1e-9);
    }

    #[test]
    fn td_padding() {
        let s = ua_style("td");
        assert!((s.padding_top_mm - 1.0).abs() < 1e-9);
        assert!((s.padding_bottom_mm - 1.0).abs() < 1e-9);
    }

    // ── Inline elements ─────────────────────────────────────────

    #[test]
    fn code_inline_courier() {
        let s = ua_style("code");
        assert_eq!(s.display, Display::Inline);
        assert_eq!(s.font_family, "Courier");
    }

    #[test]
    fn strong_bold_inline() {
        let s = ua_style("strong");
        assert_eq!(s.display, Display::Inline);
        assert_eq!(s.font_weight, FontWeight::Bold);
    }

    #[test]
    fn b_bold_inline() {
        let s = ua_style("b");
        assert_eq!(s.display, Display::Inline);
        assert_eq!(s.font_weight, FontWeight::Bold);
    }

    #[test]
    fn em_italic_inline() {
        let s = ua_style("em");
        assert_eq!(s.display, Display::Inline);
        assert_eq!(s.font_style, FontStyle::Italic);
    }

    #[test]
    fn i_italic_inline() {
        let s = ua_style("i");
        assert_eq!(s.display, Display::Inline);
        assert_eq!(s.font_style, FontStyle::Italic);
    }

    #[test]
    fn span_inline_no_special() {
        let s = ua_style("span");
        assert_eq!(s.display, Display::Inline);
        assert_eq!(s.font_weight, FontWeight::Normal);
        assert_eq!(s.font_style, FontStyle::Normal);
    }

    // ── Special elements ────────────────────────────────────────

    #[test]
    fn li_list_item_display() {
        let s = ua_style("li");
        assert_eq!(s.display, Display::ListItem);
        assert!((s.margin_bottom_mm - 1.5).abs() < 1e-9);
    }

    #[test]
    fn hr_border_and_margins() {
        let s = ua_style("hr");
        assert!((s.margin_top_mm - 4.0).abs() < 1e-9);
        assert!((s.margin_bottom_mm - 4.0).abs() < 1e-9);
        assert!((s.border_bottom_width_mm - 0.2).abs() < 1e-9);
    }

    #[test]
    fn th_padding_and_bold() {
        let s = ua_style("th");
        assert!((s.padding_top_mm - 1.0).abs() < 1e-9);
        assert!((s.padding_bottom_mm - 1.0).abs() < 1e-9);
        assert_eq!(s.font_weight, FontWeight::Bold);
    }

    // ── Unknown element ─────────────────────────────────────────

    #[test]
    fn unknown_element_gets_defaults() {
        let s = ua_style("custom-element");
        // Should be default ComputedStyle values
        assert_eq!(s.display, Display::Block);
        assert_eq!(s.font_weight, FontWeight::Normal);
        assert!((s.font_size_pt - 11.0).abs() < 1e-9); // default
    }
}
