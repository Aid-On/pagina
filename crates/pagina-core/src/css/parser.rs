use cssparser::{BasicParseError, BasicParseErrorKind, ParseError, Parser, ParserInput, SourceLocation, Token};

use super::values::*;
use super::*;

// ═══════════════════════════════════════════════════════════════
//  Public API
// ═══════════════════════════════════════════════════════════════

/// Parse a stylesheet: extract @page rules into `page_styles` and regular rules into `rules`.
pub fn parse_stylesheet(css: &str, page_styles: &mut PageStyleSet, rules: &mut Vec<CssRule>) {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);

    loop {
        let token_kind = match parser.next() {
            Ok(token) => classify_token(token),
            Err(_) => break,
        };
        dispatch_stylesheet_token(token_kind, &mut parser, page_styles, rules);
    }
}

fn dispatch_stylesheet_token(kind: TokenKind, parser: &mut Parser, page_styles: &mut PageStyleSet, rules: &mut Vec<CssRule>) {
    match kind {
        TokenKind::AtPage => parse_at_page(parser, page_styles),
        TokenKind::CurlyBlock => {
            let _ = parser.parse_nested_block(|_| -> Result<(), ParseError<'_, ()>> { Ok(()) });
        }
        TokenKind::Ident(name) => try_parse_qualified_rule(parser, &name, rules),
        TokenKind::Hash(id) => try_parse_qualified_rule(parser, &format!("#{id}"), rules),
        TokenKind::Dot => {
            if let Ok(class) = parser.expect_ident().map(|s| s.as_ref().to_owned()) {
                try_parse_qualified_rule(parser, &format!(".{class}"), rules);
            }
        }
        _ => {}
    }
}

/// Convenience: apply only @page rules (backwards compat).
pub fn apply_page_rules(css: &str, style: &mut PageStyle) {
    let mut pss = PageStyleSet {
        base: style.clone(),
        ..Default::default()
    };
    let mut rules = Vec::new();
    parse_stylesheet(css, &mut pss, &mut rules);
    *style = pss.base;
}

/// Parse an inline `style="..."` attribute into declarations.
pub fn parse_inline_style(css: &str) -> Vec<Declaration> {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    parse_declaration_list(&mut parser)
}

// ═══════════════════════════════════════════════════════════════
//  Token classification (avoids borrow issues)
// ═══════════════════════════════════════════════════════════════

#[derive(Debug)]
enum TokenKind {
    AtPage,
    CurlyBlock,
    Ident(String),
    Hash(String),
    Dot,
    Other,
}

fn classify_token(token: &Token) -> TokenKind {
    match token {
        Token::AtKeyword(kw) if kw.eq_ignore_ascii_case("page") => TokenKind::AtPage,
        Token::CurlyBracketBlock => TokenKind::CurlyBlock,
        Token::Ident(name) => TokenKind::Ident(name.as_ref().to_owned()),
        Token::IDHash(id) => TokenKind::Hash(id.as_ref().to_owned()),
        Token::Delim('.') => TokenKind::Dot,
        _ => TokenKind::Other,
    }
}

// ═══════════════════════════════════════════════════════════════
//  @page parsing
// ═══════════════════════════════════════════════════════════════

fn parse_at_page(parser: &mut Parser, pss: &mut PageStyleSet) {
    // Optional page selector: :first, :left, :right
    let page_selector = parser
        .try_parse(|p| {
            let t = p.next()?.clone();
            match t {
                Token::Colon => {
                    let ident = p.expect_ident()?.as_ref().to_ascii_lowercase();
                    Ok::<_, BasicParseError<'_>>(ident)
                }
                _ => Err(BasicParseError {
                    kind: BasicParseErrorKind::QualifiedRuleInvalid,
                    location: SourceLocation { line: 0, column: 0 },
                }),
            }
        })
        .ok();

    // Expect CurlyBracketBlock
    let found_block = loop {
        match parser.next() {
            Ok(t) if matches!(t, Token::CurlyBracketBlock) => break true,
            Ok(_) => continue,
            Err(_) => break false,
        }
    };

    if !found_block {
        return;
    }

    let _ = parser.parse_nested_block(|block| -> Result<(), ParseError<'_, ()>> {
        parse_page_block(block, pss, page_selector.as_deref());
        Ok(())
    });
}

