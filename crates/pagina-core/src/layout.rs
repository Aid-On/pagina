use std::collections::HashMap;

use crate::css::values::*;
use crate::css::PageStyleSet;
use crate::font::FontManager;
use crate::style::{ComputedStyle, StringSetSource, StyledContent, StyledNode};

// ═══════════════════════════════════════════════════════════════
//  Layout types
// ═══════════════════════════════════════════════════════════════

#[derive(Debug)]
pub struct Page {
    pub items: Vec<LayoutItem>,
    pub footnotes: Vec<LayoutItem>,
    pub margin_boxes: Vec<ResolvedMarginBox>,
    pub bookmarks: Vec<Bookmark>,
    pub links: Vec<LinkAnnotation>,
}

impl Page {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            footnotes: Vec::new(),
            margin_boxes: Vec::new(),
            bookmarks: Vec::new(),
            links: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bookmark {
    pub title: String,
    pub level: u8,
    pub y_mm: f64,
}

#[derive(Debug, Clone)]
pub struct LinkAnnotation {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub target: LinkTarget,
}

#[derive(Debug, Clone)]
pub enum LinkTarget {
    /// External URL
    Uri(String),
    /// Internal link to element ID (page resolved later)
    Internal(String),
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

impl LayoutItem {
    fn text_item(x: f64, y: f64, text: String, style: &InlineStyle) -> Self {
        Self {
            x_mm: x,
            y_mm: y,
            font_size_pt: style.font_size_pt,
            font_weight: style.font_weight,
            font_style: style.font_style,
            font_family: style.font_family.clone(),
            color: style.color,
            text,
            kind: ItemKind::Text,
        }
    }

    pub(crate) fn hr_item(x: f64, y: f64, width_mm: f64, thickness_mm: f64, color: Color) -> Self {
        Self {
            x_mm: x,
            y_mm: y,
            font_size_pt: 0.0,
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            font_family: String::new(),
            color: Color::BLACK,
            text: String::new(),
            kind: ItemKind::HorizontalRule { width_mm, thickness_mm, color },
        }
    }

    pub(crate) fn image_item(x: f64, y: f64, id: usize, width_mm: f64, height_mm: f64) -> Self {
        Self {
            x_mm: x,
            y_mm: y,
            font_size_pt: 0.0,
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            font_family: String::new(),
            color: Color::BLACK,
            text: String::new(),
            kind: ItemKind::Image { id, width_mm, height_mm },
        }
    }
}

#[derive(Debug, Clone)]
pub enum ItemKind {
    Text,
    HorizontalRule { width_mm: f64, thickness_mm: f64, color: Color },
    Image { id: usize, width_mm: f64, height_mm: f64 },
}

#[derive(Debug)]
pub struct ResolvedMarginBox {
    pub position: MarginBoxPosition,
    pub text: String,
    pub font_size_pt: f64,
    pub color: Color,
    pub text_align: TextAlign,
}

/// An image loaded from the document, ready for embedding.
#[derive(Debug)]
pub struct LoadedImage {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

// ═══════════════════════════════════════════════════════════════
//  Inline run types (for mixed-style text within a line)
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
struct InlineStyle {
    font_size_pt: f64,
    font_weight: FontWeight,
    font_style: FontStyle,
    font_family: String,
    color: Color,
}

impl InlineStyle {
    fn from_computed(s: &ComputedStyle) -> Self {
        Self {
            font_size_pt: s.font_size_pt,
            font_weight: s.font_weight,
            font_style: s.font_style,
            font_family: s.font_family.clone(),
            color: s.color,
        }
    }

    fn same_as(&self, other: &Self) -> bool {
        self.font_size_pt == other.font_size_pt
            && self.font_weight == other.font_weight
            && self.font_style == other.font_style
            && self.font_family == other.font_family
            && self.color.r == other.color.r
            && self.color.g == other.color.g
            && self.color.b == other.color.b
    }
}

/// A word (or non-breakable token) with its style.
#[derive(Debug, Clone)]
struct StyledWord {
    text: String,
    style: InlineStyle,
    width_mm: f64,
}

/// A laid-out line: sequence of segments, each with position and style.
struct LayoutLine {
    segments: Vec<LineSegment>,
    total_width_mm: f64,
    max_line_height_mm: f64,
}

struct LineSegment {
    text: String,
    x_mm: f64,
    width_mm: f64,
    style: InlineStyle,
}

// ═══════════════════════════════════════════════════════════════
//  Layout state
// ═══════════════════════════════════════════════════════════════

struct LayoutState<'a> {
    page_styles: PageStyleSet,
    fm: &'a FontManager,
    content_width_mm: f64,
    content_height_mm: f64,

