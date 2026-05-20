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

/// Lookup table for named page sizes in mm (width, height in portrait).
const PAGE_SIZE_TABLE: &[(&str, f64, f64)] = &[
    ("a3", 297.0, 420.0),
    ("a4", 210.0, 297.0),
    ("a5", 148.0, 210.0),
    ("b4", 250.0, 353.0),
    ("b5", 176.0, 250.0),
    ("letter", 215.9, 279.4),
    ("legal", 215.9, 355.6),
    ("ledger", 279.4, 431.8),
];

/// Named page sizes in mm (width, height in portrait).
pub fn named_page_size(name: &str) -> Option<(f64, f64)> {
    let lower = name.to_ascii_lowercase();
    PAGE_SIZE_TABLE.iter()
        .find(|(n, _, _)| *n == lower)
        .map(|(_, w, h)| (*w, *h))
}

/// A parsed CSS rule: selector(s) + declarations.
#[derive(Debug, Clone)]
pub struct CssRule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

/// A simple selector (matches a single element).
#[derive(Debug, Clone)]
pub enum SimpleSelector {
    Universal,
    Type(String),
    Class(String),
    Id(String),
    TypeAndClass(String, String),
}

impl SimpleSelector {
    pub fn specificity(&self) -> (u16, u16, u16) {
        match self {
            Self::Universal => (0, 0, 0),
            Self::Type(_) => (0, 0, 1),
            Self::Class(_) => (0, 1, 0),
            Self::Id(_) => (1, 0, 0),
            Self::TypeAndClass(_, _) => (0, 1, 1),
        }
    }

    pub fn matches(&self, tag: &str, id: &Option<String>, classes: &[String]) -> bool {
        match self {
            Self::Universal => true,
            Self::Type(t) => t == tag,
            Self::Class(c) => classes.iter().any(|cl| cl == c),
            Self::Id(i) => id.as_deref() == Some(i.as_str()),
            Self::TypeAndClass(t, c) => t == tag && classes.iter().any(|cl| cl == c),
        }
    }

    fn matches_ancestor(&self, anc: &AncestorInfo) -> bool {
        self.matches(&anc.tag, &anc.id, &anc.classes)
    }
}

/// Combinator between simple selectors.
#[derive(Debug, Clone, Copy)]
pub enum Combinator {
    /// ` ` (descendant)
    Descendant,
    /// `>` (child)
    Child,
}

/// A compound selector: a chain of simple selectors joined by combinators.
/// Read right-to-left: the last element is the subject.
#[derive(Debug, Clone)]
pub struct Selector {
    /// Chain of (combinator, simple_selector) pairs, from outermost ancestor to subject.
    /// The first entry has a dummy Descendant combinator (ignored).
    pub parts: Vec<(Combinator, SimpleSelector)>,
}

impl Selector {
    /// Create a simple (single-element) selector.
    pub fn simple(s: SimpleSelector) -> Self {
        Self { parts: vec![(Combinator::Descendant, s)] }
    }

    pub fn specificity(&self) -> (u16, u16, u16) {
        self.parts.iter().fold((0u16, 0u16, 0u16), |(a, b, c), (_, s)| {
            let (sa, sb, sc) = s.specificity();
            (a + sa, b + sb, c + sc)
        })
    }

    /// The subject (rightmost) simple selector.
    pub fn subject(&self) -> &SimpleSelector {
        &self.parts.last().expect("selector should have at least one part").1
    }

    /// Match this selector against an element with its ancestor chain.
    /// `ancestors` is ordered from closest parent to root.
    pub fn matches(
        &self,
        elem: &MatchTarget,
        ancestors: &[AncestorInfo],
    ) -> bool {
        let n = self.parts.len();
        if n == 0 {
            return false;
        }

        // Subject must match
        if !self.parts[n - 1].1.matches(&elem.tag, &elem.id, &elem.classes) {
            return false;
        }

        if n == 1 {
            return true;
        }

        self.match_ancestor_chain(ancestors)
    }

