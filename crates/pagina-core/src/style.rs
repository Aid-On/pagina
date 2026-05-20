use markup5ever_rcdom::{Handle, NodeData};

use crate::css::values::*;
use crate::css::{AncestorInfo, CssRule, Declaration};
use crate::css::parser;

// ═══════════════════════════════════════════════════════════════
//  Computed style
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub display: Display,
    pub font_size_pt: f64,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_family: String,
    pub color: Color,
    pub text_align: TextAlign,
    pub line_height: f64,
    pub margin_top_mm: f64,
    pub margin_bottom_mm: f64,
    pub padding_top_mm: f64,
    pub padding_bottom_mm: f64,
    pub break_before: BreakValue,
    pub break_after: BreakValue,
    pub border_bottom_width_mm: f64,
    pub border_bottom_color: Color,
    // CSS Paged Media
    pub string_set: Option<(String, StringSetSource)>,
    pub is_footnote: bool,
    /// CSS `content` property for generated content (e.g. target-counter in TOC links).
    pub content: Option<Vec<ContentItem>>,
}

#[derive(Debug, Clone)]
pub enum StringSetSource {
    Content,
    Attr(String),
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            font_size_pt: 11.0,
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            font_family: "Helvetica".to_string(),
            color: Color::BLACK,
            text_align: TextAlign::Left,
            line_height: 1.4,
            margin_top_mm: 0.0,
            margin_bottom_mm: 0.0,
            padding_top_mm: 0.0,
            padding_bottom_mm: 0.0,
            break_before: BreakValue::Auto,
            break_after: BreakValue::Auto,
            border_bottom_width_mm: 0.0,
            border_bottom_color: Color::BLACK,
            string_set: None,
            is_footnote: false,
            content: None,
        }
    }
}

/// UA defaults for common HTML tags.
fn ua_style(tag: &str) -> ComputedStyle {
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
        "p" => { s.margin_bottom_mm = 3.5; }
        "blockquote" => { s.margin_top_mm = 3.0; s.margin_bottom_mm = 3.0; }
        "pre" => { s.font_family = "Courier".to_string(); s.font_size_pt = 10.0; s.margin_top_mm = 2.0; s.margin_bottom_mm = 2.0; }
        "li" => { s.display = Display::ListItem; s.margin_bottom_mm = 1.5; }
        "hr" => { s.margin_top_mm = 4.0; s.margin_bottom_mm = 4.0; s.border_bottom_width_mm = 0.2; }
        "table" => { s.margin_top_mm = 3.0; s.margin_bottom_mm = 3.0; }
        "td" => { s.padding_top_mm = 1.0; s.padding_bottom_mm = 1.0; }
        "th" => { s.padding_top_mm = 1.0; s.padding_bottom_mm = 1.0; s.font_weight = FontWeight::Bold; }
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════════
//  Styled tree
// ═══════════════════════════════════════════════════════════════

#[derive(Debug)]
pub struct StyledNode {
    pub tag: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub style: ComputedStyle,
    pub children: Vec<StyledContent>,
    pub attrs: Vec<(String, String)>,
}

#[derive(Debug)]
pub enum StyledContent {
    Element(StyledNode),
    Text(String),
}

/// Build a styled tree from a DOM handle + CSS rules.
pub fn build_styled_tree(handle: &Handle, rules: &[CssRule]) -> Option<StyledNode> {
    build_styled_node(handle, rules, &ComputedStyle::default(), &[])
}

/// Context passed through the tree-building recursion.
struct StyleContext<'a> {
    rules: &'a [CssRule],
    parent_style: &'a ComputedStyle,
    ancestors: &'a [AncestorInfo],
}

fn build_styled_node(
    handle: &Handle,
    rules: &[CssRule],
    parent_style: &ComputedStyle,
    ancestors: &[AncestorInfo],
) -> Option<StyledNode> {
    let ctx = StyleContext { rules, parent_style, ancestors };
    match &handle.data {
        NodeData::Document => build_document_node(handle, &ctx),
        NodeData::Element { name, attrs, .. } => {
            let tag = name.local.as_ref().to_ascii_lowercase();
            let attrs_vec = collect_attrs(attrs);
            build_element_node(handle, &ctx, tag, attrs_vec)
        }
        _ => None,
    }
}

fn build_document_node(handle: &Handle, ctx: &StyleContext) -> Option<StyledNode> {
    let mut children = Vec::new();
    for child in handle.children.borrow().iter() {
        if let Some(node) = build_styled_node(child, ctx.rules, ctx.parent_style, ctx.ancestors) {
            children.push(StyledContent::Element(node));
            continue;
        }
        let NodeData::Text { contents } = &child.data else { continue };
        let text = contents.borrow().to_string();
        if !text.trim().is_empty() {
            children.push(StyledContent::Text(text));
        }
    }
    Some(StyledNode {
        tag: "#document".to_string(),
        id: None,
        classes: Vec::new(),
        style: ctx.parent_style.clone(),
        children,
        attrs: Vec::new(),
    })
}