    pages: Vec<Page>,
    current_y: f64,

    running_strings: HashMap<String, String>,
    footnotes_pending: Vec<FootnoteData>,
    footnote_counter: usize,
    footnote_area_height: f64,

    images: Vec<LoadedImage>,

    /// Map from element ID to the page number (1-indexed) where it was laid out.
    id_to_page: HashMap<String, usize>,
}

struct FootnoteData {
    number: usize,
    text: String,
    style: InlineStyle,
}

impl<'a> LayoutState<'a> {
    fn new(page_styles: PageStyleSet, fm: &'a FontManager) -> Self {
        let cw = page_styles.base.content_width_mm();
        let ch = page_styles.base.content_height_mm();
        Self {
            page_styles,
            fm,
            content_width_mm: cw,
            content_height_mm: ch,
            pages: vec![Page::new()],
            current_y: 0.0,
            running_strings: HashMap::new(),
            footnotes_pending: Vec::new(),
            footnote_counter: 0,
            footnote_area_height: 0.0,
            images: Vec::new(),
            id_to_page: HashMap::new(),
        }
    }

    fn new_page(&mut self) {
        self.flush_footnotes();
        self.pages.push(Page::new());
        self.current_y = 0.0;
        self.footnote_area_height = 0.0;
    }

    fn available_height(&self) -> f64 {
        self.content_height_mm - self.current_y - self.footnote_area_height
    }

    fn current_page_mut(&mut self) -> &mut Page {
        self.pages.last_mut().expect("pages should never be empty")
    }

    fn current_page_has_items(&self) -> bool {
        self.pages.last().map_or(false, |p| !p.items.is_empty())
    }

    fn push_item(&mut self, item: LayoutItem) {
        self.current_page_mut().items.push(item);
    }

    fn needs_page_break(&self, height_needed: f64) -> bool {
        self.current_y + height_needed > self.available_height() && self.current_page_has_items()
    }

    fn ensure_space(&mut self, height_needed: f64) {
        if self.needs_page_break(height_needed) {
            self.new_page();
        }
    }

    fn add_footnote(&mut self, text: String, style: &InlineStyle) {
        self.footnote_counter += 1;
        let num = self.footnote_counter;
        let fn_style = InlineStyle {
            font_size_pt: 8.0,
            ..style.clone()
        };
        self.footnotes_pending.push(FootnoteData { number: num, text, style: fn_style });
        let lh = 8.0 * 1.3 * 25.4 / 72.0;
        self.footnote_area_height += lh + 1.0;
    }

    fn flush_footnotes(&mut self) {
        if self.footnotes_pending.is_empty() {
            return;
        }
        let page = self.pages.last_mut().expect("pages should never be empty");
        let footnotes = std::mem::take(&mut self.footnotes_pending);
        let mut fn_y = self.content_height_mm - self.footnote_area_height;

        page.footnotes.push(LayoutItem::hr_item(
            0.0, fn_y, self.content_width_mm * 0.3, 0.15, Color::rgb(128, 128, 128),
        ));
        fn_y += 2.0;

        for fnd in &footnotes {
            let lh = fnd.style.font_size_pt * 1.3 * 25.4 / 72.0;
            let text = format!("{}. {}", fnd.number, fnd.text);
            page.footnotes.push(LayoutItem::text_item(0.0, fn_y, text, &fnd.style));
            fn_y += lh;
        }
        self.footnote_area_height = 0.0;
    }

    fn resolve_target_counters(&mut self) {
        let id_map = &self.id_to_page;
        for page in &mut self.pages {
            for item in page.items.iter_mut().chain(page.footnotes.iter_mut()) {
                resolve_target_placeholders(&mut item.text, id_map);
            }
        }
    }

