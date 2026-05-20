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

impl Length {
    /// Resolve to millimetres. `em_base_pt` is the current font size in pt.
    pub fn to_mm(self, em_base_pt: f64) -> f64 {
        match self {
            Length::Mm(v) => v,
            Length::Cm(v) => v * 10.0,
            Length::In(v) => v * 25.4,
            Length::Pt(v) => v * 25.4 / 72.0,
            Length::Pc(v) => v * 25.4 / 6.0,
            Length::Px(v) => v * 25.4 / 96.0,
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
        primary_color_name(&lower).or_else(|| secondary_color_name(&lower))
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

fn primary_color_name(name: &str) -> Option<Color> {
    Some(match name {
        "black" => Color::BLACK,
        "white" => Color::WHITE,
        "red" => Color::rgb(255, 0, 0),
        "green" => Color::rgb(0, 128, 0),
        "blue" => Color::rgb(0, 0, 255),
        "navy" => Color::rgb(0, 0, 128),
        "transparent" => Color::TRANSPARENT,
        _ => return None,
    })
}

fn secondary_color_name(name: &str) -> Option<Color> {
    Some(match name {
        "gray" | "grey" => Color::rgb(128, 128, 128),
        "darkgray" | "darkgrey" => Color::rgb(169, 169, 169),
        "lightgray" | "lightgrey" => Color::rgb(211, 211, 211),
        "maroon" => Color::rgb(128, 0, 0),
        "orange" => Color::rgb(255, 165, 0),
        "purple" => Color::rgb(128, 0, 128),
        "teal" => Color::rgb(0, 128, 128),
        "silver" => Color::rgb(192, 192, 192),
        _ => return None,
    })
}

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

fn margin_box_top_bottom(name: &str) -> Option<MarginBoxPosition> {
    Some(match name {
        "top-left-corner" => MarginBoxPosition::TopLeftCorner,
        "top-left" => MarginBoxPosition::TopLeft,
        "top-center" => MarginBoxPosition::TopCenter,
        "top-right" => MarginBoxPosition::TopRight,
        "top-right-corner" => MarginBoxPosition::TopRightCorner,
        "bottom-right-corner" => MarginBoxPosition::BottomRightCorner,
        "bottom-right" => MarginBoxPosition::BottomRight,
        "bottom-center" => MarginBoxPosition::BottomCenter,
        "bottom-left" => MarginBoxPosition::BottomLeft,
        "bottom-left-corner" => MarginBoxPosition::BottomLeftCorner,
        _ => return None,
    })
}

fn margin_box_side(name: &str) -> Option<MarginBoxPosition> {
    Some(match name {
        "right-top" => MarginBoxPosition::RightTop,
        "right-middle" => MarginBoxPosition::RightMiddle,
        "right-bottom" => MarginBoxPosition::RightBottom,
        "left-bottom" => MarginBoxPosition::LeftBottom,
        "left-middle" => MarginBoxPosition::LeftMiddle,
        "left-top" => MarginBoxPosition::LeftTop,
        _ => return None,
    })
}

impl MarginBoxPosition {
    pub fn from_name(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();
        margin_box_top_bottom(&lower).or_else(|| margin_box_side(&lower))
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