fn parse_page_block(parser: &mut Parser, pss: &mut PageStyleSet, selector: Option<&str>) {
    while !parser.is_exhausted() {
        let next_kind = match parser.next() {
            Ok(t) => classify_page_block_token(t),
            Err(_) => break,
        };
        handle_page_block_token(next_kind, parser, pss, selector);
    }
}

fn handle_page_block_token(token: PageBlockToken, parser: &mut Parser, pss: &mut PageStyleSet, selector: Option<&str>) {
    match token {
        PageBlockToken::AtRule(name) => handle_page_at_rule(&name, parser, pss, selector),
        PageBlockToken::Ident(name) => handle_page_declaration(&name, parser, pss),
        PageBlockToken::Semicolon | PageBlockToken::Other => {}
    }
}

fn handle_page_at_rule(name: &str, parser: &mut Parser, pss: &mut PageStyleSet, selector: Option<&str>) {
    if let Some(pos) = MarginBoxPosition::from_name(name) {
        parse_margin_box_rule(parser, pss, selector, pos);
    } else {
        skip_at_rule(parser);
    }
}

fn handle_page_declaration(name: &str, parser: &mut Parser, pss: &mut PageStyleSet) {
    if parser.expect_colon().is_err() {
        return;
    }
    apply_page_declaration(name, parser, &mut pss.base);
    let _ = parser.try_parse(|p| p.expect_semicolon());
}

#[derive(Debug)]
enum PageBlockToken {
    AtRule(String),
    Ident(String),
    Semicolon,
    Other,
}

fn classify_page_block_token(token: &Token) -> PageBlockToken {
    match token {
        Token::AtKeyword(name) => PageBlockToken::AtRule(name.as_ref().to_owned()),
        Token::Ident(name) => PageBlockToken::Ident(name.as_ref().to_ascii_lowercase()),
        Token::Semicolon => PageBlockToken::Semicolon,
        _ => PageBlockToken::Other,
    }
}

fn apply_page_declaration(name: &str, parser: &mut Parser, style: &mut PageStyle) {
    match name {
        "size" => parse_size(parser, style),
        "margin" => parse_margin(parser, style),
        "margin-top" => apply_page_margin_side(parser, &mut style.margin_top_mm),
        "margin-right" => apply_page_margin_side(parser, &mut style.margin_right_mm),
        "margin-bottom" => apply_page_margin_side(parser, &mut style.margin_bottom_mm),
        "margin-left" => apply_page_margin_side(parser, &mut style.margin_left_mm),
        _ => skip_value(parser),
    }
}

fn apply_page_margin_side(parser: &mut Parser, target: &mut f64) {
    if let Some(v) = try_length_mm(parser) {
        *target = v;
    }
}

// ═══════════════════════════════════════════════════════════════
//  Margin box parsing
// ═══════════════════════════════════════════════════════════════

fn parse_margin_box_rule(
    parser: &mut Parser,
    pss: &mut PageStyleSet,
    page_selector: Option<&str>,
    pos: MarginBoxPosition,
) {
    if !skip_to_curly_block(parser) {
        return;
    }

    let _ = parser.parse_nested_block(|block| -> Result<(), ParseError<'_, ()>> {
        let parsed = parse_margin_box_declarations(block);
        apply_margin_box(pss, page_selector, pos, parsed);
        Ok(())
    });
}

fn skip_to_curly_block(parser: &mut Parser) -> bool {
    loop {
        match parser.next() {
            Ok(t) if matches!(t, Token::CurlyBracketBlock) => return true,
            Ok(_) => continue,
            Err(_) => return false,
        }
    }
}

