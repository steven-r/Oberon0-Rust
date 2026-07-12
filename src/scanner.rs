use anyhow::{Result, bail};
use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
/// Lexical tokens recognized by the Oberon0 scanner.
pub enum Token {
    #[token("MODULE")]
    KwModule,
    #[token("IMPORT")]
    KwImport,
    #[token("CONST")]
    KwConst,
    #[token("TYPE")]
    KwType,
    #[token("VAR")]
    KwVar,
    #[token("PROCEDURE")]
    KwProcedure,
    #[token("BEGIN")]
    KwBegin,
    #[token("END")]
    KwEnd,
    #[token("IF")]
    KwIf,
    #[token("THEN")]
    KwThen,
    #[token("ELSE")]
    KwElse,
    #[token("WHILE")]
    KwWhile,
    #[token("DO")]
    KwDo,
    #[token("OR")]
    KwOr,
    #[token("DIV")]
    OpDiv,
    #[token("MOD")]
    OpMod,

    #[token(":=")]
    Assign,
    #[token("=")]
    Equal,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("#")]
    Hash,
    #[token("<=")]
    LessEqual,
    #[token("<")]
    Less,
    #[token(">=")]
    GreaterEqual,
    #[token(">")]
    Greater,
    #[token("&")]
    Ampersand,
    #[token("~")]
    Tilde,

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Integer(i64),

    #[regex(r#"\"([^\"\n]|\"\")*\""#, parse_pascal_string)]
    String(String),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
/// Token annotated with its byte span in the original source file.
pub struct SpannedToken {
    /// Token kind and payload.
    pub token: Token,
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

/// Converts raw source text into a stream of spanned tokens.
pub fn scan(source: &str) -> Result<Vec<SpannedToken>> {
    let mut lexer = Token::lexer(source);
    let mut out = Vec::new();

    while let Some(item) = lexer.next() {
        let span = lexer.span();
        match item {
            Ok(token) => out.push(SpannedToken {
                token,
                start: span.start,
                end: span.end,
            }),
            Err(_) => {
                let near = source.get(span.clone()).unwrap_or("");
                bail!("Unknown token at byte {}: '{}'", span.start, near);
            }
        }
    }

    Ok(out)
}

fn parse_pascal_string(lex: &mut logos::Lexer<'_, Token>) -> Option<String> {
    unescape_pascal_string(lex.slice()).ok()
}

fn unescape_pascal_string(raw: &str) -> Result<String> {
    if raw.len() < 2 || !raw.starts_with('"') || !raw.ends_with('"') {
        bail!("Invalid string literal: {}", raw);
    }

    let inner = &raw[1..raw.len() - 1];
    Ok(inner.replace("\"\"", "\""))
}

#[cfg(test)]
mod tests {
    use super::{Token, scan};

    #[test]
    fn scans_const_declaration_with_equal_token() {
        let source = "MODULE Main; CONST BASE = 10; BEGIN END Main.";
        let tokens = scan(source).expect("scanner should accept CONST declaration syntax");

        let has_equal = tokens.iter().any(|t| matches!(t.token, Token::Equal));
        assert!(has_equal, "scanner output should contain '=' token");
    }

    #[test]
    fn scans_control_flow_and_procedure_keywords() {
        let source = "PROCEDURE P(x); BEGIN IF x THEN WHILE x DO x := x - 1 END END END P;";
        let tokens = scan(source).expect("scanner should accept procedure and control-flow syntax");

        let has_procedure = tokens.iter().any(|t| matches!(t.token, Token::KwProcedure));
        let has_if = tokens.iter().any(|t| matches!(t.token, Token::KwIf));
        let has_then = tokens.iter().any(|t| matches!(t.token, Token::KwThen));
        let has_while = tokens.iter().any(|t| matches!(t.token, Token::KwWhile));
        let has_do = tokens.iter().any(|t| matches!(t.token, Token::KwDo));

        assert!(has_procedure, "scanner should tokenize PROCEDURE as a keyword");
        assert!(has_if, "scanner should tokenize IF as a keyword");
        assert!(has_then, "scanner should tokenize THEN as a keyword");
        assert!(has_while, "scanner should tokenize WHILE as a keyword");
        assert!(has_do, "scanner should tokenize DO as a keyword");
    }

    #[test]
    fn scans_extended_operator_tokens() {
        let source = "MODULE Main; BEGIN x := +a - ~b OR c DIV 2 MOD 3 & d / 4 * 5; IF x # 0 THEN x := (x <= 10) + (x >= 1) + (x < 11) + (x > 0) END; END Main.";
        let tokens = scan(source).expect("scanner should accept extended operator syntax");

        assert!(tokens.iter().any(|t| matches!(t.token, Token::KwOr)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::OpDiv)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::OpMod)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::Ampersand)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::Tilde)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::Plus)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::Minus)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::Hash)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::LessEqual)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::GreaterEqual)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::Less)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::Greater)));
    }

    #[test]
    fn scans_declaration_keywords_as_keywords() {
        let source = "MODULE Main; CONST BASE = 10; TYPE Count = INTEGER; VAR x: Count; BEGIN END Main.";
        let tokens = scan(source).expect("scanner should accept declaration keywords");

        assert!(tokens.iter().any(|t| matches!(t.token, Token::KwModule)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::KwConst)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::KwType)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::KwVar)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::Colon)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::KwBegin)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::KwEnd)));
    }

    #[test]
    fn scans_pascal_style_string_literals() {
        let source = "MODULE Main; BEGIN WriteString(\"Hello, \"\"Oberon\"\"\"); END Main.";
        let tokens = scan(source).expect("scanner should accept string literal syntax");

        let string_token = tokens
            .iter()
            .find_map(|t| match &t.token {
                Token::String(value) => Some(value.clone()),
                _ => None,
            })
            .expect("scanner should emit a string token");

        assert_eq!(string_token, "Hello, \"Oberon\"");
    }
}
