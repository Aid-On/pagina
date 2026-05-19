use crate::css::PageStyle;
use crate::dom::TextBlock;

#[derive(Debug)]
pub struct Page {
    pub items: Vec<LayoutItem>,
}

#[derive(Debug)]
pub struct LayoutItem {
    /// X offset from left margin (mm).
    pub x_mm: f64,
    /// Y offset from top margin (mm), grows downward.
    pub y_mm: f64,
    pub font_size_pt: f64,
    pub text: String,
}

/// Font metrics approximation for Helvetica.
fn char_width_mm(font_size_pt: f64) -> f64 {
    // Average character width ≈ 0.5 × font size for Helvetica
    // 1 pt = 25.4/72 mm ≈ 0.353 mm
    font_size_pt * 0.5 * 25.4 / 72.0
}

fn line_height_mm(font_size_pt: f64) -> f64 {
    font_size_pt * 1.4 * 25.4 / 72.0
}

fn font_size_for_tag(tag: &str) -> f64 {
    match tag {
        "h1" => 24.0,
        "h2" => 20.0,
        "h3" => 16.0,
        "h4" => 14.0,
        "h5" | "h6" => 12.0,
        _ => 11.0,
    }
}

fn spacing_after_mm(tag: &str) -> f64 {
    match tag {
        "h1" => 6.0,
        "h2" => 5.0,
        "h3" | "h4" => 4.0,
        _ => 3.0,
    }
}

/// Word-wrap `text` into lines that fit within `max_width_mm`.
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

/// Lay out text blocks into pages.
pub fn lay_out(style: &PageStyle, blocks: &[TextBlock]) -> Vec<Page> {
    let content_w = style.content_width_mm();
    let content_h = style.content_height_mm();

    let mut pages: Vec<Page> = vec![Page { items: Vec::new() }];
    let mut y = 0.0_f64; // current Y within content area

    for block in blocks {
        let fs = font_size_for_tag(&block.tag);
        let lh = line_height_mm(fs);
        let cw = char_width_mm(fs);
        let lines = wrap_lines(&block.text, content_w, cw);

        for line in &lines {
            // Check if we need a new page
            if y + lh > content_h && !pages.last().unwrap().items.is_empty() {
                pages.push(Page { items: Vec::new() });
                y = 0.0;
            }

            if !line.is_empty() {
                pages.last_mut().unwrap().items.push(LayoutItem {
                    x_mm: 0.0,
                    y_mm: y,
                    font_size_pt: fs,
                    text: line.clone(),
                });
            }
            y += lh;
        }

        y += spacing_after_mm(&block.tag);
    }

    // Don't return a trailing empty page
    if pages.len() > 1 && pages.last().unwrap().items.is_empty() {
        pages.pop();
    }

    pages
}