struct ParsedMarginBox {
    content_items: Vec<ContentItem>,
    font_size: Option<f64>,
    color: Option<Color>,
    text_align: Option<TextAlign>,
    is_none: bool,
}

fn parse_margin_box_declarations(block: &mut Parser) -> ParsedMarginBox {
    let decls = parse_declaration_list(block);
    let mut result = ParsedMarginBox {
        content_items: Vec::new(),
        font_size: None,
        color: None,
        text_align: None,
        is_none: false,
    };

    for decl in &decls {
        match decl.property.as_str() {
            "content" if decl.value.trim() == "none" => result.is_none = true,
            "content" => result.content_items = parse_content_value(&decl.value),
            "font-size" => result.font_size = parse_length_value(&decl.value).map(|l| l.to_pt(11.0)),
            "color" => result.color = parse_color_value(&decl.value),
            "text-align" => result.text_align = parse_text_align_value(&decl.value),
            _ => {}
        }
    }
    result
}

fn apply_margin_box(
    pss: &mut PageStyleSet,
    page_selector: Option<&str>,
    pos: MarginBoxPosition,
    parsed: ParsedMarginBox,
) {
    let mb = MarginBox {
        content: parsed.content_items,
        font_size_pt: parsed.font_size,
        color: parsed.color,
        text_align: parsed.text_align,
    };

    match page_selector {
        Some("first") | Some("left") | Some("right") => {
            let target = get_or_create_override(pss, page_selector.expect("matched above"));
            if parsed.is_none {
                target.suppress_boxes.push(pos);
            } else if !mb.content.is_empty() {
                target.margin_boxes.insert(pos, mb);
            }
        }
        _ => {
            if parsed.is_none {
                pss.base.margin_boxes.remove(&pos);
            } else if !mb.content.is_empty() {
                pss.base.margin_boxes.insert(pos, mb);
            }
        }
    }
}

fn get_or_create_override<'a>(pss: &'a mut PageStyleSet, selector: &str) -> &'a mut PageStyleOverride {
    match selector {
        "first" => pss.first.get_or_insert_with(PageStyleOverride::default),
        "left" => pss.left.get_or_insert_with(PageStyleOverride::default),
        "right" => pss.right.get_or_insert_with(PageStyleOverride::default),
        _ => unreachable!("only called with first/left/right"),
    }
}

// ═══════════════════════════════════════════════════════════════
//  content property parsing
// ═══════════════════════════════════════════════════════════════

/// Parse a `content` value string into ContentItems.
pub fn parse_content_value(raw: &str) -> Vec<ContentItem> {
    let mut items = Vec::new();
    let mut input = ParserInput::new(raw);
    let mut parser = Parser::new(&mut input);

    while !parser.is_exhausted() {
        let item = {
            match parser.next() {
                Ok(Token::QuotedString(s)) => Some(ContentItem::String(s.as_ref().to_string())),
                Ok(Token::Function(name)) => {
                    let fname = name.as_ref().to_ascii_lowercase();
                    parse_content_function(&mut parser, &fname)
                }
                Ok(Token::Ident(name)) if name.eq_ignore_ascii_case("none") => {
                    Some(ContentItem::None)
                }
                _ => None,
            }
        };
        if let Some(item) = item {
            items.push(item);
        }
    }

    items
}

fn parse_content_function(parser: &mut Parser, fname: &str) -> Option<ContentItem> {
    parser
        .parse_nested_block(|block| -> Result<ContentItem, ParseError<'_, ()>> {
            match fname {
                "counter" => {
                    let name = block.expect_ident()?.as_ref().to_owned();
                    Ok(ContentItem::Counter(name))
                }
                "string" => {
                    let name = block.expect_ident()?.as_ref().to_owned();
                    Ok(ContentItem::RunningString(name))
                }
                "attr" => {
                    let name = block.expect_ident()?.as_ref().to_owned();
                    Ok(ContentItem::Attr(name))
                }
                "target-counter" => {
                    // target-counter(attr(href), page)
                    let _fn_token = block.expect_function_matching("attr")?;
                    let attr_name = block
                        .parse_nested_block(|inner| -> Result<String, ParseError<'_, ()>> {
                            Ok(inner.expect_ident()?.as_ref().to_owned())
                        })?;
                    let _ = block.expect_comma();
                    let counter_name = block.expect_ident()?.as_ref().to_owned();
                    Ok(ContentItem::TargetCounter(attr_name, counter_name))
                }
                _ => Err(block.new_custom_error(())),
            }
        })
        .ok()
}