    fn resolve_margin_boxes(&mut self) {
        let total_pages = self.pages.len();
        for page_num in 0..total_pages {
            let page_style = self.page_styles.for_page(page_num + 1, total_pages);
            let ctx = ContentResolveContext {
                page_num: page_num + 1,
                total_pages,
                running_strings: &self.running_strings,
            };
            self.pages[page_num].margin_boxes = build_margin_boxes(&page_style, &ctx);
        }
    }
}

fn resolve_target_placeholders(text: &mut String, id_map: &HashMap<String, usize>) {
    if !text.contains("__TARGET_PAGE:") {
        return;
    }
    let mut resolved = text.clone();
    while let Some(start) = resolved.find("__TARGET_PAGE:") {
        let rest = &resolved[start + 14..];
        let Some(end) = rest.find("__") else { break };
        let id = &rest[..end];
        let page_num = id_map.get(id).copied().unwrap_or(0);
        let replacement = if page_num > 0 { page_num.to_string() } else { "?".to_string() };
        resolved = format!("{}{}{}", &resolved[..start], replacement, &rest[end + 2..]);
    }
    *text = resolved;
}

struct ContentResolveContext<'a> {
    page_num: usize,
    total_pages: usize,
    running_strings: &'a HashMap<String, String>,
}

fn build_margin_boxes(
    page_style: &crate::css::PageStyle,
    ctx: &ContentResolveContext,
) -> Vec<ResolvedMarginBox> {
    let mut boxes = Vec::new();
    for (pos, mb) in &page_style.margin_boxes {
        let text = resolve_content(&mb.content, ctx);
        if text.is_empty() {
            continue;
        }
        boxes.push(ResolvedMarginBox {
            position: *pos,
            text,
            font_size_pt: mb.font_size_pt.unwrap_or(9.0),
            color: mb.color.unwrap_or(Color::BLACK),
            text_align: mb.text_align.unwrap_or_else(|| default_text_align_for(*pos)),
        });
    }
    boxes
}

fn default_text_align_for(pos: MarginBoxPosition) -> TextAlign {
    match pos {
        MarginBoxPosition::TopLeft | MarginBoxPosition::BottomLeft => TextAlign::Left,
        MarginBoxPosition::TopCenter | MarginBoxPosition::BottomCenter => TextAlign::Center,
        MarginBoxPosition::TopRight | MarginBoxPosition::BottomRight => TextAlign::Right,
        _ => TextAlign::Center,
    }
}

fn resolve_content(items: &[ContentItem], ctx: &ContentResolveContext) -> String {
    let mut out = String::new();
    for item in items {
        resolve_single_content_item(item, ctx, &mut out);
    }
    out
}

fn resolve_single_content_item(item: &ContentItem, ctx: &ContentResolveContext, out: &mut String) {
    match item {
        ContentItem::String(s) => out.push_str(s),
        ContentItem::Counter(name) => match name.as_str() {
            "page" => out.push_str(&ctx.page_num.to_string()),
            "pages" => out.push_str(&ctx.total_pages.to_string()),
            _ => {}
        },
        ContentItem::RunningString(name) => {
            if let Some(val) = ctx.running_strings.get(name) {
                out.push_str(val);
            }
        }
        ContentItem::TargetCounter(attr_name, counter_name) => {
            let _ = (attr_name, counter_name);
        }
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════════
//  Inline run collection
// ═══════════════════════════════════════════════════════════════

/// Collector context for inline word gathering.
struct WordCollector<'a> {
    fm: &'a FontManager,
    words: Vec<StyledWord>,
    footnotes: Vec<(String, InlineStyle)>,
    footnote_counter: usize,
}

impl<'a> WordCollector<'a> {
    fn new(fm: &'a FontManager, footnote_counter: usize) -> Self {
        Self { fm, words: Vec::new(), footnotes: Vec::new(), footnote_counter }
    }

    fn collect(&mut self, node: &StyledNode) {
        for child in &node.children {
            match child {
                StyledContent::Text(text) => self.collect_text(text, &node.style),
                StyledContent::Element(child_node) => self.collect_element(child_node, &node.style),
            }
        }
    }

    fn collect_text(&mut self, text: &str, parent_style: &ComputedStyle) {
        let style = InlineStyle::from_computed(parent_style);
        let resolved = self.fm.resolve(&style.font_family, style.font_weight, style.font_style);
        let metrics = self.fm.metrics(&resolved);

        for segment in text.split_inclusive(' ') {
            let word_part = segment.trim_end_matches(' ');
            if !word_part.is_empty() {
                let width = metrics.text_width_mm(word_part, style.font_size_pt);
                self.words.push(StyledWord { text: word_part.to_string(), style: style.clone(), width_mm: width });
            }
            if segment.ends_with(' ') {
                let sw = metrics.space_width_mm(style.font_size_pt);
                self.words.push(StyledWord { text: " ".to_string(), style: style.clone(), width_mm: sw });
            }
        }
    }

    fn collect_element(&mut self, child_node: &StyledNode, parent_style: &ComputedStyle) {
        if child_node.style.is_footnote {
            self.collect_footnote(child_node, parent_style);
        } else if child_node.style.content.is_some() {
            self.collect_generated_content(child_node);
        } else {
            self.collect(child_node);
        }
    }

    fn collect_footnote(&mut self, child_node: &StyledNode, parent_style: &ComputedStyle) {
        self.footnote_counter += 1;
        let num = self.footnote_counter;
        let fn_text = collect_text_content(child_node);
        self.footnotes.push((fn_text, InlineStyle::from_computed(&child_node.style)));

        let ref_text = format!("[{num}]");
        let ref_style = InlineStyle {
            font_size_pt: parent_style.font_size_pt * 0.7,
            ..InlineStyle::from_computed(parent_style)
        };
        let resolved = self.fm.resolve(&ref_style.font_family, ref_style.font_weight, ref_style.font_style);
        let width = self.fm.metrics(&resolved).text_width_mm(&ref_text, ref_style.font_size_pt);
        self.words.push(StyledWord { text: ref_text, style: ref_style, width_mm: width });
    }

    fn collect_generated_content(&mut self, child_node: &StyledNode) {
        let content_items = child_node.style.content.as_ref().expect("checked is_some above");
        let style = InlineStyle::from_computed(&child_node.style);

        // First, render the element's own inline children
        self.collect(child_node);

        // Then append generated content
        for ci in content_items {
            let text = resolve_generated_content_item(ci, child_node);
            if text.is_empty() {
                continue;
            }
            let resolved = self.fm.resolve(&style.font_family, style.font_weight, style.font_style);
            let width = self.fm.metrics(&resolved).text_width_mm(&text, style.font_size_pt);
            self.words.push(StyledWord { text, style: style.clone(), width_mm: width });
        }
    }
}

fn resolve_generated_content_item(ci: &ContentItem, node: &StyledNode) -> String {
    match ci {
        ContentItem::String(s) => s.clone(),
        ContentItem::Counter(name) => format!("__COUNTER:{name}__"),
        ContentItem::TargetCounter(_attr, _counter) => {
            let href = node.attrs.iter()
                .find(|(k, _)| k == "href")
                .map(|(_, v)| v.as_str())
                .unwrap_or("");
            let target_id = href.strip_prefix('#').unwrap_or(href);
            if target_id.is_empty() {
                "?".to_string()
            } else {
                format!("__TARGET_PAGE:{target_id}__")
            }
        }
        _ => String::new(),
    }
}

// ═══════════════════════════════════════════════════════════════
//  Line breaking with accurate widths
// ═══════════════════════════════════════════════════════════════

struct LineBreaker {
    lines: Vec<LayoutLine>,
    current_segments: Vec<LineSegment>,
    current_x: f64,
    max_lh: f64,
    max_width_mm: f64,
    default_lh: f64,
}

impl LineBreaker {
    fn new(max_width_mm: f64, default_lh: f64) -> Self {
        Self {
            lines: Vec::new(),
            current_segments: Vec::new(),
            current_x: 0.0,
            max_lh: default_lh,
            max_width_mm,
            default_lh,
        }
    }

    fn break_words(mut self, words: &[StyledWord]) -> Vec<LayoutLine> {
        for word in words {
            if word.text.contains('\n') {
                self.handle_newline_word(word);
            } else if word.text == " " {
                self.handle_space(word);
            } else {
                self.handle_regular_word(word);
            }
        }
        if !self.current_segments.is_empty() {
            self.finish_current_line();
        }
        self.lines
    }

    fn handle_newline_word(&mut self, word: &StyledWord) {
        for (i, part) in word.text.split('\n').enumerate() {
            if i > 0 {
                self.finish_current_line();
            }
            if !part.is_empty() {
                self.current_segments.push(LineSegment {
                    text: part.to_string(),
                    x_mm: self.current_x,
                    width_mm: word.width_mm,
                    style: word.style.clone(),
                });
                self.current_x += word.width_mm;
            }
        }
    }

    fn handle_space(&mut self, word: &StyledWord) {
        if self.current_x <= 0.0 {
            return;
        }
        if let Some(last) = self.current_segments.last_mut() {
            if last.style.same_as(&word.style) {
                last.text.push(' ');
                last.width_mm += word.width_mm;
            }
        }
        self.current_x += word.width_mm;
    }

    fn handle_regular_word(&mut self, word: &StyledWord) {
        let word_lh = word.style.font_size_pt * 1.4 * 25.4 / 72.0;

        // Wrap if word doesn't fit
        if self.current_x + word.width_mm > self.max_width_mm && self.current_x > 0.0 {
            self.trim_trailing_space();
            self.finish_current_line();
        }

        self.max_lh = self.max_lh.max(word_lh);

        // Merge with previous segment if same style and was space-terminated
        if self.try_merge_with_previous(word) {
            return;
        }

        self.current_segments.push(LineSegment {
            text: word.text.clone(),
            x_mm: self.current_x,
            width_mm: word.width_mm,
            style: word.style.clone(),
        });
        self.current_x += word.width_mm;
    }

    fn try_merge_with_previous(&mut self, word: &StyledWord) -> bool {
        let Some(last) = self.current_segments.last_mut() else { return false };
        if !last.style.same_as(&word.style) || !last.text.ends_with(' ') {
            return false;
        }
        last.text.push_str(&word.text);
        last.width_mm = self.current_x + word.width_mm - last.x_mm;
        self.current_x += word.width_mm;
        true
    }

    fn trim_trailing_space(&mut self) {
        if let Some(last) = self.current_segments.last_mut() {
            if last.text.ends_with(' ') {
                last.text.pop();
            }
        }
    }

    fn finish_current_line(&mut self) {
        self.lines.push(LayoutLine {
            segments: std::mem::take(&mut self.current_segments),
            total_width_mm: self.current_x,
            max_line_height_mm: self.max_lh,
        });
        self.current_x = 0.0;
        self.max_lh = self.default_lh;
    }
}

fn break_into_lines(words: &[StyledWord], max_width_mm: f64, default_lh: f64) -> Vec<LayoutLine> {
    LineBreaker::new(max_width_mm, default_lh).break_words(words)
}

// ═══════════════════════════════════════════════════════════════
//  Main layout
// ═══════════════════════════════════════════════════════════════

pub fn lay_out(page_styles: &PageStyleSet, tree: &StyledNode, fm: &FontManager) -> (Vec<Page>, Vec<LoadedImage>) {
    let mut state = LayoutState::new(page_styles.clone(), fm);
    lay_out_node(tree, &mut state, true);
    state.flush_footnotes();
    state.resolve_target_counters();
    state.resolve_margin_boxes();

    while state.pages.len() > 1
        && state.pages.last().map_or(false, |p| p.items.is_empty() && p.footnotes.is_empty())
    {
        state.pages.pop();
    }

    let images = std::mem::take(&mut state.images);
    (state.pages, images)
}

fn lay_out_node(node: &StyledNode, state: &mut LayoutState, is_first_block: bool) {
    update_running_strings(node, state);
    handle_break_before(node, state, is_first_block);
    record_element_id(node, state);

    match node.tag.as_str() {
        "#document" | "html" | "body" | "main" | "article" | "section" | "div"
        | "header" | "footer" | "nav" | "aside" | "figure" => {
            lay_out_container(node, state, is_first_block);
        }
        "hr" => lay_out_hr(node, state),
        "img" => lay_out_image(node, state),
        "math" => lay_out_math(node, state),
        "ul" | "ol" => lay_out_list(node, state),
        "table" => lay_out_table(node, state),
        _ => lay_out_block(node, state),
    }

    if node.style.break_after == BreakValue::Page && state.current_y > 0.0 {
        state.new_page();
    }
}

fn update_running_strings(node: &StyledNode, state: &mut LayoutState) {
    let Some((name, source)) = &node.style.string_set else { return };
    let value = match source {
        StringSetSource::Content => collect_text_content(node),
        StringSetSource::Attr(attr) => node.attrs.iter()
            .find(|(k, _)| k == attr)
            .map(|(_, v)| v.clone())
            .unwrap_or_default(),
    };
    state.running_strings.insert(name.clone(), value);
}

fn handle_break_before(node: &StyledNode, state: &mut LayoutState, is_first_block: bool) {
    if node.style.break_before == BreakValue::Page && !is_first_block && state.current_y > 0.0 {
        state.new_page();
    }
}

fn record_element_id(node: &StyledNode, state: &mut LayoutState) {
    if let Some(id) = &node.id {
        state.id_to_page.insert(id.clone(), state.pages.len());
    }
}

fn lay_out_container(node: &StyledNode, state: &mut LayoutState, is_first_block: bool) {
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
                    lay_out_simple_text(trimmed, &node.style, state);
                }
            }
        }
    }
}

