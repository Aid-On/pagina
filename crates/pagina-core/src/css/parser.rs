use cssparser::{BasicParseError, BasicParseErrorKind, ParseError, Parser, ParserInput, SourceLocation, Token};

use super::{named_page_size, PageStyle};

/// Walk a stylesheet and apply any `@page` declarations to `style`.
pub fn apply_page_rules(css: &str, style: &mut PageStyle) {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);

    loop {
        let token_kind = {
            match parser.next() {
                Ok(token) => {
                    if matches!(token, Token::AtKeyword(kw) if kw.eq_ignore_ascii_case("page")) {
                        1 // @page keyword
                    } else if matches!(token, Token::CurlyBracketBlock) {
                        2 // curly bracket block
                    } else {
                        0 // other
                    }
                }
                Err(_) => break,
            }
        };

        match token_kind {
            1 => {
                // Found @page; skip optional page selector tokens, then expect CurlyBracketBlock
                let found_block = loop {
                    match parser.next() {
                        Ok(t) if matches!(t, Token::CurlyBracketBlock) => break true,
                        Ok(_) => continue,
                        Err(_) => break false,
                    }
                };
                if found_block {
                    let _ = parser.parse_nested_block(
                        |block| -> Result<(), ParseError<'_, ()>> {
                            parse_declarations(block, style);
                            Ok(())
                        },
                    );
                }
            }
            2 => {
                // Stray block — skip its contents
                let _ = parser.parse_nested_block(
                    |_| -> Result<(), ParseError<'_, ()>> { Ok(()) },
                );
            }
            _ => {}
        }
    }
}

fn parse_declarations(parser: &mut Parser, style: &mut PageStyle) {
    while !parser.is_exhausted() {
        let name = match parser.expect_ident() {
            Ok(n) => n.as_ref().to_ascii_lowercase(),
            Err(_) => {
                let _ = parser.next();
                continue;
            }
        };

        if parser.expect_colon().is_err() {
            continue;
        }

        match name.as_str() {
            "size" => parse_size(parser, style),
            "margin" => parse_margin(parser, style),
            "margin-top" => {
                if let Some(v) = try_length_mm(parser) {
                    style.margin_top_mm = v;
                }
            }
            "margin-right" => {
                if let Some(v) = try_length_mm(parser) {
                    style.margin_right_mm = v;
                }
            }
            "margin-bottom" => {
                if let Some(v) = try_length_mm(parser) {
                    style.margin_bottom_mm = v;
                }
            }
            "margin-left" => {
                if let Some(v) = try_length_mm(parser) {
                    style.margin_left_mm = v;
                }
            }
            _ => skip_value(parser),
        }

        let _ = parser.try_parse(|p| p.expect_semicolon());
    }
}

// ── size ──────────────────────────────────────────────

fn parse_size(parser: &mut Parser, style: &mut PageStyle) {
    // Try named size (e.g. `A4`, `letter`)
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

    // Try explicit lengths: `size: 210mm 297mm`
    if let Some(w) = try_length_mm(parser) {
        let h = try_length_mm(parser).unwrap_or(w);
        style.width_mm = w;
        style.height_mm = h;
    }
}

// ── margin ────────────────────────────────────────────

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

// ── helpers ───────────────────────────────────────────

fn try_length_mm(parser: &mut Parser) -> Option<f64> {
    parser
        .try_parse(|p| {
            let token = p.next()?.clone();
            match token {
                Token::Dimension { value, ref unit, .. } => {
                    to_mm(value, unit.as_ref()).ok_or(BasicParseError {
                        kind: BasicParseErrorKind::QualifiedRuleInvalid,
                        location: SourceLocation { line: 0, column: 0 },
                    })
                }
                other => Err(BasicParseError {
                    kind: BasicParseErrorKind::UnexpectedToken(other),
                    location: SourceLocation { line: 0, column: 0 },
                }),
            }
        })
        .ok()
}

fn to_mm(value: f32, unit: &str) -> Option<f64> {
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
    fn parse_letter() {
        let mut style = PageStyle::default();
        apply_page_rules("@page { size: letter; }", &mut style);
        assert!((style.width_mm - 215.9).abs() < 0.01);
        assert!((style.height_mm - 279.4).abs() < 0.01);
    }

    #[test]
    fn parse_margin_shorthand() {
        let mut style = PageStyle::default();
        apply_page_rules("@page { margin: 10mm 20mm; }", &mut style);
        assert!((style.margin_top_mm - 10.0).abs() < 0.01);
        assert!((style.margin_right_mm - 20.0).abs() < 0.01);
        assert!((style.margin_bottom_mm - 10.0).abs() < 0.01);
        assert!((style.margin_left_mm - 20.0).abs() < 0.01);
    }

    #[test]
    fn parse_custom_size() {
        let mut style = PageStyle::default();
        apply_page_rules("@page { size: 100mm 200mm; }", &mut style);
        assert!((style.width_mm - 100.0).abs() < 0.01);
        assert!((style.height_mm - 200.0).abs() < 0.01);
    }
}
