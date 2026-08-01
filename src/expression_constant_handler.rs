//! Compile constants in expressions to their literal values.

use anyhow::Result;

use crate::ast::{BinaryOp, Expr, UnaryOp};

macro_rules! bin_op {
    (match $op_var:ident, $left_var:ident, $right_var:ident => $(op $op:pat, $left:pat, $right:pat => $result:block),+) => {
        match (*$op_var, $left_var.clone(), $right_var.clone()) {
                $(($op, $left, $right) => {
                    $result
                },)+
                _ => Expr::Binary { op: *$op_var, left: Box::new($left_var), right: Box::new($right_var) },
            }
    };
}

pub(crate) fn combine_expression(e: &Expr) -> Result<Expr> {
    if e.is_literal() {
        Ok(e.clone())
    } else {
        match e {
            Expr::Unary { op, value } => {
                let lowered_value = combine_expression(&*value)?;
                match lowered_value.is_literal() {
                    true => {
                        // If the operand is a literal, we can combine the unary operation with it.
                        Ok(match (*op, lowered_value.clone()) {
                            (UnaryOp::Plus, Expr::Integer(i)) => Expr::Integer(i),
                            (UnaryOp::Minus, Expr::Integer(i)) => Expr::Integer(-i),
                            (UnaryOp::Plus, Expr::Real(f)) => Expr::Real(f),
                            (UnaryOp::Minus, Expr::Real(f)) => Expr::Real(-f),
                            (UnaryOp::Plus, Expr::LongReal(f)) => Expr::LongReal(f),
                            (UnaryOp::Minus, Expr::LongReal(f)) => Expr::LongReal(-f),
                            (UnaryOp::Not, Expr::Boolean(b)) => Expr::Boolean(!b),
                            _ => {
                                panic!("Unsupported unary operation: {:?} {:?}", op, lowered_value)
                            }
                        })
                    }
                    false => {
                        // If the operand is not a literal, we cannot combine it.
                        Ok(Expr::Unary {
                            op: *op,
                            value: Box::new(lowered_value),
                        })
                    }
                }
            }
            Expr::Binary { op, left, right } => {
                let lowered_left = combine_expression(&*left)?;
                let lowered_right = combine_expression(&*right)?;
                if lowered_left.is_literal() && lowered_right.is_literal() {
                    // If both operands are literals, we can combine the binary operation with them.
                    let res = bin_op! {
                    match op, lowered_left, lowered_right =>
                        // additions: all combinations of integer, real, and long real types
                        op BinaryOp::Add, Expr::Integer(l), Expr::Integer(r) => { Expr::Integer(l + r) },
                        op BinaryOp::Add, Expr::Real(l), Expr::Real(r) => { Expr::Real(l + r) },
                        op BinaryOp::Add, Expr::LongReal(l), Expr::LongReal(r) => { Expr::LongReal(l + r) },
                        op BinaryOp::Add, Expr::Integer(l), Expr::Real(r) => { Expr::Real(l as f32 + r) },
                        op BinaryOp::Add, Expr::Real(l), Expr::Integer(r) => { Expr::Real(l + r as f32) },
                        op BinaryOp::Add, Expr::Integer(l), Expr::LongReal(r) => { Expr::LongReal(l as f64 + r) },
                        op BinaryOp::Add, Expr::LongReal(l), Expr::Integer(r) => { Expr::LongReal(l + r as f64) },

                        // subtractions: all combinations of integer, real, and long real types
                        op BinaryOp::Sub, Expr::Integer(l), Expr::Integer(r) => { Expr::Integer(l - r) },
                        op BinaryOp::Sub, Expr::Real(l), Expr::Real(r) => { Expr::Real(l - r) },
                        op BinaryOp::Sub, Expr::LongReal(l), Expr::LongReal(r) => { Expr::LongReal(l - r) },
                        op BinaryOp::Sub, Expr::Integer(l), Expr::Real(r) => { Expr::Real(l as f32 - r) },
                        op BinaryOp::Sub, Expr::Real(l), Expr::Integer(r) => { Expr::Real(l - r as f32) },
                        op BinaryOp::Sub, Expr::Integer(l), Expr::LongReal(r) => { Expr::LongReal(l as f64 - r) },
                        op BinaryOp::Sub, Expr::LongReal(l), Expr::Integer(r) => { Expr::LongReal(l - r as f64) },

                        // multiplications: all combinations of integer, real, and long real types
                        op BinaryOp::Mul, Expr::Integer(l), Expr::Integer(r) => { Expr::Integer(l * r) },
                        op BinaryOp::Mul, Expr::Real(l), Expr::Real(r) => { Expr::Real(l * r) },
                        op BinaryOp::Mul, Expr::LongReal(l), Expr::LongReal(r) => { Expr::LongReal(l * r) },
                        op BinaryOp::Mul, Expr::Integer(l), Expr::Real(r) => { Expr::Real(l as f32 * r) },
                        op BinaryOp::Mul, Expr::Real(l), Expr::Integer(r) => { Expr::Real(l * r as f32) },
                        op BinaryOp::Mul, Expr::Integer(l), Expr::LongReal(r) => { Expr::LongReal(l as f64 * r) },
                        op BinaryOp::Mul, Expr::LongReal(l), Expr::Integer(r) => { Expr::LongReal(l * r as f64) },

                        // divisions: all combinations of integer, real, and long real types
                        op BinaryOp::Div, Expr::Integer(l), Expr::Integer(r) => {
                            if r == 0 {
                                panic!("Division by zero");
                            }
                            Expr::Integer(l / r)
                        },
                        op BinaryOp::IntDiv, Expr::Integer(l), Expr::Integer(r) => {
                            if r == 0 {
                                panic!("Division by zero");
                            }
                            Expr::Integer(l / r)
                        },
                        op BinaryOp::Div, Expr::Real(l), Expr::Real(r) => {
                            if r <= f32::EPSILON {
                                panic!("Division by zero");
                            }
                            Expr::Real(l / r)
                        },
                        op BinaryOp::Div, Expr::LongReal(l), Expr::LongReal(r) => {
                            if r <= f64::EPSILON {
                                panic!("Division by zero");
                            }
                            Expr::LongReal(l / r)
                        },
                        op BinaryOp::Div, Expr::Integer(l), Expr::Real(r) => {
                            if r <= f32::EPSILON {
                                panic!("Division by zero");
                            }
                            Expr::Real(l as f32 / r)
                        },
                        op BinaryOp::Div, Expr::Real(l), Expr::Integer(r) => {
                            if r == 0 {
                                panic!("Division by zero");
                            }
                            Expr::Real(l / r as f32)
                        },
                        op BinaryOp::Div, Expr::Integer(l), Expr::LongReal(r) => {
                            if r <= f64::EPSILON {
                                panic!("Division by zero");
                            }
                            Expr::LongReal(l as f64 / r)
                        },
                        op BinaryOp::Div, Expr::LongReal(l), Expr::Integer(r) => {
                            if r == 0 {
                                panic!("Division by zero");
                            }
                            Expr::LongReal(l / r as f64)
                        },

                        // modulus: only defined for integers
                        op BinaryOp::Mod, Expr::Integer(l), Expr::Integer(r) => {
                            if r == 0 {
                                panic!("Division by zero");
                            }
                            Expr::Integer(l % r)
                        },

                        // logical AND and OR: only defined for booleans
                        op BinaryOp::And, Expr::Boolean(l), Expr::Boolean(r) => { Expr::Boolean(l && r) },
                        op BinaryOp::Or, Expr::Boolean(l), Expr::Boolean(r) => { Expr::Boolean(l || r) },

                        // equality: can compare any two literals of the same type
                        op BinaryOp::Eq, l, r => {
                            if std::mem::discriminant(&l) != std::mem::discriminant(&r) {
                                panic!("Cannot compare different types for equality");
                            }
                            match l {
                                Expr::Integer(li) => match r {
                                    Expr::Integer(ri) => Expr::Boolean(li == ri),
                                    _ => unreachable!(),
                                },
                                Expr::Real(lf) => match r {
                                    Expr::Real(rf) => Expr::Boolean((lf - rf).abs() < f32::EPSILON),
                                    _ => unreachable!(),
                                },
                                Expr::LongReal(lf) => match r {
                                    Expr::LongReal(rf) => Expr::Boolean((lf - rf).abs() < f64::EPSILON),
                                    _ => unreachable!(),
                                },
                                Expr::Boolean(lb) => match r {
                                    Expr::Boolean(rb) => Expr::Boolean(lb == rb),
                                    _ => unreachable!(),
                                },
                                Expr::String(ls) => match r {
                                    Expr::String(rs) => Expr::Boolean(ls == rs),
                                    _ => unreachable!(),
                                },
                                _ => unreachable!(),
                            }
                        }
                    };
                    Ok(res)
                } else {
                    Ok(Expr::Binary {
                        op: *op,
                        left: Box::new(lowered_left),
                        right: Box::new(lowered_right),
                    })
                }
            }
            _ => Ok(e.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