fn lay_out_hr(node: &StyledNode, state: &mut LayoutState) {
    state.current_y += node.style.margin_top_mm;
    state.push_item(LayoutItem::hr_item(
        0.0,
        state.current_y,
        state.content_width_mm,
        node.style.border_bottom_width_mm.max(0.2),
        node.style.border_bottom_color,
    ));
    state.current_y += node.style.margin_bottom_mm + 0.5;
}

fn lay_out_list(node: &StyledNode, state: &mut LayoutState) {
    state.current_y += node.style.margin_top_mm;
    let mut counter = 0;
    for child in &node.children {
        let StyledContent::Element(li) = child else { continue };
        counter += 1;
        let prefix = if node.tag == "ol" {
            format!("{}. ", counter)
        } else {
            "- ".to_string()
        };
        lay_out_list_item(li, &prefix, state);
    }
    state.current_y += node.style.margin_bottom_mm;
}

/// Layout a block element with inline children (the core mixed-style path).
fn lay_out_block(node: &StyledNode, state: &mut LayoutState) {
    let style = &node.style;

    let mut collector = WordCollector::new(state.fm, state.footnote_counter);
    collector.collect(node);
    state.footnote_counter = collector.footnote_counter;

    // Register footnotes
    for (fn_text, fn_style) in &collector.footnotes {
        state.add_footnote(fn_text.clone(), fn_style);
    }

    let has_text = collector.words.iter().any(|w| !w.text.trim().is_empty());
    if !has_text && style.border_bottom_width_mm == 0.0 {
        return;
    }

    state.current_y += style.margin_top_mm + style.padding_top_mm;

    emit_heading_bookmark(node, state);

    if has_text {
        emit_block_text_lines(&collector.words, style, state);
    }

    emit_border_bottom(style, state);
    collect_links_from_node(node, state);

    state.current_y += style.padding_bottom_mm + style.margin_bottom_mm;
}

