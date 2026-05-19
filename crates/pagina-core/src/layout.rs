use std::collections::HashMap;

use crate::css::values::*;
use crate::css::PageStyleSet;
use crate::style::{ComputedStyle, StringSetSource, StyledContent, StyledNode};

// ═══════════════════════════════════════════════════════════════
//  Layout types
// ═══════════════════════════════════════════════════════════════

#[derive(Debug)]
pub struct Page {
    pub items: Vec<LayoutItem>,
    pub footnotes: Vec<LayoutItem>,
    pub margin_boxes: Vec<ResolvedMarginBox>,
}

#[derive(Debug, Clone)]
pub struct LayoutItem {
    pub x_mm: f64,
    pub y_mm: f64,
    pub font_size_pt: f64,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_family: String,
    pub color: Color,
    pub text: String,
    pub kind: ItemKind,
}

#[derive(Debug, Clone)]
pub enum ItemKind {
    Text,
    HorizontalRule { width_mm: f64, thickness_mm: f64, color: Color },
    FootnoteMarker(usize),
    FootnoteRef(usize),
}

#[derive(Debug)]
pub struct ResolvedMarginBox {
    pub position: MarginBoxPosition,
    pub text: String,
    pub font_size_pt: f64,
    pub color: Color,
    pub text_align: TextAlign,
}

// ═══════════════════════════════════════════════════════════════
//  Layout state
// ═══════════════════════════════════════════════════════════════

struct LayoutState {
    page_styles: PageStyleSet,
    content_width_mm: f64,
    content_height_mm: f64,

    pages: Vec<Page>,
    current_y: f64,

    // Running strings for margin boxes
    running_strings: HashMap<String, String>,

    // Footnote state
    footnotes_pending: Vec<FootnoteData>,
    footnote_counter: usize,
    footnote_area_height: f64,
}

struct FootnoteData {
    number: usize,
    text: String,
    style: ComputedStyle,
}

impl LayoutState {
    fn new(page_styles: PageStyleSet) -> Self {
        let content_width = page_styles.base.content_width_mm();
        let content_height = page_styles.base.content_height_mm();
        Self {
            page_styles,
            content_width_mm: content_width,
            content_height_mm: content_height,
            pages: vec![Page {
                items: Vec::new(),
                footnotes: Vec::new(),
                margin_boxes: Vec::new(),
            }],
            current_y: 0.0,
            running_strings: HashMap::new(),
            footnotes_pending: Vec::new(),
            footnote_counter: 0,
            footnote_area_height: 0.0,
        }
    }

    fn new_page(&mut self) {
        self.flush_footnotes();
        self.pages.push(Page {
            items: Vec::new(),
            footnotes: Vec::new(),
            margin_boxes: Vec::new(),
        });
        self.current_y = 0.0;
        self.footnote_area_height = 0.0;
    }

    fn push_item(&mut self, item: LayoutItem) {
        self.pages.last_mut().unwrap().items.push(item);
    }

    fn add_footnote(&mut self, text: String, style: &ComputedStyle) {
        self.footnote_counter += 1;
        let num = self.footnote_counter;
        let fn_style = ComputedStyle {
            font_size_pt: 8.0,
            ..style.clone()
        };
        self.footnotes_pending.push(FootnoteData {
            number: num,
            text,
            style: fn_style,
        });
        // Reserve space for the footnote
        let lh = 8.0 * 1.3 * 25.4 / 72.0;
        self.footnote_area_height += lh + 1.0; // rough estimate
    }

