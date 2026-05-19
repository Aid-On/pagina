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

/// CSS color.
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f64,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 1.0 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 1.0 };
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0.0 };

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "black" => Self::BLACK,
            "white" => Self::WHITE,
            "red" => Self::rgb(255, 0, 0),
            "green" => Self::rgb(0, 128, 0),
            "blue" => Self::rgb(0, 0, 255),
            "gray" | "grey" => Self::rgb(128, 128, 128),
            "darkgray" | "darkgrey" => Self::rgb(169, 169, 169),
            "lightgray" | "lightgrey" => Self::rgb(211, 211, 211),
            "navy" => Self::rgb(0, 0, 128),
            "maroon" => Self::rgb(128, 0, 0),
            "orange" => Self::rgb(255, 165, 0),
            "purple" => Self::rgb(128, 0, 128),
            "teal" => Self::rgb(0, 128, 128),
            "silver" => Self::rgb(192, 192, 192),
            "transparent" => Self::TRANSPARENT,
            _ => return None,
        })
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

impl MarginBoxPosition {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "top-left-corner" => Self::TopLeftCorner,
            "top-left" => Self::TopLeft,
            "top-center" => Self::TopCenter,
            "top-right" => Self::TopRight,
            "top-right-corner" => Self::TopRightCorner,
            "right-top" => Self::RightTop,
            "right-middle" => Self::RightMiddle,
            "right-bottom" => Self::RightBottom,
            "bottom-right-corner" => Self::BottomRightCorner,
            "bottom-right" => Self::BottomRight,
            "bottom-center" => Self::BottomCenter,
            "bottom-left" => Self::BottomLeft,
            "bottom-left-corner" => Self::BottomLeftCorner,
            "left-bottom" => Self::LeftBottom,
            "left-middle" => Self::LeftMiddle,
            "left-top" => Self::LeftTop,
            _ => return None,
        })
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
