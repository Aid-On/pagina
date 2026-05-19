pub mod parser;
pub mod values;

use std::collections::HashMap;
use values::*;

/// Resolved `@page` style.
#[derive(Debug, Clone)]
pub struct PageStyle {
    pub width_mm: f64,
    pub height_mm: f64,
    pub margin_top_mm: f64,
    pub margin_right_mm: f64,
    pub margin_bottom_mm: f64,
    pub margin_left_mm: f64,
    pub margin_boxes: HashMap<MarginBoxPosition, MarginBox>,
}

impl PageStyle {
    pub fn content_width_mm(&self) -> f64 {
        self.width_mm - self.margin_left_mm - self.margin_right_mm
    }

    pub fn content_height_mm(&self) -> f64 {
        self.height_mm - self.margin_top_mm - self.margin_bottom_mm
    }
}

impl Default for PageStyle {
    fn default() -> Self {
        Self {
            width_mm: 210.0,
            height_mm: 297.0,
            margin_top_mm: 25.0,
            margin_right_mm: 20.0,
            margin_bottom_mm: 25.0,
            margin_left_mm: 20.0,
            margin_boxes: HashMap::new(),
        }
    }
}

/// Content and style of a page-margin box.
#[derive(Debug, Clone)]
pub struct MarginBox {
    pub content: Vec<ContentItem>,
    pub font_size_pt: Option<f64>,
    pub color: Option<Color>,
    pub text_align: Option<TextAlign>,
}

/// Named page sizes in mm (width, height in portrait).
pub fn named_page_size(name: &str) -> Option<(f64, f64)> {
    Some(match name.to_ascii_lowercase().as_str() {
        "a3" => (297.0, 420.0),
        "a4" => (210.0, 297.0),
        "a5" => (148.0, 210.0),
        "b4" => (250.0, 353.0),
        "b5" => (176.0, 250.0),
        "letter" => (215.9, 279.4),
        "legal" => (215.9, 355.6),
        "ledger" => (279.4, 431.8),
        _ => return None,
    })
}

/// A parsed CSS rule: selector(s) + declarations.
#[derive(Debug, Clone)]
pub struct CssRule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

/// Simple CSS selector.
#[derive(Debug, Clone)]
pub enum Selector {
    /// `*`
    Universal,
    /// `h1`, `p`, etc.
    Type(String),
    /// `.classname`
    Class(String),
    /// `#id`
    Id(String),
    /// `tag.class`
    TypeAndClass(String, String),
}

impl Selector {
    pub fn specificity(&self) -> (u16, u16, u16) {
        match self {
            Selector::Universal => (0, 0, 0),
            Selector::Type(_) => (0, 0, 1),
            Selector::Class(_) => (0, 1, 0),
            Selector::Id(_) => (1, 0, 0),
            Selector::TypeAndClass(_, _) => (0, 1, 1),
        }
    }
}

/// A single CSS declaration.
#[derive(Debug, Clone)]
pub struct Declaration {
    pub property: String,
    pub value: String,
}

/// Resolved page style for a specific page type.
#[derive(Debug, Clone)]
pub struct PageStyleSet {
    pub base: PageStyle,
    pub first: Option<PageStyleOverride>,
    pub left: Option<PageStyleOverride>,
    pub right: Option<PageStyleOverride>,
}

impl Default for PageStyleSet {
    fn default() -> Self {
        Self {
            base: PageStyle::default(),
            first: None,
            left: None,
            right: None,
        }
    }
}

/// Override for specific page types (`:first`, `:left`, `:right`).
#[derive(Debug, Clone, Default)]
pub struct PageStyleOverride {
    pub margin_boxes: HashMap<MarginBoxPosition, MarginBox>,
    // Content `none` entries to suppress base margin boxes
    pub suppress_boxes: Vec<MarginBoxPosition>,
}

impl PageStyleSet {
    /// Get effective page style for a given page number (1-indexed).
    pub fn for_page(&self, page_num: usize, total_pages: usize) -> PageStyle {
        let mut style = self.base.clone();

        // Apply :first override
        if page_num == 1 {
            if let Some(first) = &self.first {
                for pos in &first.suppress_boxes {
                    style.margin_boxes.remove(pos);
                }
                for (pos, mb) in &first.margin_boxes {
                    style.margin_boxes.insert(*pos, mb.clone());
                }
            }
        }

        // Apply :left / :right (even pages are left in a left-to-right book)
        let is_left = page_num % 2 == 0;
        let side = if is_left { &self.left } else { &self.right };
        if let Some(s) = side {
            for pos in &s.suppress_boxes {
                style.margin_boxes.remove(pos);
            }
            for (pos, mb) in &s.margin_boxes {
                style.margin_boxes.insert(*pos, mb.clone());
            }
        }

        let _ = total_pages; // reserved for future use
        style
    }
}
