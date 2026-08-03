
use super::{HExpr, HResolvedIdent};
use crate::ast::{BinaryOp, UnaryOp};
use crate::symbols::SymbolKind;

#[test]
fn h_expr_is_literal_detects_literal_and_non_literal_variants() {
    assert!(HExpr::Integer(7).is_literal());
    assert!(HExpr::Real(2.5).is_literal());
    assert!(HExpr::LongReal(3.25).is_literal());
    assert!(HExpr::Boolean(true).is_literal());
    assert!(HExpr::String("hi".to_string()).is_literal());

    assert!(
        !HExpr::Name(HResolvedIdent {
            id: 1,
            name: "value".to_string(),
            kind: SymbolKind::Variable,
        })
        .is_literal()
    );
    assert!(
        !HExpr::Indexed {
            name: HResolvedIdent {
                id: 1,
                name: "value".to_string(),
                kind: SymbolKind::Variable,
            },
            index: Box::new(HExpr::Integer(0)),
        }
        .is_literal()
    );
    assert!(
        !HExpr::Call {
            name: HResolvedIdent {
                id: 2,
                name: "f".to_string(),
                kind: SymbolKind::Procedure,
            },
            args: vec![],
        }
        .is_literal()
    );
    assert!(
        !HExpr::Unary {
            op: UnaryOp::Not,
            value: Box::new(HExpr::Boolean(true)),
        }
        .is_literal()
    );
    assert!(
        !HExpr::Binary {
            op: BinaryOp::Add,
            left: Box::new(HExpr::Integer(1)),
            right: Box::new(HExpr::Integer(2)),
        }
        .is_literal()
    );
}

#[test]
fn h_expr_to_string_formats_names_calls_unary_and_binary_exprs() {
    let ident = HResolvedIdent {
        id: 10,
        name: "value".to_string(),
        kind: SymbolKind::Variable,
    };

    assert_eq!(HExpr::Integer(5).to_string(), "5");
    assert_eq!(HExpr::Boolean(false).to_string(), "false");
    assert_eq!(HExpr::Name(ident.clone()).to_string(), "value");

    let call = HExpr::Call {
        name: HResolvedIdent {
            id: 11,
            name: "f".to_string(),
            kind: SymbolKind::Procedure,
        },
        args: vec![HExpr::Integer(1), HExpr::Integer(2)],
    };
    assert_eq!(call.to_string(), "f(1, 2)");

    let unary = HExpr::Unary {
        op: UnaryOp::Not,
        value: Box::new(HExpr::Boolean(true)),
    };
    assert_eq!(unary.to_string(), "NOT true");

    let binary = HExpr::Binary {
        op: BinaryOp::Add,
        left: Box::new(HExpr::Integer(1)),
        right: Box::new(HExpr::Integer(2)),
    };
    assert_eq!(binary.to_string(), "(1 + 2)");
}