    fn flush_footnotes(&mut self) {
        if self.footnotes_pending.is_empty() {
            return;
        }

        let page = self.pages.last_mut().unwrap();
        let footnotes = std::mem::take(&mut self.footnotes_pending);

        // Footnotes render at the bottom of the content area
        let fn_start_y = self.content_height_mm - self.footnote_area_height;
        let mut fn_y = fn_start_y;

        // Separator line
        page.footnotes.push(LayoutItem {
            x_mm: 0.0,
            y_mm: fn_y,
            font_size_pt: 8.0,
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            font_family: "Helvetica".to_string(),
            color: Color::BLACK,
            text: String::new(),
            kind: ItemKind::HorizontalRule {
                width_mm: self.content_width_mm * 0.3,
                thickness_mm: 0.15,
                color: Color::rgb(128, 128, 128),
            },
        });
        fn_y += 2.0;

        for fnd in &footnotes {
            let lh = fnd.style.font_size_pt * 1.3 * 25.4 / 72.0;
            let marker_text = format!("{}. {}", fnd.number, fnd.text);
            page.footnotes.push(LayoutItem {
                x_mm: 0.0,
                y_mm: fn_y,
                font_size_pt: fnd.style.font_size_pt,
                font_weight: fnd.style.font_weight,
                font_style: fnd.style.font_style,
                font_family: fnd.style.font_family.clone(),
                color: fnd.style.color,
                text: marker_text,
                kind: ItemKind::Text,
            });
            fn_y += lh;
        }

        self.footnote_area_height = 0.0;
    }

    fn resolve_margin_boxes(&mut self) {
        let total_pages = self.pages.len();
        for page_num in 0..total_pages {
            let page_style = self.page_styles.for_page(page_num + 1, total_pages);
            let mut boxes = Vec::new();

            for (pos, mb) in &page_style.margin_boxes {
                let text = resolve_content(
                    &mb.content,
                    page_num + 1,
                    total_pages,
                    &self.running_strings,
                );
                if !text.is_empty() {
                    boxes.push(ResolvedMarginBox {
                        position: *pos,
                        text,
                        font_size_pt: mb.font_size_pt.unwrap_or(9.0),
                        color: mb.color.unwrap_or(Color::BLACK),
                        text_align: mb.text_align.unwrap_or(match pos {
                            MarginBoxPosition::TopLeft | MarginBoxPosition::BottomLeft => TextAlign::Left,
                            MarginBoxPosition::TopCenter | MarginBoxPosition::BottomCenter => TextAlign::Center,
                            MarginBoxPosition::TopRight | MarginBoxPosition::BottomRight => TextAlign::Right,
                            _ => TextAlign::Center,
                        }),
                    });
                }
            }

            self.pages[page_num].margin_boxes = boxes;
        }
    }
}

fn resolve_content(
    items: &[ContentItem],
    page_num: usize,
    total_pages: usize,
    running_strings: &HashMap<String, String>,
) -> String {
    let mut out = String::new();
    for item in items {
        match item {
            ContentItem::String(s) => out.push_str(s),
            ContentItem::Counter(name) => match name.as_str() {
                "page" => out.push_str(&page_num.to_string()),
                "pages" => out.push_str(&total_pages.to_string()),
                _ => {}
            },
            ContentItem::RunningString(name) => {
                if let Some(val) = running_strings.get(name) {
                    out.push_str(val);
                }
            }
            ContentItem::None => {}
            _ => {}
        }
    }
    out
}

// ═══════════════════════════════════════════════════════════════
//  Text helpers
// ═══════════════════════════════════════════════════════════════

fn char_width_mm(font_size_pt: f64, font_weight: FontWeight) -> f64 {
    let factor = match font_weight {
        FontWeight::Bold => 0.54,
        FontWeight::Normal => 0.50,
    };
    font_size_pt * factor * 25.4 / 72.0
}

fn line_height_mm(font_size_pt: f64, line_height: f64) -> f64 {
    font_size_pt * line_height * 25.4 / 72.0
}

