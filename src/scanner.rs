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
    #[token("ARRAY")]
    KwArray,
    #[token("OF")]
    KwOf,
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
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
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
mod tests;