// ═══════════════════════════════════════════════════════════════
//  Qualified rule (selector + block) parsing
// ═══════════════════════════════════════════════════════════════

fn try_parse_qualified_rule(parser: &mut Parser, first_token: &str, rules: &mut Vec<CssRule>) {
    let Some(selector_text) = collect_selector_text(parser, first_token) else { return };

    let selectors = parse_selector_list(&selector_text);
    if selectors.is_empty() {
        let _ = parser.parse_nested_block(|_| -> Result<(), ParseError<'_, ()>> { Ok(()) });
        return;
    }

    let declarations =
        parser
            .parse_nested_block(|block| -> Result<Vec<Declaration>, ParseError<'_, ()>> {
                Ok(parse_declaration_list(block))
            })
            .unwrap_or_default();

    if !declarations.is_empty() {
        rules.push(CssRule { selectors, declarations });
    }
}

/// Collect selector text tokens until `{`. Returns None if parser exhausted.
fn collect_selector_text(parser: &mut Parser, first_token: &str) -> Option<String> {
    let mut text = first_token.to_owned();
    loop {
        match parser.next() {
            Ok(Token::CurlyBracketBlock) => return Some(text),
            Ok(ref token) => append_selector_token(&mut text, token),
            Err(_) => return None,
        }
    }
}

fn append_selector_token(text: &mut String, token: &Token) {
    match token {
        Token::Ident(s) => { text.push(' '); text.push_str(s.as_ref()); }
        Token::Delim('.') => text.push('.'),
        Token::IDHash(s) => { text.push('#'); text.push_str(s.as_ref()); }
        Token::Comma => text.push(','),
        Token::Colon => text.push(':'),
        Token::WhiteSpace(_) => text.push(' '),
        _ => {}
    }
}

fn parse_selector_list(text: &str) -> Vec<Selector> {
    text.split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                return None;
            }
            Some(parse_compound_selector(s))
        })
        .collect()
}

/// Parse a single compound selector (e.g. ".toc > a", "table td.highlight").
fn parse_compound_selector(text: &str) -> Selector {
    let tokens = tokenize_selector(text);
    let parts = build_selector_parts(&tokens);

    match parts.len() {
        0 => Selector::simple(SimpleSelector::Universal),
        1 => {
            let (_, simple) = parts.into_iter().next().expect("len checked");
            Selector::simple(simple)
        }
        _ => Selector { parts },
    }
}

fn tokenize_selector(text: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        rest = rest.trim_start();
        if rest.starts_with('>') {
            tokens.push(">");
            rest = &rest[1..];
            continue;
        }
        let end = rest.find(|c: char| c.is_whitespace() || c == '>').unwrap_or(rest.len());
        if end == 0 {
            break;
        }
        tokens.push(&rest[..end]);
        rest = &rest[end..];
    }
    tokens
}

fn build_selector_parts(tokens: &[&str]) -> Vec<(Combinator, SimpleSelector)> {
    let mut parts = Vec::new();
    let mut next_combinator = Combinator::Descendant;
    for token in tokens {
        if *token == ">" {
            next_combinator = Combinator::Child;
            continue;
        }
        parts.push((next_combinator, parse_simple_selector(token)));
        next_combinator = Combinator::Descendant;
    }
    parts
}

