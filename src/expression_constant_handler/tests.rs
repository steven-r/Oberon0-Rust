use super::*;
use crate::ast::{BinaryOp, Expr, UnaryOp};
use rstest::rstest;

#[test]
#[should_panic(expected = "Unsupported unary operation: Minus String(\"test\")")]
fn test_unary_panic() {
    let expr = Expr::Unary {
        op: UnaryOp::Minus,
        value: Box::new(Expr::String("test".to_string())),
    };
    combine_expression(&expr).unwrap();
}

#[rstest]
#[case(UnaryOp::Minus, Expr::Integer(5), Expr::Integer(-5))]
#[case(UnaryOp::Minus, Expr::Real(5.0), Expr::Real(-5.0))]
#[case(UnaryOp::Minus, Expr::LongReal(5.0), Expr::LongReal(-5.0))]
#[case(UnaryOp::Plus, Expr::Integer(5), Expr::Integer(5))]
#[case(UnaryOp::Plus, Expr::Real(5.0), Expr::Real(5.0))]
#[case(UnaryOp::Plus, Expr::LongReal(5.0), Expr::LongReal(5.0))]
#[case(UnaryOp::Not, Expr::Boolean(true), Expr::Boolean(false))]
#[case(UnaryOp::Not, Expr::Variable("x".to_string()), Expr::Unary { op: UnaryOp::Not, value: Box::new(Expr::Variable("x".to_string())) })]
fn test_unary(#[case] op: UnaryOp, #[case] input: Expr, #[case] expected: Expr) {
    let expr = Expr::Unary {
        op,
        value: Box::new(input),
    };
    let combined = combine_expression(&expr).unwrap();
    assert_eq!(combined, expected);
}