fn emit_heading_bookmark(node: &StyledNode, state: &mut LayoutState) {
    if !matches!(node.tag.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
        return;
    }
    let level = node.tag.as_bytes()[1] - b'0';
    let title = collect_text_content(node).trim().to_string();
    if title.is_empty() {
        return;
    }
    let y = state.current_y;
    state.current_page_mut().bookmarks.push(Bookmark { title, level, y_mm: y });
}

fn emit_block_text_lines(words: &[StyledWord], style: &ComputedStyle, state: &mut LayoutState) {
    let default_lh = state.fm.metrics(
        &state.fm.resolve(&style.font_family, style.font_weight, style.font_style)
    ).line_height_mm(style.font_size_pt, style.line_height);

    let lines = break_into_lines(words, state.content_width_mm, default_lh);

    for line in &lines {
        state.ensure_space(line.max_line_height_mm);
        let align_offset = compute_align_offset(style.text_align, state.content_width_mm, line.total_width_mm);
        emit_line_segments(&line.segments, state.current_y, align_offset, state);
        state.current_y += line.max_line_height_mm;
    }
}

fn compute_align_offset(text_align: TextAlign, content_width: f64, line_width: f64) -> f64 {
    match text_align {
        TextAlign::Center => (content_width - line_width).max(0.0) / 2.0,
        TextAlign::Right => (content_width - line_width).max(0.0),
        _ => 0.0,
    }
}

