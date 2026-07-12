use anyhow::{Result, bail};
use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum Token {
    #[token("MODULE")]
    KwModule,
    #[token("IMPORT")]
    KwImport,
    #[token("CONST")]
    KwConst,
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

    #[token(":=")]
    Assign,
    #[token("=")]
    Equal,
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
    fn scans_declaration_keywords_as_keywords() {
        let source = "MODULE Main; CONST BASE = 10; VAR x; BEGIN END Main.";
        let tokens = scan(source).expect("scanner should accept declaration keywords");

        assert!(tokens.iter().any(|t| matches!(t.token, Token::KwModule)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::KwConst)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::KwVar)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::KwBegin)));
        assert!(tokens.iter().any(|t| matches!(t.token, Token::KwEnd)));
    }
}
