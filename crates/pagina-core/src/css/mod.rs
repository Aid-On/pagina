pub mod parser;

/// Resolved `@page` style.
#[derive(Debug, Clone)]
pub struct PageStyle {
    pub width_mm: f64,
    pub height_mm: f64,
    pub margin_top_mm: f64,
    pub margin_right_mm: f64,
    pub margin_bottom_mm: f64,
    pub margin_left_mm: f64,
}

impl PageStyle {
    /// Content-area width (page width minus horizontal margins).
    pub fn content_width_mm(&self) -> f64 {
        self.width_mm - self.margin_left_mm - self.margin_right_mm
    }

    /// Content-area height (page height minus vertical margins).
    pub fn content_height_mm(&self) -> f64 {
        self.height_mm - self.margin_top_mm - self.margin_bottom_mm
    }
}

impl Default for PageStyle {
    fn default() -> Self {
        // A4 with 25mm top/bottom, 20mm left/right
        Self {
            width_mm: 210.0,
            height_mm: 297.0,
            margin_top_mm: 25.0,
            margin_right_mm: 20.0,
            margin_bottom_mm: 25.0,
            margin_left_mm: 20.0,
        }
    }
}

/// Named page sizes in millimetres (width, height in portrait).
pub fn named_page_size(name: &str) -> Option<(f64, f64)> {
    match name.to_ascii_lowercase().as_str() {
        "a3" => Some((297.0, 420.0)),
        "a4" => Some((210.0, 297.0)),
        "a5" => Some((148.0, 210.0)),
        "b4" => Some((250.0, 353.0)),
        "b5" => Some((176.0, 250.0)),
        "letter" => Some((215.9, 279.4)),
        "legal" => Some((215.9, 355.6)),
        "ledger" => Some((279.4, 431.8)),
        _ => None,
    }
}