fn emit_line_segments(segments: &[LineSegment], y: f64, x_offset: f64, state: &mut LayoutState) {
    for seg in segments {
        state.push_item(LayoutItem::text_item(
            seg.x_mm + x_offset, y, seg.text.clone(), &seg.style,
        ));
    }
}

fn emit_border_bottom(style: &ComputedStyle, state: &mut LayoutState) {
    if style.border_bottom_width_mm <= 0.0 {
        return;
    }
    state.push_item(LayoutItem::hr_item(
        0.0, state.current_y, state.content_width_mm,
        style.border_bottom_width_mm, style.border_bottom_color,
    ));
    state.current_y += style.border_bottom_width_mm + 0.5;
}

/// Simple text layout (fallback, single style).
fn lay_out_simple_text(text: &str, style: &ComputedStyle, state: &mut LayoutState) {
    let inline_style = InlineStyle::from_computed(style);
    let resolved = state.fm.resolve(&style.font_family, style.font_weight, style.font_style);
    let metrics = state.fm.metrics(&resolved);
    let lh = metrics.line_height_mm(style.font_size_pt, style.line_height);

    let mut words = Vec::new();
    for word in text.split_whitespace() {
        let w = metrics.text_width_mm(word, style.font_size_pt);
        words.push(StyledWord { text: word.to_string(), style: inline_style.clone(), width_mm: w });
        let sw = metrics.space_width_mm(style.font_size_pt);
        words.push(StyledWord { text: " ".to_string(), style: inline_style.clone(), width_mm: sw });
    }

    let lines = break_into_lines(&words, state.content_width_mm, lh);
    for line in &lines {
        if state.current_y + line.max_line_height_mm > state.content_height_mm
            && state.current_page_has_items()
        {
            state.new_page();
        }
        emit_line_segments(&line.segments, state.current_y, 0.0, state);
        state.current_y += line.max_line_height_mm;
    }
}