    fn match_ancestor_chain(&self, ancestors: &[AncestorInfo]) -> bool {
        let n = self.parts.len();
        let mut ancestor_idx = 0;

        for part_idx in (0..n - 1).rev() {
            let (combinator, ref simple) = self.parts[part_idx];
            let matched = match combinator {
                Combinator::Child => match_child(simple, ancestors, &mut ancestor_idx),
                Combinator::Descendant => match_descendant(simple, ancestors, &mut ancestor_idx),
            };
            if !matched {
                return false;
            }
        }
        true
    }
}

fn match_child(simple: &SimpleSelector, ancestors: &[AncestorInfo], idx: &mut usize) -> bool {
    if *idx >= ancestors.len() {
        return false;
    }
    let matched = simple.matches_ancestor(&ancestors[*idx]);
    if matched {
        *idx += 1;
    }
    matched
}

fn match_descendant(simple: &SimpleSelector, ancestors: &[AncestorInfo], idx: &mut usize) -> bool {
    while *idx < ancestors.len() {
        let anc = &ancestors[*idx];
        *idx += 1;
        if simple.matches_ancestor(anc) {
            return true;
        }
    }
    false
}

/// Info about an ancestor element, for selector matching.
#[derive(Debug, Clone)]
pub struct AncestorInfo {
    pub tag: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
}

/// Target element for selector matching.
pub struct MatchTarget<'a> {
    pub tag: &'a str,
    pub id: &'a Option<String>,
    pub classes: &'a [String],
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

        if page_num == 1 {
            apply_override(&mut style, self.first.as_ref());
        }

        let side_override = if page_num % 2 == 0 { &self.left } else { &self.right };
        apply_override(&mut style, side_override.as_ref());

        let _ = total_pages; // reserved for future use
        style
    }
}

