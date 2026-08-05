use super::{AssignTarget, BinaryOp, Expr, UnaryOp};

#[test]
fn expr_is_literal_detects_literal_and_non_literal_nodes() {
    assert!(Expr::Integer(42).is_literal());
    assert!(Expr::Real(3.5).is_literal());
    assert!(Expr::LongReal(4.5).is_literal());
    assert!(Expr::Boolean(true).is_literal());
    assert!(Expr::String("hello".to_string()).is_literal());

    assert!(!Expr::Variable("value".to_string()).is_literal());
    assert!(
        !Expr::Indexed {
            name: "value".to_string(),
            index: Box::new(Expr::Integer(0)),
        }
        .is_literal()
    );
    assert!(
        !Expr::QualifiedVariable {
            module: "M".to_string(),
            name: "value".to_string(),
        }
        .is_literal()
    );
    assert!(
        !Expr::Call {
            module: None,
            name: "f".to_string(),
            args: vec![],
        }
        .is_literal()
    );
    assert!(
        !Expr::Unary {
            op: UnaryOp::Not,
            value: Box::new(Expr::Boolean(true)),
        }
        .is_literal()
    );
    assert!(
        !Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Integer(1)),
            right: Box::new(Expr::Integer(2)),
        }
        .is_literal()
    );
}

#[test]
fn expr_equality_matches_all_supported_expression_variants() {
    assert_eq!(Expr::Integer(1), Expr::Integer(1));
    assert_ne!(Expr::Integer(1), Expr::Integer(2));
    assert_eq!(Expr::Real(1.5), Expr::Real(1.5));
    assert_ne!(Expr::Real(1.5), Expr::Real(2.5));
    assert_eq!(Expr::LongReal(1.25), Expr::LongReal(1.25));
    assert_ne!(Expr::LongReal(1.25), Expr::LongReal(2.25));
    assert_eq!(Expr::Boolean(true), Expr::Boolean(true));
    assert_ne!(Expr::Boolean(true), Expr::Boolean(false));
    assert_eq!(Expr::String("x".to_string()), Expr::String("x".to_string()));
    assert_ne!(Expr::String("x".to_string()), Expr::String("y".to_string()));

    assert_eq!(
        Expr::Variable("name".to_string()),
        Expr::Variable("name".to_string())
    );
    assert_ne!(
        Expr::Variable("name".to_string()),
        Expr::Variable("other".to_string())
    );

    assert_eq!(
        Expr::Indexed {
            name: "values".to_string(),
            index: Box::new(Expr::Integer(0)),
        },
        Expr::Indexed {
            name: "values".to_string(),
            index: Box::new(Expr::Integer(0)),
        }
    );
    assert_ne!(
        Expr::Indexed {
            name: "values".to_string(),
            index: Box::new(Expr::Integer(0)),
        },
        Expr::Indexed {
            name: "values".to_string(),
            index: Box::new(Expr::Integer(1)),
        }
    );

    assert_eq!(
        Expr::Field {
            name: "point".to_string(),
            field: "x".to_string(),
        },
        Expr::Field {
            name: "point".to_string(),
            field: "x".to_string(),
        }
    );
    assert_ne!(
        Expr::Field {
            name: "point".to_string(),
            field: "x".to_string(),
        },
        Expr::Field {
            name: "point".to_string(),
            field: "y".to_string(),
        }
    );

    assert_eq!(
        Expr::QualifiedVariable {
            module: "M".to_string(),
            name: "name".to_string(),
        },
        Expr::QualifiedVariable {
            module: "M".to_string(),
            name: "name".to_string(),
        }
    );
    assert_ne!(
        Expr::QualifiedVariable {
            module: "M".to_string(),
            name: "name".to_string(),
        },
        Expr::QualifiedVariable {
            module: "N".to_string(),
            name: "name".to_string(),
        }
    );

    let call = Expr::Call {
        module: Some("M".to_string()),
        name: "f".to_string(),
        args: vec![Expr::Integer(1)],
    };
    assert_eq!(
        call,
        Expr::Call {
            module: Some("M".to_string()),
            name: "f".to_string(),
            args: vec![Expr::Integer(1)],
        }
    );
    assert_ne!(
        call,
        Expr::Call {
            module: Some("M".to_string()),
            name: "g".to_string(),
            args: vec![Expr::Integer(1)],
        }
    );

    let unary = Expr::Unary {
        op: UnaryOp::Minus,
        value: Box::new(Expr::Integer(3)),
    };
    assert_eq!(
        unary,
        Expr::Unary {
            op: UnaryOp::Minus,
            value: Box::new(Expr::Integer(3)),
        }
    );
    assert_ne!(
        unary,
        Expr::Unary {
            op: UnaryOp::Plus,
            value: Box::new(Expr::Integer(3)),
        }
    );

    let binary = Expr::Binary {
        op: BinaryOp::Add,
        left: Box::new(Expr::Integer(1)),
        right: Box::new(Expr::Integer(2)),
    };
    assert_eq!(
        binary,
        Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Integer(1)),
            right: Box::new(Expr::Integer(2)),
        }
    );
    assert_ne!(
        binary,
        Expr::Binary {
            op: BinaryOp::Sub,
            left: Box::new(Expr::Integer(1)),
            right: Box::new(Expr::Integer(2)),
        }
    );

    assert_eq!(
        AssignTarget::Name("x".to_string()),
        AssignTarget::Name("x".to_string())
    );
    assert_eq!(
        AssignTarget::Indexed {
            name: "values".to_string(),
            index: Expr::Integer(0),
        },
        AssignTarget::Indexed {
            name: "values".to_string(),
            index: Expr::Integer(0),
        }
    );
    assert_ne!(
        AssignTarget::Name("x".to_string()),
        AssignTarget::Indexed {
            name: "x".to_string(),
            index: Expr::Integer(0),
        }
    );
    assert_eq!(
        AssignTarget::Field {
            name: "point".to_string(),
            field: "x".to_string(),
        },
        AssignTarget::Field {
            name: "point".to_string(),
            field: "x".to_string(),
        }
    );
    assert_ne!(
        AssignTarget::Field {
            name: "point".to_string(),
            field: "x".to_string(),
        },
        AssignTarget::Field {
            name: "point".to_string(),
            field: "y".to_string(),
        }
    );

    assert_ne!(Expr::Integer(1), Expr::Boolean(true));
}