fn parse_simple_selector(s: &str) -> SimpleSelector {
    if s == "*" {
        return SimpleSelector::Universal;
    }
    if let Some(id) = s.strip_prefix('#') {
        return SimpleSelector::Id(id.to_owned());
    }
    if let Some(class) = s.strip_prefix('.') {
        return SimpleSelector::Class(class.to_owned());
    }
    // tag.class
    if let Some((tag, class)) = s.split_once('.') {
        if !tag.is_empty() && !class.is_empty() {
            return SimpleSelector::TypeAndClass(tag.to_ascii_lowercase(), class.to_owned());
        }
    }
    SimpleSelector::Type(s.to_ascii_lowercase())
}

// ═══════════════════════════════════════════════════════════════
//  Declaration list parsing
// ═══════════════════════════════════════════════════════════════

fn parse_declaration_list(parser: &mut Parser) -> Vec<Declaration> {
    let mut decls = Vec::new();
    while !parser.is_exhausted() {
        if let Some(decl) = try_parse_declaration(parser) {
            decls.push(decl);
        }
    }
    decls
}

fn try_parse_declaration(parser: &mut Parser) -> Option<Declaration> {
    let prop = match parser.expect_ident() {
        Ok(name) => name.as_ref().to_ascii_lowercase(),
        Err(_) => {
            let _ = parser.next();
            return None;
        }
    };

    if parser.expect_colon().is_err() {
        return None;
    }

    let value_parts = collect_value_tokens(parser);
    let value = value_parts.join(" ").trim().to_string();
    if value.is_empty() {
        return None;
    }
    Some(Declaration { property: prop, value })
}

fn collect_value_tokens(parser: &mut Parser) -> Vec<String> {
    let mut parts = Vec::new();
    loop {
        match parser.next() {
            Ok(Token::Semicolon) | Err(_) => break,
            Ok(token) => {
                if let Some(s) = value_token_to_string(&token) {
                    parts.push(s);
                } else if let Token::Function(name) = token {
                    let fname = name.as_ref().to_string();
                    let inner = parse_function_args(parser);
                    parts.push(format!("{fname}({inner})"));
                }
            }
        }
    }
    parts
}

fn value_token_to_string(token: &Token) -> Option<String> {
    match token {
        Token::Ident(s) => Some(s.as_ref().to_string()),
        Token::QuotedString(s) => Some(format!("\"{}\"", s.as_ref())),
        Token::Number { value, .. } => Some(format!("{value}")),
        Token::Percentage { unit_value, .. } => Some(format!("{}%", unit_value * 100.0)),
        Token::Dimension { value, unit, .. } => Some(format!("{value}{}", unit.as_ref())),
        Token::Hash(s) | Token::IDHash(s) => Some(format!("#{}", s.as_ref())),
        Token::Delim('/') => Some("/".to_string()),
        Token::Comma => Some(",".to_string()),
        _ => None,
    }
}

fn parse_function_args(parser: &mut Parser) -> String {
    parser
        .parse_nested_block(|block| -> Result<String, ParseError<'_, ()>> {
            Ok(collect_block_tokens(block))
        })
        .unwrap_or_default()
}

fn collect_block_tokens(block: &mut Parser) -> String {
    let mut parts = Vec::new();
    loop {
        // Classify the token and extract owned data before doing anything else.
        let classified = match block.next() {
            Ok(token) => classify_value_token(token),
            Err(_) => break,
        };
        match classified {
            ValueToken::Simple(s) => parts.push(s),
            ValueToken::NestedFunction(fname) => {
                let inner_args = collect_inner_function_idents(block);
                parts.push(format!("{fname}({inner_args})"));
            }
            ValueToken::Skip => {}
        }
    }
    parts.join(" ")
}

enum ValueToken {
    Simple(String),
    NestedFunction(String),
    Skip,
}

fn classify_value_token(token: &Token) -> ValueToken {
    match token {
        Token::Ident(s) => ValueToken::Simple(s.as_ref().to_string()),
        Token::QuotedString(s) => ValueToken::Simple(format!("\"{}\"", s.as_ref())),
        Token::Number { value, .. } => ValueToken::Simple(format!("{value}")),
        Token::Comma => ValueToken::Simple(",".to_string()),
        Token::Dimension { value, unit, .. } => ValueToken::Simple(format!("{value}{}", unit.as_ref())),
        Token::Function(name) => ValueToken::NestedFunction(name.as_ref().to_string()),
        _ => ValueToken::Skip,
    }
}