fn apply_override(style: &mut PageStyle, page_override: Option<&PageStyleOverride>) {
    let Some(ovr) = page_override else { return };
    for pos in &ovr.suppress_boxes {
        style.margin_boxes.remove(pos);
    }
    for (pos, mb) in &ovr.margin_boxes {
        style.margin_boxes.insert(*pos, mb.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ─────────────────────────────────────────────────

    fn target<'a>(tag: &'a str, id: &'a Option<String>, classes: &'a [String]) -> MatchTarget<'a> {
        MatchTarget { tag, id, classes }
    }

    fn ancestor(tag: &str, id: Option<&str>, classes: &[&str]) -> AncestorInfo {
        AncestorInfo {
            tag: tag.to_string(),
            id: id.map(String::from),
            classes: classes.iter().map(|s| s.to_string()).collect(),
        }
    }

    // ── SimpleSelector::matches ─────────────────────────────────

    #[test]
    fn universal_matches_anything() {
        let sel = SimpleSelector::Universal;
        let no_id = None;
        assert!(sel.matches("div", &no_id, &[]));
        assert!(sel.matches("p", &Some("myid".into()), &["cls".into()]));
    }

    #[test]
    fn type_selector_matches_tag() {
        let sel = SimpleSelector::Type("p".into());
        assert!(sel.matches("p", &None, &[]));
        assert!(!sel.matches("div", &None, &[]));
    }

    #[test]
    fn class_selector_matches_class() {
        let sel = SimpleSelector::Class("note".into());
        assert!(sel.matches("p", &None, &["note".into()]));
        assert!(sel.matches("div", &None, &["foo".into(), "note".into()]));
        assert!(!sel.matches("p", &None, &["other".into()]));
    }

    #[test]
    fn id_selector_matches_id() {
        let sel = SimpleSelector::Id("main".into());
        assert!(sel.matches("div", &Some("main".into()), &[]));
        assert!(!sel.matches("div", &Some("other".into()), &[]));
        assert!(!sel.matches("div", &None, &[]));
    }

    #[test]
    fn type_and_class_selector() {
        let sel = SimpleSelector::TypeAndClass("p".into(), "highlight".into());
        assert!(sel.matches("p", &None, &["highlight".into()]));
        assert!(!sel.matches("div", &None, &["highlight".into()]));
        assert!(!sel.matches("p", &None, &["other".into()]));
    }

    // ── SimpleSelector::specificity ─────────────────────────────

    #[test]
    fn specificity_universal() {
        assert_eq!(SimpleSelector::Universal.specificity(), (0, 0, 0));
    }

    #[test]
    fn specificity_type() {
        assert_eq!(SimpleSelector::Type("p".into()).specificity(), (0, 0, 1));
    }

    #[test]
    fn specificity_class() {
        assert_eq!(SimpleSelector::Class("c".into()).specificity(), (0, 1, 0));
    }

    #[test]
    fn specificity_id() {
        assert_eq!(SimpleSelector::Id("i".into()).specificity(), (1, 0, 0));
    }

    #[test]
    fn specificity_type_and_class() {
        assert_eq!(SimpleSelector::TypeAndClass("p".into(), "c".into()).specificity(), (0, 1, 1));
    }

    // ── Selector (simple) ───────────────────────────────────────

    #[test]
    fn simple_selector_matches_element() {
        let sel = Selector::simple(SimpleSelector::Type("h1".into()));
        let no_id = None;
        let t = target("h1", &no_id, &[]);
        assert!(sel.matches(&t, &[]));
    }

    #[test]
    fn simple_selector_no_match() {
        let sel = Selector::simple(SimpleSelector::Type("h1".into()));
        let no_id = None;
        let t = target("h2", &no_id, &[]);
        assert!(!sel.matches(&t, &[]));
    }

    // ── Selector with descendant combinator ─────────────────────

    #[test]
    fn descendant_selector_div_p() {
        // "div p" matches <p> inside <div>
        let sel = Selector {
            parts: vec![
                (Combinator::Descendant, SimpleSelector::Type("div".into())),
                (Combinator::Descendant, SimpleSelector::Type("p".into())),
            ],
        };
        let no_id = None;
        let t = target("p", &no_id, &[]);
        let ancestors = vec![ancestor("div", None, &[])];
        assert!(sel.matches(&t, &ancestors));
    }

    #[test]
    fn descendant_selector_deep_nesting() {
        // "div p" should match <p> even through intermediary elements
        let sel = Selector {
            parts: vec![
                (Combinator::Descendant, SimpleSelector::Type("div".into())),
                (Combinator::Descendant, SimpleSelector::Type("p".into())),
            ],
        };
        let no_id = None;
        let t = target("p", &no_id, &[]);
        let ancestors = vec![
            ancestor("section", None, &[]),
            ancestor("div", None, &[]),
        ];
        assert!(sel.matches(&t, &ancestors));
    }

    #[test]
    fn descendant_selector_no_match_wrong_ancestor() {
        let sel = Selector {
            parts: vec![
                (Combinator::Descendant, SimpleSelector::Type("div".into())),
                (Combinator::Descendant, SimpleSelector::Type("p".into())),
            ],
        };
        let no_id = None;
        let t = target("p", &no_id, &[]);
        let ancestors = vec![ancestor("section", None, &[])];
        assert!(!sel.matches(&t, &ancestors));
    }

    // ── Selector with child combinator ──────────────────────────

    #[test]
    fn child_selector_direct_parent() {
        // "div > p" matches <p> directly inside <div>
        let sel = Selector {
            parts: vec![
                (Combinator::Descendant, SimpleSelector::Type("div".into())),
                (Combinator::Child, SimpleSelector::Type("p".into())),
            ],
        };
        let no_id = None;
        let t = target("p", &no_id, &[]);
        let ancestors = vec![ancestor("div", None, &[])];
        assert!(sel.matches(&t, &ancestors));
    }

    #[test]
    fn three_part_child_combinator_requires_direct_parent() {
        // "body > div > p": the Child combinator on `div` part IS consulted
        // because `div` is not the subject. So <p> must be directly inside <div>.
        let sel = Selector {
            parts: vec![
                (Combinator::Descendant, SimpleSelector::Type("body".into())),
                (Combinator::Child, SimpleSelector::Type("div".into())),
                (Combinator::Child, SimpleSelector::Type("p".into())),
            ],
        };
        let no_id = None;

        // Direct: body > div > p -- should match
        let t = target("p", &no_id, &[]);
        let ancestors = vec![
            ancestor("div", None, &[]),
            ancestor("body", None, &[]),
        ];
        assert!(sel.matches(&t, &ancestors));

        // Intermediary between div and p: body > div > section > p -- should NOT match
        // because part_idx=1 is (Child, div), so ancestors[0] must be div, but it's section
        let ancestors_with_gap = vec![
            ancestor("section", None, &[]),
            ancestor("div", None, &[]),
            ancestor("body", None, &[]),
        ];
        assert!(!sel.matches(&t, &ancestors_with_gap));
    }

    // ── Selector with class in ancestor chain ───────────────────

    #[test]
    fn class_in_ancestor_chain() {
        // ".container p"
        let sel = Selector {
            parts: vec![
                (Combinator::Descendant, SimpleSelector::Class("container".into())),
                (Combinator::Descendant, SimpleSelector::Type("p".into())),
            ],
        };
        let no_id = None;
        let t = target("p", &no_id, &[]);
        let ancestors = vec![ancestor("div", None, &["container"])];
        assert!(sel.matches(&t, &ancestors));
    }

    // ── Selector::specificity compound ──────────────────────────

    #[test]
    fn compound_specificity_adds_up() {
        // "div .note" => (0,0,1) + (0,1,0) = (0,1,1)
        let sel = Selector {
            parts: vec![
                (Combinator::Descendant, SimpleSelector::Type("div".into())),
                (Combinator::Descendant, SimpleSelector::Class("note".into())),
            ],
        };
        assert_eq!(sel.specificity(), (0, 1, 1));
    }

    // ── Empty selector ──────────────────────────────────────────

    #[test]
    fn empty_selector_never_matches() {
        let sel = Selector { parts: vec![] };
        let no_id = None;
        let t = target("p", &no_id, &[]);
        assert!(!sel.matches(&t, &[]));
    }

    // ── PageStyle ───────────────────────────────────────────────

    #[test]
    fn page_style_default_a4() {
        let ps = PageStyle::default();
        assert!((ps.width_mm - 210.0).abs() < 1e-9);
        assert!((ps.height_mm - 297.0).abs() < 1e-9);
    }

    #[test]
    fn page_style_content_dimensions() {
        let ps = PageStyle::default();
        let cw = ps.content_width_mm();
        let ch = ps.content_height_mm();
        assert!((cw - (210.0 - 20.0 - 20.0)).abs() < 1e-9);
        assert!((ch - (297.0 - 25.0 - 25.0)).abs() < 1e-9);
    }

    // ── named_page_size ─────────────────────────────────────────

    #[test]
    fn named_page_size_a4() {
        let (w, h) = named_page_size("a4").unwrap();
        assert!((w - 210.0).abs() < 1e-9);
        assert!((h - 297.0).abs() < 1e-9);
    }

    #[test]
    fn named_page_size_letter() {
        let (w, h) = named_page_size("letter").unwrap();
        assert!((w - 215.9).abs() < 1e-9);
        assert!((h - 279.4).abs() < 1e-9);
    }

    #[test]
    fn named_page_size_case_insensitive() {
        assert!(named_page_size("A4").is_some());
        assert!(named_page_size("LETTER").is_some());
    }

    #[test]
    fn named_page_size_unknown() {
        assert!(named_page_size("tabloid").is_none());
    }

    // ── PageStyleSet::for_page ──────────────────────────────────

    #[test]
    fn page_style_set_first_page_override() {
        let mut pss = PageStyleSet::default();
        pss.first = Some(PageStyleOverride {
            margin_boxes: HashMap::new(),
            suppress_boxes: vec![MarginBoxPosition::TopCenter],
        });
        // Insert a top-center box in the base
        pss.base.margin_boxes.insert(MarginBoxPosition::TopCenter, MarginBox {
            content: vec![ContentItem::String("Header".into())],
            font_size_pt: None,
            color: None,
            text_align: None,
        });

        let page1 = pss.for_page(1, 5);
        // First page should have the box suppressed
        assert!(!page1.margin_boxes.contains_key(&MarginBoxPosition::TopCenter));

        let page2 = pss.for_page(2, 5);
        // Non-first pages keep the box
        assert!(page2.margin_boxes.contains_key(&MarginBoxPosition::TopCenter));
    }
}