fn lay_out_list_item(li: &StyledNode, prefix: &str, state: &mut LayoutState) {
    state.current_y += li.style.margin_top_mm;

    let inline_style = InlineStyle::from_computed(&li.style);
    let resolved = state.fm.resolve(&li.style.font_family, li.style.font_weight, li.style.font_style);
    let metrics = state.fm.metrics(&resolved);

    let mut words = Vec::new();
    let prefix_w = metrics.text_width_mm(prefix, li.style.font_size_pt);
    words.push(StyledWord { text: prefix.to_string(), style: inline_style, width_mm: prefix_w });

    let mut collector = WordCollector::new(state.fm, state.footnote_counter);
    collector.collect(li);
    state.footnote_counter = collector.footnote_counter;
    words.extend(collector.words);
    for (fn_text, fn_style) in &collector.footnotes {
        state.add_footnote(fn_text.clone(), fn_style);
    }

    let lh = metrics.line_height_mm(li.style.font_size_pt, li.style.line_height);
    let lines = break_into_lines(&words, state.content_width_mm, lh);

    for line in &lines {
        state.ensure_space(line.max_line_height_mm);
        emit_line_segments(&line.segments, state.current_y, 0.0, state);
        state.current_y += line.max_line_height_mm;
    }

    state.current_y += li.style.margin_bottom_mm;
}

fn lay_out_image(node: &StyledNode, state: &mut LayoutState) {
    let Some(src) = node.attrs.iter().find(|(k, _)| k == "src").map(|(_, v)| v.as_str()) else {
        return;
    };

    if src.ends_with(".svg") {
        if let Some(loaded) = crate::svg::render_svg_file(src, 300.0) {
            embed_loaded_image(loaded, node, state);
        }
        return;
    }

    let Ok(img) = image::open(src) else { return };

    let (img_w, img_h) = (img.width(), img.height());
    let rgb = img.to_rgb8();

    let (display_w, display_h) = scale_to_fit(img_w, img_h, 96.0, state.content_width_mm);

    state.current_y += node.style.margin_top_mm;
    state.ensure_space(display_h);

    let image_id = state.images.len();
    state.images.push(LoadedImage { pixels: rgb.into_raw(), width: img_w, height: img_h });
    state.push_item(LayoutItem::image_item(0.0, state.current_y, image_id, display_w, display_h));
    state.current_y += display_h + node.style.margin_bottom_mm;
}

fn embed_loaded_image(loaded: LoadedImage, node: &StyledNode, state: &mut LayoutState) {
    let (display_w, display_h) = scale_to_fit(loaded.width, loaded.height, 300.0, state.content_width_mm);

    state.current_y += node.style.margin_top_mm;
    state.ensure_space(display_h);

    let image_id = state.images.len();
    state.images.push(loaded);
    state.push_item(LayoutItem::image_item(0.0, state.current_y, image_id, display_w, display_h));
    state.current_y += display_h + node.style.margin_bottom_mm;
}

fn scale_to_fit(pixel_w: u32, pixel_h: u32, dpi: f64, max_width_mm: f64) -> (f64, f64) {
    let natural_w = pixel_w as f64 / dpi * 25.4;
    let natural_h = pixel_h as f64 / dpi * 25.4;
    let scale = if natural_w > max_width_mm { max_width_mm / natural_w } else { 1.0 };
    (natural_w * scale, natural_h * scale)
}

fn lay_out_table(node: &StyledNode, state: &mut LayoutState) {
    state.current_y += node.style.margin_top_mm;

    let (rows, is_header) = collect_table_rows(node);
    if rows.is_empty() {
        return;
    }

    let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(1);
    let col_width = state.content_width_mm / num_cols as f64;
    let cell_padding = 1.5;
    let resolved = state.fm.resolve(&node.style.font_family, node.style.font_weight, node.style.font_style);
    let lh = state.fm.metrics(&resolved).line_height_mm(node.style.font_size_pt, node.style.line_height);
    let row_height = lh + cell_padding * 2.0;
    let total_rows = rows.len();

    for (row_idx, row) in rows.iter().enumerate() {
        if state.current_y + row_height > state.content_height_mm {
            state.new_page();
        }
        let is_hdr = is_header.get(row_idx).copied().unwrap_or(false);
        emit_table_row(row, is_hdr, col_width, cell_padding, &node.style, state);
        state.current_y += row_height;
        emit_table_row_separator(is_hdr, row_idx, total_rows, state);
    }
    state.current_y += node.style.margin_bottom_mm;
}