fn collect_inner_function_idents(block: &mut Parser) -> String {
    block
        .parse_nested_block(|ib| -> Result<String, ParseError<'_, ()>> {
            let mut ip = Vec::new();
            while let Ok(it) = ib.next() {
                if let Token::Ident(s) = it {
                    ip.push(s.as_ref().to_string());
                }
            }
            Ok(ip.join(" "))
        })
        .unwrap_or_default()
}

// ═══════════════════════════════════════════════════════════════
//  CSS value parsers (from string)
// ═══════════════════════════════════════════════════════════════

pub fn parse_length_value(s: &str) -> Option<Length> {
    let s = s.trim();
    if s == "0" {
        return Some(Length::Zero);
    }
    // Try to parse `<number><unit>`
    let (num_end, _) = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit() && *c != '.' && *c != '-')?;
    let num: f64 = s[..num_end].parse().ok()?;
    let unit = &s[num_end..];
    Some(match unit.to_ascii_lowercase().as_str() {
        "mm" => Length::Mm(num),
        "cm" => Length::Cm(num),
        "in" => Length::In(num),
        "pt" => Length::Pt(num),
        "pc" => Length::Pc(num),
        "px" => Length::Px(num),
        "em" => Length::Em(num),
        "%" => Length::Percent(num),
        _ => return None,
    })
}

pub fn parse_color_value(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.starts_with('#') {
        return Color::from_hex(s);
    }
    if s.starts_with("rgb") {
        return parse_rgb_color(s);
    }
    if s.starts_with("cmyk") || s.starts_with("device-cmyk") {
        return parse_cmyk_color(s);
    }
    Color::from_name(s)
}

fn extract_function_args(s: &str) -> Option<Vec<&str>> {
    let inner = s.split_once('(')?.1.strip_suffix(')')?.trim();
    Some(inner.split([',', ' ']).filter(|p| !p.is_empty()).collect())
}

fn parse_rgb_color(s: &str) -> Option<Color> {
    let parts = extract_function_args(s)?;
    if parts.len() < 3 {
        return None;
    }
    let r = parts[0].trim().parse().ok()?;
    let g = parts[1].trim().parse().ok()?;
    let b = parts[2].trim().parse().ok()?;
    let a = parts.get(3).and_then(|s| s.trim().parse().ok()).unwrap_or(1.0);
    Some(Color { r, g, b, a, cmyk: None })
}

fn parse_cmyk_color(s: &str) -> Option<Color> {
    let parts = extract_function_args(s)?;
    if parts.len() < 4 {
        return None;
    }
    let c = parse_cmyk_component(parts[0])?;
    let m = parse_cmyk_component(parts[1])?;
    let y = parse_cmyk_component(parts[2])?;
    let k = parse_cmyk_component(parts[3])?;
    Some(Color::cmyk(c, m, y, k))
}

fn parse_cmyk_component(s: &str) -> Option<f32> {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%') {
        let v: f32 = pct.trim().parse().ok()?;
        Some(v / 100.0)
    } else {
        let v: f32 = s.parse().ok()?;
        // Normalize: if > 1.0, treat as percentage
        Some(if v > 1.0 { v / 100.0 } else { v })
    }
}

pub fn parse_text_align_value(s: &str) -> Option<TextAlign> {
    Some(match s.trim().to_ascii_lowercase().as_str() {
        "left" => TextAlign::Left,
        "center" => TextAlign::Center,
        "right" => TextAlign::Right,
        "justify" => TextAlign::Justify,
        _ => return None,
    })
}

pub fn parse_font_weight_value(s: &str) -> Option<FontWeight> {
    Some(match s.trim().to_ascii_lowercase().as_str() {
        "bold" | "700" | "800" | "900" => FontWeight::Bold,
        "normal" | "400" | "100" | "200" | "300" => FontWeight::Normal,
        _ => return None,
    })
}

