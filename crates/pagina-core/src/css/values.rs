/// CSS length with unit.
#[derive(Debug, Clone, Copy)]
pub enum Length {
    Mm(f64),
    Cm(f64),
    In(f64),
    Pt(f64),
    Pc(f64),
    Px(f64),
    Em(f64),
    Percent(f64),
    Zero,
}

/// Conversion factor from a length unit to millimetres (em_base_pt = 1.0).
/// Em and Percent are handled separately in `to_mm`.
fn unit_to_mm_factor(value: f64, variant_index: u8, em_base_pt: f64) -> f64 {
    const FACTORS: [(f64, f64); 6] = [
        (1.0, 0.0),          // Mm:  v * 1.0
        (10.0, 0.0),         // Cm:  v * 10.0
        (25.4, 0.0),         // In:  v * 25.4
        (25.4, 72.0),        // Pt:  v * 25.4 / 72.0
        (25.4, 6.0),         // Pc:  v * 25.4 / 6.0
        (25.4, 96.0),        // Px:  v * 25.4 / 96.0
    ];
    let _ = em_base_pt;
    let (mul, div) = FACTORS[variant_index as usize];
    if div == 0.0 { value * mul } else { value * mul / div }
}

impl Length {
    /// Resolve to millimetres. `em_base_pt` is the current font size in pt.
    pub fn to_mm(self, em_base_pt: f64) -> f64 {
        match self {
            Length::Mm(v) => unit_to_mm_factor(v, 0, em_base_pt),
            Length::Cm(v) => unit_to_mm_factor(v, 1, em_base_pt),
            Length::In(v) => unit_to_mm_factor(v, 2, em_base_pt),
            Length::Pt(v) => unit_to_mm_factor(v, 3, em_base_pt),
            Length::Pc(v) => unit_to_mm_factor(v, 4, em_base_pt),
            Length::Px(v) => unit_to_mm_factor(v, 5, em_base_pt),
            Length::Em(v) => v * em_base_pt * 25.4 / 72.0,
            Length::Percent(v) => v / 100.0 * em_base_pt * 25.4 / 72.0,
            Length::Zero => 0.0,
        }
    }

    /// Resolve to points.
    pub fn to_pt(self, em_base_pt: f64) -> f64 {
        match self {
            Length::Pt(v) => v,
            Length::Em(v) => v * em_base_pt,
            Length::Percent(v) => v / 100.0 * em_base_pt,
            other => other.to_mm(em_base_pt) * 72.0 / 25.4,
        }
    }
}

/// CMYK color (values 0.0 - 1.0).
#[derive(Debug, Clone, Copy)]
pub struct CmykColor {
    pub c: f32,
    pub m: f32,
    pub y: f32,
    pub k: f32,
}

/// CSS color (RGB with optional CMYK for print output).
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f64,
    /// When set, PDF output uses CMYK instead of RGB.
    pub cmyk: Option<CmykColor>,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 1.0, cmyk: None };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 1.0, cmyk: None };
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0.0, cmyk: None };

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 1.0, cmyk: None }
    }

    pub fn cmyk(c: f32, m: f32, y: f32, k: f32) -> Self {
        // Also store an approximate RGB fallback
        let r = (255.0 * (1.0 - c) * (1.0 - k)) as u8;
        let g = (255.0 * (1.0 - m) * (1.0 - k)) as u8;
        let b = (255.0 * (1.0 - y) * (1.0 - k)) as u8;
        Self { r, g, b, a: 1.0, cmyk: Some(CmykColor { c, m, y, k }) }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();
        COLOR_TABLE.iter()
            .find(|(n, _)| *n == lower)
            .map(|(_, c)| *c)
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        let (r, g, b) = match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                (r, g, b)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                (r, g, b)
            }
            _ => return None,
        };
        Some(Self::rgb(r, g, b))
    }
}