fn collect_attrs(attrs: &std::cell::RefCell<Vec<markup5ever::Attribute>>) -> Vec<(String, String)> {
    attrs.borrow().iter()
        .map(|a| (a.name.local.as_ref().to_string(), a.value.to_string()))
        .collect()
}

fn find_attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn build_element_node(
    handle: &Handle,
    ctx: &StyleContext,
    tag: String,
    attrs_vec: Vec<(String, String)>,
) -> Option<StyledNode> {
    if matches!(tag.as_str(), "style" | "script" | "link" | "meta" | "title" | "head") {
        return None;
    }

    let id = find_attr(&attrs_vec, "id").map(String::from);
    let classes: Vec<String> = find_attr(&attrs_vec, "class")
        .map(|v| v.split_whitespace().map(String::from).collect())
        .unwrap_or_default();
    let inline_style_str = find_attr(&attrs_vec, "style").map(String::from);

    let elem = ElementInfo { tag: &tag, id: &id, classes: &classes };
    let mut style = compute_element_style(&elem, ctx);

    if let Some(inline_css) = inline_style_str {
        let decls = parser::parse_inline_style(&inline_css);
        apply_declarations(&mut style, &decls);
    }

    if style.display == Display::None {
        return None;
    }

    let child_ancestors = make_child_ancestors(&tag, &id, &classes, ctx.ancestors);
    let child_ctx = StyleContext { rules: ctx.rules, parent_style: &style, ancestors: &child_ancestors };
    let children = build_children(handle, &child_ctx, &tag);

    Some(StyledNode { tag, id, classes, style, children, attrs: attrs_vec })
}

struct ElementInfo<'a> {
    tag: &'a str,
    id: &'a Option<String>,
    classes: &'a [String],
}

fn compute_element_style(elem: &ElementInfo, ctx: &StyleContext) -> ComputedStyle {
    let mut style = ua_style(elem.tag);
    inherit_from_parent(&mut style, ctx.parent_style);
    apply_matched_rules(&mut style, elem, ctx);
    style
}

fn apply_matched_rules(style: &mut ComputedStyle, elem: &ElementInfo, ctx: &StyleContext) {
    let mut matched: Vec<(u16, u16, u16, usize, &[Declaration])> = Vec::new();
    for (i, rule) in ctx.rules.iter().enumerate() {
        for sel in &rule.selectors {
            if sel.matches(elem.tag, elem.id, elem.classes, ctx.ancestors) {
                let s = sel.specificity();
                matched.push((s.0, s.1, s.2, i, &rule.declarations));
            }
        }
    }
    matched.sort_by_key(|m| (m.0, m.1, m.2, m.3));
    for (_, _, _, _, decls) in &matched {
        apply_declarations(style, decls);
    }
}

fn inherit_from_parent(style: &mut ComputedStyle, parent: &ComputedStyle) {
    style.font_size_pt = if style.font_size_pt != 11.0 { style.font_size_pt } else { parent.font_size_pt };
    style.color = if matches!(style.color, Color { r: 0, g: 0, b: 0, a: 1.0, .. }) {
        parent.color
    } else {
        style.color
    };
    style.font_family = if style.font_family == "Helvetica" && parent.font_family != "Helvetica" {
        parent.font_family.clone()
    } else {
        std::mem::take(&mut style.font_family)
    };
    style.text_align = parent.text_align;
    style.line_height = parent.line_height;
}

fn make_child_ancestors(tag: &str, id: &Option<String>, classes: &[String], ancestors: &[AncestorInfo]) -> Vec<AncestorInfo> {
    let mut child_ancestors = vec![AncestorInfo {
        tag: tag.to_string(),
        id: id.clone(),
        classes: classes.to_vec(),
    }];
    child_ancestors.extend_from_slice(ancestors);
    child_ancestors
}

fn build_children(handle: &Handle, ctx: &StyleContext, parent_tag: &str) -> Vec<StyledContent> {
    let is_pre = parent_tag == "pre";
    let mut children = Vec::new();
    for child in handle.children.borrow().iter() {
        match &child.data {
            NodeData::Text { contents } => {
                let text = contents.borrow().to_string();
                if !text.trim().is_empty() || (is_pre && !text.is_empty()) {
                    children.push(StyledContent::Text(text));
                }
            }
            NodeData::Element { .. } => {
                if let Some(node) = build_styled_node(child, ctx.rules, ctx.parent_style, ctx.ancestors) {
                    children.push(StyledContent::Element(node));
                }
            }
            _ => {}
        }
    }
    children
}

fn apply_declarations(style: &mut ComputedStyle, decls: &[Declaration]) {
    for decl in decls {
        apply_single_declaration(style, decl);
    }
}