pub fn parse_font_style_value(s: &str) -> Option<FontStyle> {
    Some(match s.trim().to_ascii_lowercase().as_str() {
        "italic" | "oblique" => FontStyle::Italic,
        "normal" => FontStyle::Normal,
        _ => return None,
    })
}

pub fn parse_break_value(s: &str) -> Option<BreakValue> {
    Some(match s.trim().to_ascii_lowercase().as_str() {
        "page" | "always" => BreakValue::Page,
        "avoid" => BreakValue::Avoid,
        "auto" => BreakValue::Auto,
        _ => return None,
    })
}

pub fn parse_display_value(s: &str) -> Option<Display> {
    Some(match s.trim().to_ascii_lowercase().as_str() {
        "block" => Display::Block,
        "inline" => Display::Inline,
        "none" => Display::None,
        "list-item" => Display::ListItem,
        _ => return None,
    })
}

// ═══════════════════════════════════════════════════════════════
//  @page size / margin parsing (low-level, uses cssparser Parser)
// ═══════════════════════════════════════════════════════════════

fn parse_size(parser: &mut Parser, style: &mut PageStyle) {
    if let Ok(name) = parser.try_parse(|p| {
        let s = p.expect_ident()?.as_ref().to_owned();
        Ok::<_, BasicParseError<'_>>(s)
    }) {
        if let Some((w, h)) = named_page_size(&name) {
            let landscape = parser
                .try_parse(|p| {
                    let o = p.expect_ident()?.as_ref().to_ascii_lowercase();
                    Ok::<_, BasicParseError<'_>>(o == "landscape")
                })
                .unwrap_or(false);
            if landscape {
                style.width_mm = h;
                style.height_mm = w;
            } else {
                style.width_mm = w;
                style.height_mm = h;
            }
            return;
        }
    }
    if let Some(w) = try_length_mm(parser) {
        let h = try_length_mm(parser).unwrap_or(w);
        style.width_mm = w;
        style.height_mm = h;
    }
}

fn parse_margin(parser: &mut Parser, style: &mut PageStyle) {
    let mut values = Vec::with_capacity(4);
    for _ in 0..4 {
        match try_length_mm(parser) {
            Some(v) => values.push(v),
            None => break,
        }
    }
    match values.len() {
        1 => {
            style.margin_top_mm = values[0];
            style.margin_right_mm = values[0];
            style.margin_bottom_mm = values[0];
            style.margin_left_mm = values[0];
        }
        2 => {
            style.margin_top_mm = values[0];
            style.margin_bottom_mm = values[0];
            style.margin_right_mm = values[1];
            style.margin_left_mm = values[1];
        }
        3 => {
            style.margin_top_mm = values[0];
            style.margin_right_mm = values[1];
            style.margin_bottom_mm = values[2];
            style.margin_left_mm = values[1];
        }
        4 => {
            style.margin_top_mm = values[0];
            style.margin_right_mm = values[1];
            style.margin_bottom_mm = values[2];
            style.margin_left_mm = values[3];
        }
        _ => {}
    }
}

fn try_length_mm(parser: &mut Parser) -> Option<f64> {
    parser
        .try_parse(|p| {
            let token = p.next()?.clone();
            match token {
                Token::Dimension { value, ref unit, .. } => {
                    length_to_mm(value, unit.as_ref()).ok_or(BasicParseError {
                        kind: BasicParseErrorKind::QualifiedRuleInvalid,
                        location: SourceLocation { line: 0, column: 0 },
                    })
                }
                Token::Number { value, .. } if value == 0.0 => Ok(0.0),
                other => Err(BasicParseError {
                    kind: BasicParseErrorKind::UnexpectedToken(other),
                    location: SourceLocation { line: 0, column: 0 },
                }),
            }
        })
        .ok()
}

