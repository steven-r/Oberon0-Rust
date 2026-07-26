//! Compile constants in expressions to their literal values.

use anyhow::Result;

use crate::ast::{Expr, UnaryOp, BinaryOp};

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
                            _ => Expr::Unary { op: *op, value: Box::new(lowered_value) },
                        })
                    }
                    false => {
                        // If the operand is not a literal, we cannot combine it.
                        Ok(Expr::Unary { op: *op, value: Box::new(lowered_value) })
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
                    Ok(Expr::Binary { op: *op, left: Box::new(lowered_left), right: Box::new(lowered_right) })
                }
            }
            _ => Ok(e.clone()),
        }
    }
}