fn apply_single_declaration(style: &mut ComputedStyle, decl: &Declaration) {
    let base_pt = style.font_size_pt;
    if !apply_font_declaration(style, &decl.property, &decl.value, base_pt) {
        apply_box_declaration(style, &decl.property, &decl.value, base_pt);
    }
}

/// Apply font/text-related declarations. Returns true if the property was handled.
fn apply_font_declaration(style: &mut ComputedStyle, prop: &str, value: &str, base_pt: f64) -> bool {
    match prop {
        "font-size" => apply_opt_length_pt(value, base_pt, &mut style.font_size_pt),
        "font-weight" => set_if_some(&mut style.font_weight, parser::parse_font_weight_value(value)),
        "font-style" => set_if_some(&mut style.font_style, parser::parse_font_style_value(value)),
        "color" => set_if_some(&mut style.color, parser::parse_color_value(value)),
        "text-align" => set_if_some(&mut style.text_align, parser::parse_text_align_value(value)),
        "line-height" => apply_line_height(style, value, base_pt),
        "display" => set_if_some(&mut style.display, parser::parse_display_value(value)),
        _ => return false,
    }
    true
}

fn set_if_some<T>(target: &mut T, value: Option<T>) {
    if let Some(v) = value {
        *target = v;
    }
}

fn apply_opt_length_pt(value: &str, base_pt: f64, target: &mut f64) {
    if let Some(len) = parser::parse_length_value(value) {
        *target = len.to_pt(base_pt);
    }
}

/// Apply box-model and other declarations.
fn apply_box_declaration(style: &mut ComputedStyle, prop: &str, value: &str, base_pt: f64) {
    if !apply_spacing_declaration(style, prop, value, base_pt) {
        apply_misc_declaration(style, prop, value, base_pt);
    }
}

fn apply_spacing_declaration(style: &mut ComputedStyle, prop: &str, value: &str, base_pt: f64) -> bool {
    match prop {
        "margin" => apply_margin_shorthand(style, value, base_pt),
        "margin-top" => apply_opt_length_mm(value, base_pt, &mut style.margin_top_mm),
        "margin-bottom" => apply_opt_length_mm(value, base_pt, &mut style.margin_bottom_mm),
        "padding-top" => apply_opt_length_mm(value, base_pt, &mut style.padding_top_mm),
        "padding-bottom" => apply_opt_length_mm(value, base_pt, &mut style.padding_bottom_mm),
        "border-bottom" => apply_border_bottom(style, value, base_pt),
        _ => return false,
    }
    true
}

fn apply_opt_length_mm(value: &str, base_pt: f64, target: &mut f64) {
    if let Some(len) = parser::parse_length_value(value) {
        *target = len.to_mm(base_pt);
    }
}

fn apply_misc_declaration(style: &mut ComputedStyle, prop: &str, value: &str, _base_pt: f64) {
    match prop {
        "break-before" | "page-break-before" => set_if_some(&mut style.break_before, parser::parse_break_value(value)),
        "break-after" | "page-break-after" => set_if_some(&mut style.break_after, parser::parse_break_value(value)),
        "string-set" => apply_string_set(style, value),
        "float" => { if value.trim() == "footnote" { style.is_footnote = true; } }
        "content" => { let items = parser::parse_content_value(value); if !items.is_empty() { style.content = Some(items); } }
        _ => {}
    }
}

fn apply_line_height(style: &mut ComputedStyle, value: &str, base_pt: f64) {
    if let Ok(v) = value.trim().parse::<f64>() {
        style.line_height = v;
    } else if let Some(len) = parser::parse_length_value(value) {
        style.line_height = len.to_pt(base_pt) / base_pt;
    }
}

fn apply_margin_shorthand(style: &mut ComputedStyle, value: &str, base_pt: f64) {
    if let Some(len) = parser::parse_length_value(value) {
        let mm = len.to_mm(base_pt);
        style.margin_top_mm = mm;
        style.margin_bottom_mm = mm;
    }
}

fn apply_string_set(style: &mut ComputedStyle, value: &str) {
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let name = parts[0].to_string();
    let source = if parts[1].starts_with("attr(") {
        let attr = parts[1].strip_prefix("attr(").and_then(|s| s.strip_suffix(')'))
            .unwrap_or("title").to_string();
        StringSetSource::Attr(attr)
    } else {
        StringSetSource::Content
    };
    style.string_set = Some((name, source));
}

fn apply_border_bottom(style: &mut ComputedStyle, value: &str, base_pt: f64) {
    let parts: Vec<&str> = value.split_whitespace().collect();
    if let Some(first) = parts.first() {
        if let Some(len) = parser::parse_length_value(first) {
            style.border_bottom_width_mm = len.to_mm(base_pt);
        }
    }
    if let Some(color_str) = parts.last() {
        if let Some(c) = parser::parse_color_value(color_str) {
            style.border_bottom_color = c;
        }
    }
}