fn length_to_mm(value: f32, unit: &str) -> Option<f64> {
    let v = value as f64;
    Some(match unit.to_ascii_lowercase().as_str() {
        "mm" => v,
        "cm" => v * 10.0,
        "in" => v * 25.4,
        "pt" => v * 25.4 / 72.0,
        "pc" => v * 25.4 / 6.0,
        "px" => v * 25.4 / 96.0,
        _ => return None,
    })
}

fn skip_value(parser: &mut Parser) {
    while !parser.is_exhausted() {
        match parser.next() {
            Ok(Token::Semicolon) | Err(_) => break,
            _ => {}
        }
    }
}

fn skip_at_rule(parser: &mut Parser) {
    loop {
        match parser.next() {
            Ok(Token::CurlyBracketBlock) => {
                let _ = parser
                    .parse_nested_block(|_| -> Result<(), ParseError<'_, ()>> { Ok(()) });
                break;
            }
            Ok(Token::Semicolon) | Err(_) => break,
            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_a4() {
        let mut style = PageStyle::default();
        apply_page_rules("@page { size: A4; }", &mut style);
        assert!((style.width_mm - 210.0).abs() < 0.01);
        assert!((style.height_mm - 297.0).abs() < 0.01);
    }

    #[test]
    fn parse_a4_landscape() {
        let mut style = PageStyle::default();
        apply_page_rules("@page { size: A4 landscape; }", &mut style);
        assert!((style.width_mm - 297.0).abs() < 0.01);
        assert!((style.height_mm - 210.0).abs() < 0.01);
    }

    #[test]
    fn parse_margin_shorthand() {
        let mut style = PageStyle::default();
        apply_page_rules("@page { margin: 10mm 20mm; }", &mut style);
        assert!((style.margin_top_mm - 10.0).abs() < 0.01);
        assert!((style.margin_right_mm - 20.0).abs() < 0.01);
    }

    #[test]
    fn parse_margin_boxes() {
        let mut pss = PageStyleSet::default();
        let mut rules = Vec::new();
        parse_stylesheet(
            r#"@page {
                size: A4;
                @top-center { content: "Header"; font-size: 9pt; }
                @bottom-center { content: counter(page) " / " counter(pages); }
            }"#,
            &mut pss,
            &mut rules,
        );
        assert!(pss.base.margin_boxes.contains_key(&MarginBoxPosition::TopCenter));
        assert!(pss.base.margin_boxes.contains_key(&MarginBoxPosition::BottomCenter));
    }

    #[test]
    fn parse_css_rules() {
        let mut pss = PageStyleSet::default();
        let mut rules = Vec::new();
        parse_stylesheet(
            "h1 { font-size: 24pt; color: navy; } .note { font-size: 9pt; }",
            &mut pss,
            &mut rules,
        );
        assert_eq!(rules.len(), 2);
        assert!(matches!(rules[0].selectors[0].subject(), SimpleSelector::Type(t) if t == "h1"));
        assert!(matches!(rules[1].selectors[0].subject(), SimpleSelector::Class(c) if c == "note"));
    }

    #[test]
    fn parse_content_items() {
        let items = parse_content_value(r#""Chapter " counter(page) " of " counter(pages)"#);
        assert_eq!(items.len(), 4);
        assert!(matches!(&items[0], ContentItem::String(s) if s == "Chapter "));
        assert!(matches!(&items[1], ContentItem::Counter(c) if c == "page"));
    }

    #[test]
    fn parse_first_page_override() {
        let mut pss = PageStyleSet::default();
        let mut rules = Vec::new();
        parse_stylesheet(
            r#"
            @page {
                @top-center { content: "Header"; }
            }
            @page :first {
                @top-center { content: none; }
            }"#,
            &mut pss,
            &mut rules,
        );
        assert!(pss.base.margin_boxes.contains_key(&MarginBoxPosition::TopCenter));
        assert!(pss.first.is_some());
        assert!(pss.first.as_ref().unwrap().suppress_boxes.contains(&MarginBoxPosition::TopCenter));
    }
}