fn wrap_lines(text: &str, max_width_mm: f64, cw: f64) -> Vec<String> {
    let max_chars = (max_width_mm / cw).floor() as usize;
    if max_chars == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    for raw_line in text.split('\n') {
        let words: Vec<&str> = raw_line.split_whitespace().collect();
        if words.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in words {
            if current.is_empty() {
                current = word.to_string();
            } else if current.len() + 1 + word.len() <= max_chars {
                current.push(' ');
                current.push_str(word);
            } else {
                lines.push(std::mem::take(&mut current));
                current = word.to_string();
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

// ═══════════════════════════════════════════════════════════════
//  Main layout
// ═══════════════════════════════════════════════════════════════

pub fn lay_out(page_styles: &PageStyleSet, tree: &StyledNode) -> Vec<Page> {
    let mut state = LayoutState::new(page_styles.clone());
    lay_out_node(tree, &mut state, false);
    state.flush_footnotes();
    state.resolve_margin_boxes();

    // Remove trailing empty pages
    while state.pages.len() > 1 && state.pages.last().map_or(false, |p| p.items.is_empty() && p.footnotes.is_empty()) {
        state.pages.pop();
    }

    state.pages
}

fn lay_out_node(node: &StyledNode, state: &mut LayoutState, is_first_block: bool) {
    // Update running strings
    if let Some((name, source)) = &node.style.string_set {
        let value = match source {
            StringSetSource::Content => collect_text_content(node),
            StringSetSource::Attr(attr) => node
                .attrs
                .iter()
                .find(|(k, _)| k == attr)
                .map(|(_, v)| v.clone())
                .unwrap_or_default(),
        };
        state.running_strings.insert(name.clone(), value);
    }

    // Handle break-before
    if node.style.break_before == BreakValue::Page && !is_first_block && state.current_y > 0.0 {
        state.new_page();
    }

    match node.tag.as_str() {
        "#document" | "html" | "body" | "main" | "article" | "section" | "div" | "header"
        | "footer" | "nav" | "aside" | "figure" => {
            let mut first = true;
            for child in &node.children {
                match child {
                    StyledContent::Element(child_node) => {
                        lay_out_node(child_node, state, first && is_first_block);
                        first = false;
                    }
                    StyledContent::Text(text) => {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            lay_out_text_block(trimmed, &node.style, state);
                        }
                    }
                }
            }
        }
        "hr" => {
            state.current_y += node.style.margin_top_mm;
            state.push_item(LayoutItem {
                x_mm: 0.0,
                y_mm: state.current_y,
                font_size_pt: 0.0,
                font_weight: FontWeight::Normal,
                font_style: FontStyle::Normal,
                font_family: String::new(),
                color: Color::BLACK,
                text: String::new(),
                kind: ItemKind::HorizontalRule {
                    width_mm: state.content_width_mm,
                    thickness_mm: node.style.border_bottom_width_mm.max(0.2),
                    color: node.style.border_bottom_color,
                },
            });
            state.current_y += node.style.margin_bottom_mm + 0.5;
        }
        "ul" | "ol" => {
            state.current_y += node.style.margin_top_mm;
            let mut counter = 0;
            for child in &node.children {
                if let StyledContent::Element(li) = child {
                    counter += 1;
                    let prefix = if node.tag == "ol" {
                        format!("{}. ", counter)
                    } else {
                        "\u{2022} ".to_string() // bullet
                    };
                    lay_out_list_item(li, &prefix, state);
                }
            }
            state.current_y += node.style.margin_bottom_mm;
        }
        "table" => {
            lay_out_table(node, state);
        }
        _ => {
            // Leaf block: h1-h6, p, blockquote, pre, li, etc.
            lay_out_block(node, state);
        }
    }

    // Handle break-after
    if node.style.break_after == BreakValue::Page && state.current_y > 0.0 {
        state.new_page();
    }
}

fn lay_out_block(node: &StyledNode, state: &mut LayoutState) {
    let style = &node.style;

    // Collect text content (handling inline children like <strong>, <em>, <a>)
    let mut segments = Vec::new();
    let mut footnote_refs = Vec::new();
    collect_inline_segments(node, &mut segments, &mut footnote_refs, state);

    // Process footnotes
    for (fn_text, fn_style) in &footnote_refs {
        state.add_footnote(fn_text.clone(), fn_style);
    }

    // Merge segments into a single text for word wrapping (simplified)
    let full_text: String = segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    let trimmed = full_text.trim();
    if trimmed.is_empty() && style.border_bottom_width_mm == 0.0 {
        return;
    }

    state.current_y += style.margin_top_mm + style.padding_top_mm;

    if !trimmed.is_empty() {
        // Use the primary style for layout metrics
        let primary_style = if segments.is_empty() { style } else { &segments[0].style };
        lay_out_text_block(trimmed, primary_style, state);
    }

    // Border bottom
    if style.border_bottom_width_mm > 0.0 {
        state.push_item(LayoutItem {
            x_mm: 0.0,
            y_mm: state.current_y,
            font_size_pt: 0.0,
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            font_family: String::new(),
            color: Color::BLACK,
            text: String::new(),
            kind: ItemKind::HorizontalRule {
                width_mm: state.content_width_mm,
                thickness_mm: style.border_bottom_width_mm,
                color: style.border_bottom_color,
            },
        });
        state.current_y += style.border_bottom_width_mm + 0.5;
    }

    state.current_y += style.padding_bottom_mm + style.margin_bottom_mm;
}

struct InlineSegment {
    text: String,
    style: ComputedStyle,
}

fn collect_inline_segments(
    node: &StyledNode,
    segments: &mut Vec<InlineSegment>,
    footnotes: &mut Vec<(String, ComputedStyle)>,
    state: &mut LayoutState,
) {
    for child in &node.children {
        match child {
            StyledContent::Text(text) => {
                segments.push(InlineSegment {
                    text: text.clone(),
                    style: node.style.clone(),
                });
            }
            StyledContent::Element(child_node) => {
                if child_node.style.is_footnote {
                    // Extract footnote text
                    let fn_text = collect_text_content(child_node);
                    state.footnote_counter += 1;
                    let num = state.footnote_counter;
                    footnotes.push((fn_text, child_node.style.clone()));
                    // Insert superscript reference
                    segments.push(InlineSegment {
                        text: format!("[{num}]"),
                        style: ComputedStyle {
                            font_size_pt: node.style.font_size_pt * 0.7,
                            ..node.style.clone()
                        },
                    });
                    // Fix: we incremented counter here AND in add_footnote. Undo one.
                    state.footnote_counter -= 1;
                } else if child_node.style.display == Display::Inline {
                    collect_inline_segments(child_node, segments, footnotes, state);
                } else {
                    // Block inside inline — treat as inline for simplicity
                    collect_inline_segments(child_node, segments, footnotes, state);
                }
            }
        }
    }
}

fn lay_out_text_block(text: &str, style: &ComputedStyle, state: &mut LayoutState) {
    let cw = char_width_mm(style.font_size_pt, style.font_weight);
    let lh = line_height_mm(style.font_size_pt, style.line_height);
    let lines = wrap_lines(text, state.content_width_mm, cw);

    for line in &lines {
        if state.current_y + lh > state.content_height_mm - state.footnote_area_height
            && !state.pages.last().unwrap().items.is_empty()
        {
            state.new_page();
        }

        if !line.is_empty() {
            let x = match style.text_align {
                TextAlign::Center => {
                    let text_width = line.len() as f64 * cw;
                    (state.content_width_mm - text_width).max(0.0) / 2.0
                }
                TextAlign::Right => {
                    let text_width = line.len() as f64 * cw;
                    (state.content_width_mm - text_width).max(0.0)
                }
                _ => 0.0,
            };

            state.push_item(LayoutItem {
                x_mm: x,
                y_mm: state.current_y,
                font_size_pt: style.font_size_pt,
                font_weight: style.font_weight,
                font_style: style.font_style,
                font_family: style.font_family.clone(),
                color: style.color,
                text: line.clone(),
                kind: ItemKind::Text,
            });
        }
        state.current_y += lh;
    }
}

fn lay_out_list_item(li: &StyledNode, prefix: &str, state: &mut LayoutState) {
    let text = collect_text_content(li).trim().to_string();
    if text.is_empty() {
        return;
    }
    let full_text = format!("{prefix}{text}");
    state.current_y += li.style.margin_top_mm;
    lay_out_text_block(&full_text, &li.style, state);
    state.current_y += li.style.margin_bottom_mm;
}

fn lay_out_table(node: &StyledNode, state: &mut LayoutState) {
    state.current_y += node.style.margin_top_mm;

    // Collect rows
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut is_header: Vec<bool> = Vec::new();

    for child in &node.children {
        if let StyledContent::Element(child_node) = child {
            match child_node.tag.as_str() {
                "thead" | "tbody" | "tfoot" => {
                    for row_child in &child_node.children {
                        if let StyledContent::Element(tr) = row_child {
                            if tr.tag == "tr" {
                                let (cells, is_hdr) = collect_table_row(tr);
                                rows.push(cells);
                                is_header.push(is_hdr);
                            }
                        }
                    }
                }
                "tr" => {
                    let (cells, is_hdr) = collect_table_row(child_node);
                    rows.push(cells);
                    is_header.push(is_hdr);
                }
                _ => {}
            }
        }
    }

    if rows.is_empty() {
        return;
    }

    // Calculate column widths (equal distribution)
    let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(1);
    let col_width = state.content_width_mm / num_cols as f64;
    let cell_padding = 1.5; // mm

    let fs = node.style.font_size_pt;
    let lh = line_height_mm(fs, node.style.line_height);

    for (row_idx, row) in rows.iter().enumerate() {
        if state.current_y + lh + cell_padding * 2.0 > state.content_height_mm {
            state.new_page();
        }

        let is_hdr = is_header.get(row_idx).copied().unwrap_or(false);

        for (col_idx, cell_text) in row.iter().enumerate() {
            let x = col_idx as f64 * col_width + cell_padding;
            state.push_item(LayoutItem {
                x_mm: x,
                y_mm: state.current_y + cell_padding,
                font_size_pt: fs,
                font_weight: if is_hdr { FontWeight::Bold } else { node.style.font_weight },
                font_style: node.style.font_style,
                font_family: node.style.font_family.clone(),
                color: node.style.color,
                text: cell_text.clone(),
                kind: ItemKind::Text,
            });
        }

        // Row separator
        state.current_y += lh + cell_padding * 2.0;
        if is_hdr || row_idx == rows.len() - 1 {
            state.push_item(LayoutItem {
                x_mm: 0.0,
                y_mm: state.current_y,
                font_size_pt: 0.0,
                font_weight: FontWeight::Normal,
                font_style: FontStyle::Normal,
                font_family: String::new(),
                color: Color::BLACK,
                text: String::new(),
                kind: ItemKind::HorizontalRule {
                    width_mm: state.content_width_mm,
                    thickness_mm: if is_hdr { 0.3 } else { 0.15 },
                    color: Color::rgb(180, 180, 180),
                },
            });
            state.current_y += 0.5;
        }
    }

    state.current_y += node.style.margin_bottom_mm;
}

fn collect_table_row(tr: &StyledNode) -> (Vec<String>, bool) {
    let mut cells = Vec::new();
    let mut is_header = false;
    for child in &tr.children {
        if let StyledContent::Element(td) = child {
            if td.tag == "th" {
                is_header = true;
            }
            cells.push(collect_text_content(td).trim().to_string());
        }
    }
    (cells, is_header)
}

fn collect_text_content(node: &StyledNode) -> String {
    let mut out = String::new();
    for child in &node.children {
        match child {
            StyledContent::Text(t) => out.push_str(t),
            StyledContent::Element(n) => out.push_str(&collect_text_content(n)),
        }
    }
    out
}
