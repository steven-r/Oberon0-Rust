use anyhow::{Result, bail};
use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum Token {
    #[token("MODULE")]
    KwModule,
    #[token("IMPORT")]
    KwImport,
    #[token("BEGIN")]
    KwBegin,
    #[token("END")]
    KwEnd,

    #[token(":=")]
    Assign,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
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

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Integer(i64),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SpannedToken {
    pub token: Token,
    pub start: usize,
    pub end: usize,
}

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
