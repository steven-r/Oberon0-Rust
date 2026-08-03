
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

    assert!(
        has_procedure,
        "scanner should tokenize PROCEDURE as a keyword"
    );
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
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t.token, Token::GreaterEqual))
    );
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Less)));
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Greater)));
}

#[test]
fn scans_declaration_keywords_as_keywords() {
    let source =
        "MODULE Main; CONST BASE = 10; TYPE Count = INTEGER; VAR x: Count; BEGIN END Main.";
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