fn collect_table_rows(node: &StyledNode) -> (Vec<Vec<String>>, Vec<bool>) {
    let mut rows = Vec::new();
    let mut is_header = Vec::new();

    for child in &node.children {
        let StyledContent::Element(child_node) = child else { continue };
        match child_node.tag.as_str() {
            "thead" | "tbody" | "tfoot" => {
                collect_rows_from_section(child_node, &mut rows, &mut is_header);
            }
            "tr" => {
                let (cells, is_hdr) = collect_table_row(child_node);
                rows.push(cells);
                is_header.push(is_hdr);
            }
            _ => {}
        }
    }
    (rows, is_header)
}

fn collect_rows_from_section(section: &StyledNode, rows: &mut Vec<Vec<String>>, is_header: &mut Vec<bool>) {
    for row_child in &section.children {
        let StyledContent::Element(tr) = row_child else { continue };
        if tr.tag != "tr" {
            continue;
        }
        let (cells, is_hdr) = collect_table_row(tr);
        rows.push(cells);
        is_header.push(is_hdr);
    }
}

fn emit_table_row(
    row: &[String],
    is_hdr: bool,
    col_width: f64,
    cell_padding: f64,
    style: &ComputedStyle,
    state: &mut LayoutState,
) {
    let font_weight = if is_hdr { FontWeight::Bold } else { style.font_weight };
    let inline_style = InlineStyle {
        font_size_pt: style.font_size_pt,
        font_weight,
        font_style: style.font_style,
        font_family: style.font_family.clone(),
        color: style.color,
    };
    for (col_idx, cell_text) in row.iter().enumerate() {
        let x = col_idx as f64 * col_width + cell_padding;
        state.push_item(LayoutItem::text_item(
            x, state.current_y + cell_padding, cell_text.clone(), &inline_style,
        ));
    }
}

fn emit_table_row_separator(is_hdr: bool, row_idx: usize, total_rows: usize, state: &mut LayoutState) {
    if !is_hdr && row_idx != total_rows - 1 {
        return;
    }
    let thickness = if is_hdr { 0.3 } else { 0.15 };
    state.push_item(LayoutItem::hr_item(
        0.0, state.current_y, state.content_width_mm, thickness, Color::rgb(180, 180, 180),
    ));
    state.current_y += 0.5;
}

fn collect_table_row(tr: &StyledNode) -> (Vec<String>, bool) {
    let mut cells = Vec::new();
    let mut is_header = false;
    for child in &tr.children {
        let StyledContent::Element(td) = child else { continue };
        if td.tag == "th" { is_header = true; }
        cells.push(collect_text_content(td).trim().to_string());
    }
    (cells, is_header)
}

fn lay_out_math(node: &StyledNode, state: &mut LayoutState) {
    let items = crate::mathml::render_math(node, node.style.font_size_pt, state.fm);
    let lh = node.style.font_size_pt * 1.6 * 25.4 / 72.0;

    state.current_y += node.style.margin_top_mm;
    state.ensure_space(lh);

    for mut item in items {
        item.y_mm += state.current_y;
        state.push_item(item);
    }

    state.current_y += lh + node.style.margin_bottom_mm;
}

fn collect_links_from_node(node: &StyledNode, state: &mut LayoutState) {
    if node.tag == "a" {
        emit_link_annotation(node, state);
    }
    for child in &node.children {
        if let StyledContent::Element(child_node) = child {
            collect_links_from_node(child_node, state);
        }
    }
}

fn emit_link_annotation(node: &StyledNode, state: &mut LayoutState) {
    let Some(href) = node.attrs.iter().find(|(k, _)| k == "href").map(|(_, v)| v.clone()) else {
        return;
    };
    let text = collect_text_content(node);
    let resolved = state.fm.resolve(&node.style.font_family, node.style.font_weight, node.style.font_style);
    let text_width = state.fm.metrics(&resolved).text_width_mm(text.trim(), node.style.font_size_pt);
    let lh = node.style.font_size_pt * node.style.line_height * 25.4 / 72.0;

    let target = if let Some(id) = href.strip_prefix('#') {
        LinkTarget::Internal(id.to_string())
    } else {
        LinkTarget::Uri(href)
    };

    let annotation = LinkAnnotation {
        x_mm: 0.0,
        y_mm: state.current_y - lh,
        width_mm: text_width.min(state.content_width_mm),
        height_mm: lh,
        target,
    };
    state.current_page_mut().links.push(annotation);
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