/// Named color lookup table.
const COLOR_TABLE: &[(&str, Color)] = &[
    ("black", Color::BLACK),
    ("white", Color::WHITE),
    ("red", Color { r: 255, g: 0, b: 0, a: 1.0, cmyk: None }),
    ("green", Color { r: 0, g: 128, b: 0, a: 1.0, cmyk: None }),
    ("blue", Color { r: 0, g: 0, b: 255, a: 1.0, cmyk: None }),
    ("navy", Color { r: 0, g: 0, b: 128, a: 1.0, cmyk: None }),
    ("transparent", Color::TRANSPARENT),
    ("gray", Color { r: 128, g: 128, b: 128, a: 1.0, cmyk: None }),
    ("grey", Color { r: 128, g: 128, b: 128, a: 1.0, cmyk: None }),
    ("darkgray", Color { r: 169, g: 169, b: 169, a: 1.0, cmyk: None }),
    ("darkgrey", Color { r: 169, g: 169, b: 169, a: 1.0, cmyk: None }),
    ("lightgray", Color { r: 211, g: 211, b: 211, a: 1.0, cmyk: None }),
    ("lightgrey", Color { r: 211, g: 211, b: 211, a: 1.0, cmyk: None }),
    ("maroon", Color { r: 128, g: 0, b: 0, a: 1.0, cmyk: None }),
    ("orange", Color { r: 255, g: 165, b: 0, a: 1.0, cmyk: None }),
    ("purple", Color { r: 128, g: 0, b: 128, a: 1.0, cmyk: None }),
    ("teal", Color { r: 0, g: 128, b: 128, a: 1.0, cmyk: None }),
    ("silver", Color { r: 192, g: 192, b: 192, a: 1.0, cmyk: None }),
];

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FontWeight {
    #[default]
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BreakValue {
    #[default]
    Auto,
    Page,
    Avoid,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Display {
    #[default]
    Block,
    Inline,
    None,
    ListItem,
}

/// Items that can appear in the `content` property of margin boxes.
#[derive(Debug, Clone)]
pub enum ContentItem {
    String(String),
    Counter(String),
    Counters(String, String),
    TargetCounter(String, String),
    RunningString(String),
    Attr(String),
    None,
}

/// Position of a margin box within a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarginBoxPosition {
    TopLeftCorner,
    TopLeft,
    TopCenter,
    TopRight,
    TopRightCorner,
    RightTop,
    RightMiddle,
    RightBottom,
    BottomRightCorner,
    BottomRight,
    BottomCenter,
    BottomLeft,
    BottomLeftCorner,
    LeftBottom,
    LeftMiddle,
    LeftTop,
}

/// Lookup table for margin box position names.
const MARGIN_BOX_TABLE: &[(&str, MarginBoxPosition)] = &[
    ("top-left-corner", MarginBoxPosition::TopLeftCorner),
    ("top-left", MarginBoxPosition::TopLeft),
    ("top-center", MarginBoxPosition::TopCenter),
    ("top-right", MarginBoxPosition::TopRight),
    ("top-right-corner", MarginBoxPosition::TopRightCorner),
    ("bottom-right-corner", MarginBoxPosition::BottomRightCorner),
    ("bottom-right", MarginBoxPosition::BottomRight),
    ("bottom-center", MarginBoxPosition::BottomCenter),
    ("bottom-left", MarginBoxPosition::BottomLeft),
    ("bottom-left-corner", MarginBoxPosition::BottomLeftCorner),
    ("right-top", MarginBoxPosition::RightTop),
    ("right-middle", MarginBoxPosition::RightMiddle),
    ("right-bottom", MarginBoxPosition::RightBottom),
    ("left-bottom", MarginBoxPosition::LeftBottom),
    ("left-middle", MarginBoxPosition::LeftMiddle),
    ("left-top", MarginBoxPosition::LeftTop),
];

impl MarginBoxPosition {
    pub fn from_name(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();
        MARGIN_BOX_TABLE.iter()
            .find(|(n, _)| *n == lower)
            .map(|(_, pos)| *pos)
    }

    pub fn is_top(&self) -> bool {
        matches!(
            self,
            Self::TopLeftCorner | Self::TopLeft | Self::TopCenter | Self::TopRight | Self::TopRightCorner
        )
    }

    pub fn is_bottom(&self) -> bool {
        matches!(
            self,
            Self::BottomLeftCorner | Self::BottomLeft | Self::BottomCenter | Self::BottomRight | Self::BottomRightCorner
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Length::to_mm ────────────────────────────────────────────

    #[test]
    fn mm_identity() {
        assert!((Length::Mm(10.0).to_mm(12.0) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn cm_to_mm() {
        assert!((Length::Cm(1.0).to_mm(12.0) - 10.0).abs() < 1e-9);
        assert!((Length::Cm(2.5).to_mm(12.0) - 25.0).abs() < 1e-9);
    }

    #[test]
    fn in_to_mm() {
        assert!((Length::In(1.0).to_mm(12.0) - 25.4).abs() < 1e-9);
    }

    #[test]
    fn pt_to_mm() {
        // 72pt = 1in = 25.4mm
        assert!((Length::Pt(72.0).to_mm(12.0) - 25.4).abs() < 1e-9);
        assert!((Length::Pt(36.0).to_mm(12.0) - 12.7).abs() < 1e-9);
    }

    #[test]
    fn pc_to_mm() {
        // 1pc = 12pt = 25.4/6 mm
        let expected = 25.4 / 6.0;
        assert!((Length::Pc(1.0).to_mm(12.0) - expected).abs() < 1e-9);
    }

    #[test]
    fn px_to_mm() {
        // 96px = 1in = 25.4mm
        assert!((Length::Px(96.0).to_mm(12.0) - 25.4).abs() < 1e-9);
    }

    #[test]
    fn em_to_mm() {
        // 1em at 12pt base = 12pt in mm = 12 * 25.4 / 72
        let expected = 12.0 * 25.4 / 72.0;
        assert!((Length::Em(1.0).to_mm(12.0) - expected).abs() < 1e-9);
    }

    #[test]
    fn em_two_at_24pt() {
        // 2em at 24pt base = 48pt in mm
        let expected = 2.0 * 24.0 * 25.4 / 72.0;
        assert!((Length::Em(2.0).to_mm(24.0) - expected).abs() < 1e-9);
    }

    #[test]
    fn percent_to_mm() {
        // 100% at 12pt = 12pt in mm
        let expected = 12.0 * 25.4 / 72.0;
        assert!((Length::Percent(100.0).to_mm(12.0) - expected).abs() < 1e-9);
    }

    #[test]
    fn percent_50_at_12pt() {
        let expected = 0.5 * 12.0 * 25.4 / 72.0;
        assert!((Length::Percent(50.0).to_mm(12.0) - expected).abs() < 1e-9);
    }

    #[test]
    fn zero_is_zero() {
        assert_eq!(Length::Zero.to_mm(12.0), 0.0);
        assert_eq!(Length::Zero.to_mm(0.0), 0.0);
    }

    // ── Length::to_pt ───────────────────────────────────────────

    #[test]
    fn pt_to_pt_identity() {
        assert!((Length::Pt(14.0).to_pt(12.0) - 14.0).abs() < 1e-9);
    }

    #[test]
    fn em_to_pt() {
        assert!((Length::Em(1.5).to_pt(12.0) - 18.0).abs() < 1e-9);
    }

    #[test]
    fn percent_to_pt() {
        assert!((Length::Percent(150.0).to_pt(12.0) - 18.0).abs() < 1e-9);
    }

    #[test]
    fn mm_to_pt_roundtrip() {
        // 25.4mm = 72pt
        let pt = Length::Mm(25.4).to_pt(12.0);
        assert!((pt - 72.0).abs() < 1e-6);
    }

    // ── Color::from_hex ─────────────────────────────────────────

    #[test]
    fn hex_6digit_with_hash() {
        let c = Color::from_hex("#ff0000").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn hex_6digit_without_hash() {
        let c = Color::from_hex("00ff00").unwrap();
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn hex_3digit_shorthand() {
        let c = Color::from_hex("#f0f").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 255);
    }

    #[test]
    fn hex_3digit_expansion() {
        // #abc -> #aabbcc -> (170, 187, 204)
        let c = Color::from_hex("#abc").unwrap();
        assert_eq!(c.r, 170);
        assert_eq!(c.g, 187);
        assert_eq!(c.b, 204);
    }

    #[test]
    fn hex_invalid_length() {
        assert!(Color::from_hex("#abcd").is_none());
        assert!(Color::from_hex("#ab").is_none());
        assert!(Color::from_hex("").is_none());
    }

    #[test]
    fn hex_invalid_characters() {
        assert!(Color::from_hex("#gggggg").is_none());
    }

    #[test]
    fn hex_black() {
        let c = Color::from_hex("#000000").unwrap();
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn hex_white() {
        let c = Color::from_hex("#ffffff").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 255);
    }

    // ── Color::from_name ────────────────────────────────────────

    #[test]
    fn color_name_black() {
        let c = Color::from_name("black").unwrap();
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn color_name_white() {
        let c = Color::from_name("white").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 255);
    }

    #[test]
    fn color_name_red() {
        let c = Color::from_name("red").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn color_name_case_insensitive() {
        assert!(Color::from_name("Black").is_some());
        assert!(Color::from_name("RED").is_some());
        assert!(Color::from_name("Navy").is_some());
    }

    #[test]
    fn color_name_transparent() {
        let c = Color::from_name("transparent").unwrap();
        assert_eq!(c.a, 0.0);
    }

    #[test]
    fn color_name_grey_alias() {
        let gray = Color::from_name("gray").unwrap();
        let grey = Color::from_name("grey").unwrap();
        assert_eq!(gray.r, grey.r);
        assert_eq!(gray.g, grey.g);
        assert_eq!(gray.b, grey.b);
    }

    #[test]
    fn color_name_unknown() {
        assert!(Color::from_name("chartreuse").is_none());
        assert!(Color::from_name("").is_none());
    }

    // ── Color::cmyk ─────────────────────────────────────────────

    #[test]
    fn cmyk_pure_cyan() {
        let c = Color::cmyk(1.0, 0.0, 0.0, 0.0);
        assert!(c.cmyk.is_some());
        let cmyk = c.cmyk.unwrap();
        assert!((cmyk.c - 1.0).abs() < 1e-6);
        assert!((cmyk.m).abs() < 1e-6);
        // RGB approximation: r=0, g=255, b=255
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 255);
    }

    #[test]
    fn cmyk_pure_black() {
        let c = Color::cmyk(0.0, 0.0, 0.0, 1.0);
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn cmyk_pure_white() {
        let c = Color::cmyk(0.0, 0.0, 0.0, 0.0);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 255);
    }

    // ── Color constants and default ─────────────────────────────

    #[test]
    fn color_default_is_black() {
        let c = Color::default();
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn color_rgb_constructor() {
        let c = Color::rgb(10, 20, 30);
        assert_eq!(c.r, 10);
        assert_eq!(c.g, 20);
        assert_eq!(c.b, 30);
        assert_eq!(c.a, 1.0);
        assert!(c.cmyk.is_none());
    }

    // ── MarginBoxPosition::from_name ────────────────────────────

    #[test]
    fn margin_box_top_center() {
        let pos = MarginBoxPosition::from_name("top-center").unwrap();
        assert_eq!(pos, MarginBoxPosition::TopCenter);
    }

    #[test]
    fn margin_box_bottom_left() {
        let pos = MarginBoxPosition::from_name("bottom-left").unwrap();
        assert_eq!(pos, MarginBoxPosition::BottomLeft);
    }

    #[test]
    fn margin_box_case_insensitive() {
        assert!(MarginBoxPosition::from_name("Top-Center").is_some());
        assert!(MarginBoxPosition::from_name("BOTTOM-RIGHT").is_some());
    }

    #[test]
    fn margin_box_unknown_name() {
        assert!(MarginBoxPosition::from_name("center-center").is_none());
        assert!(MarginBoxPosition::from_name("").is_none());
    }

    #[test]
    fn margin_box_all_positions_parseable() {
        let names = [
            "top-left-corner", "top-left", "top-center", "top-right", "top-right-corner",
            "bottom-right-corner", "bottom-right", "bottom-center", "bottom-left", "bottom-left-corner",
            "right-top", "right-middle", "right-bottom",
            "left-bottom", "left-middle", "left-top",
        ];
        for name in names {
            assert!(MarginBoxPosition::from_name(name).is_some(), "failed to parse: {name}");
        }
    }

    // ── MarginBoxPosition::is_top / is_bottom ───────────────────

    #[test]
    fn is_top_positions() {
        assert!(MarginBoxPosition::TopLeftCorner.is_top());
        assert!(MarginBoxPosition::TopCenter.is_top());
        assert!(MarginBoxPosition::TopRightCorner.is_top());
        assert!(!MarginBoxPosition::BottomCenter.is_top());
        assert!(!MarginBoxPosition::LeftMiddle.is_top());
    }

    #[test]
    fn is_bottom_positions() {
        assert!(MarginBoxPosition::BottomLeftCorner.is_bottom());
        assert!(MarginBoxPosition::BottomCenter.is_bottom());
        assert!(MarginBoxPosition::BottomRightCorner.is_bottom());
        assert!(!MarginBoxPosition::TopCenter.is_bottom());
        assert!(!MarginBoxPosition::RightMiddle.is_bottom());
    }
}