#[rstest]
// addition cases
#[case(BinaryOp::Add, Expr::Integer(2), Expr::Integer(3), Expr::Integer(5))]
#[case(BinaryOp::Add, Expr::Real(2.0), Expr::Real(3.0), Expr::Real(5.0))]
#[case(BinaryOp::Add, Expr::Real(2.0), Expr::Integer(3), Expr::Real(5.0))]
#[case(BinaryOp::Add, Expr::Integer(2), Expr::Real(3.0), Expr::Real(5.0))]
#[case(
    BinaryOp::Add,
    Expr::LongReal(2.0),
    Expr::LongReal(3.0),
    Expr::LongReal(5.0)
)]
#[case(
    BinaryOp::Add,
    Expr::LongReal(2.0),
    Expr::Integer(3),
    Expr::LongReal(5.0)
)]
#[case(
    BinaryOp::Add,
    Expr::Integer(2),
    Expr::LongReal(3.0),
    Expr::LongReal(5.0)
)]
// subtraction cases
#[case(BinaryOp::Sub, Expr::Integer(5), Expr::Integer(3), Expr::Integer(2))]
#[case(BinaryOp::Sub, Expr::Real(5.0), Expr::Real(3.0), Expr::Real(2.0))]
#[case(BinaryOp::Sub, Expr::Real(5.0), Expr::Integer(3), Expr::Real(2.0))]
#[case(BinaryOp::Sub, Expr::Integer(5), Expr::Real(3.0), Expr::Real(2.0))]
#[case(
    BinaryOp::Sub,
    Expr::LongReal(5.0),
    Expr::LongReal(3.0),
    Expr::LongReal(2.0)
)]
#[case(
    BinaryOp::Sub,
    Expr::LongReal(5.0),
    Expr::Integer(3),
    Expr::LongReal(2.0)
)]
#[case(
    BinaryOp::Sub,
    Expr::Integer(5),
    Expr::LongReal(3.0),
    Expr::LongReal(2.0)
)]
// multiplication cases
#[case(BinaryOp::Mul, Expr::Integer(2), Expr::Integer(3), Expr::Integer(6))]
#[case(BinaryOp::Mul, Expr::Real(2.0), Expr::Real(3.0), Expr::Real(6.0))]
#[case(BinaryOp::Mul, Expr::Real(2.0), Expr::Integer(3), Expr::Real(6.0))]
#[case(BinaryOp::Mul, Expr::Integer(2), Expr::Real(3.0), Expr::Real(6.0))]
#[case(
    BinaryOp::Mul,
    Expr::LongReal(2.0),
    Expr::LongReal(3.0),
    Expr::LongReal(6.0)
)]
#[case(
    BinaryOp::Mul,
    Expr::LongReal(2.0),
    Expr::Integer(3),
    Expr::LongReal(6.0)
)]
#[case(
    BinaryOp::Mul,
    Expr::Integer(2),
    Expr::LongReal(3.0),
    Expr::LongReal(6.0)
)]
// division cases
#[case(BinaryOp::Div, Expr::Integer(6), Expr::Integer(3), Expr::Integer(2))]
#[case(BinaryOp::Div, Expr::Real(6.0), Expr::Real(3.0), Expr::Real(2.0))]
#[case(BinaryOp::Div, Expr::Real(6.0), Expr::Integer(3), Expr::Real(2.0))]
#[case(BinaryOp::Div, Expr::Integer(6), Expr::Real(3.0), Expr::Real(2.0))]
#[case(
    BinaryOp::Div,
    Expr::LongReal(6.0),
    Expr::LongReal(3.0),
    Expr::LongReal(2.0)
)]
#[case(
    BinaryOp::Div,
    Expr::LongReal(6.0),
    Expr::Integer(3),
    Expr::LongReal(2.0)
)]
#[case(
    BinaryOp::Div,
    Expr::Integer(6),
    Expr::LongReal(3.0),
    Expr::LongReal(2.0)
)]
#[case(BinaryOp::Mod, Expr::Integer(7), Expr::Integer(3), Expr::Integer(1))]
#[case(BinaryOp::IntDiv, Expr::Integer(7), Expr::Integer(3), Expr::Integer(2))]
#[case(
    BinaryOp::And,
    Expr::Boolean(true),
    Expr::Boolean(false),
    Expr::Boolean(false)
)]
#[case(
    BinaryOp::Or,
    Expr::Boolean(true),
    Expr::Boolean(false),
    Expr::Boolean(true)
)]
// equality cases
#[case(BinaryOp::Eq, Expr::Integer(5), Expr::Integer(5), Expr::Boolean(true))]
#[case(BinaryOp::Eq, Expr::Integer(5), Expr::Integer(3), Expr::Boolean(false))]
#[case(BinaryOp::Eq, Expr::Real(5.0), Expr::Real(5.0), Expr::Boolean(true))]
#[case(BinaryOp::Eq, Expr::Real(5.0), Expr::Real(3.0), Expr::Boolean(false))]
#[case(
    BinaryOp::Eq,
    Expr::LongReal(5.0),
    Expr::LongReal(5.0),
    Expr::Boolean(true)
)]
#[case(
    BinaryOp::Eq,
    Expr::LongReal(5.0),
    Expr::LongReal(3.0),
    Expr::Boolean(false)
)]
#[case(
    BinaryOp::Eq,
    Expr::Boolean(true),
    Expr::Boolean(true),
    Expr::Boolean(true)
)]
#[case(
    BinaryOp::Eq,
    Expr::Boolean(true),
    Expr::Boolean(false),
    Expr::Boolean(false)
)]
#[case(BinaryOp::Eq, Expr::String("test".to_string()), Expr::String("test".to_string()), Expr::Boolean(true))]
#[case(BinaryOp::Eq, Expr::String("test".to_string()), Expr::String("other".to_string()), Expr::Boolean(false))]
// non-literal cases
#[case(BinaryOp::Add, Expr::Integer(2), Expr::Variable("x".to_string()), Expr::Binary { op: BinaryOp::Add, left: Box::new(Expr::Integer(2)), right: Box::new(Expr::Variable("x".to_string())) })]
fn test_binary(
    #[case] op: BinaryOp,
    #[case] left: Expr,
    #[case] right: Expr,
    #[case] expected: Expr,
) {
    let expr = Expr::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
    };
    let combined = combine_expression(&expr).unwrap();
    assert_eq!(combined, expected);
}

#[rstest]
#[case(BinaryOp::Div, Expr::Integer(1), Expr::Integer(0))]
#[case(BinaryOp::Div, Expr::Real(1.0), Expr::Real(0.0))]
#[case(BinaryOp::Div, Expr::Real(1.0), Expr::Real(f32::EPSILON))]
#[case(BinaryOp::Div, Expr::Real(1.0), Expr::Integer(0))]
#[case(BinaryOp::Div, Expr::LongReal(1.0), Expr::LongReal(0.0))]
#[case(BinaryOp::Div, Expr::LongReal(1.0), Expr::Integer(0))]
#[case(BinaryOp::Div, Expr::Integer(1), Expr::LongReal(0.0))]
#[case(BinaryOp::Div, Expr::Integer(1), Expr::Real(0.0))]
#[case(BinaryOp::IntDiv, Expr::Integer(1), Expr::Integer(0))]
#[case(BinaryOp::Mod, Expr::Integer(1), Expr::Integer(0))]
#[case(BinaryOp::Eq, Expr::Integer(5), Expr::Real(5.0))] // comparing different types should panic
fn test_binary_panic(#[case] op: BinaryOp, #[case] left: Expr, #[case] right: Expr) {
    let expr = Expr::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
    };
    let result = std::panic::catch_unwind(|| {
        combine_expression(&expr).unwrap();
    });
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        panic!("Panic occurred, but could not extract error message.");
    };
    assert!(
        err_msg.contains("Division by zero")
            || err_msg.contains("Cannot compare different types for equality")
    );
}
