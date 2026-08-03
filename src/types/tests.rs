use super::{ScalarType, Type};
use crate::ast::{Expr, TypeRef};
use std::collections::HashMap;

#[test]
fn scalar_types_use_one_shared_compatibility_path() {
    let expected = Type::from_ast_type_ref(&TypeRef::Real);
    let actual = Type::from_ast_type_ref(&TypeRef::Real);
    assert!(expected.is_compatible_with(&actual, &HashMap::new()));
}

#[test]
fn numeric_scalar_types_do_not_implicitly_widen_across_the_documented_matrix() {
    let real_expected = Type::from_ast_type_ref(&TypeRef::Real);
    let integer_actual = Type::from_ast_type_ref(&TypeRef::Integer);
    assert!(!real_expected.is_compatible_with(&integer_actual, &HashMap::new()));

    let long_real_expected = Type::from_ast_type_ref(&TypeRef::LongReal);
    assert!(!long_real_expected.is_compatible_with(&integer_actual, &HashMap::new()));
}

#[test]
fn aliases_resolve_before_compatibility_checks() {
    let mut aliases = HashMap::new();
    aliases.insert("Alias".to_string(), Type::Scalar(ScalarType::Integer));

    let expected = Type::Alias("Alias".to_string());
    let actual = Type::Scalar(ScalarType::Integer);
    assert!(expected.is_compatible_with(&actual, &aliases));
}

#[test]
fn array_membership_is_checked_through_the_shared_model() {
    let expected = Type::Array {
        element_type: Box::new(Type::Scalar(ScalarType::Integer)),
        length: Some(10),
    };
    let actual = Type::Array {
        element_type: Box::new(Type::Scalar(ScalarType::Integer)),
        length: Some(10),
    };
    assert!(expected.is_compatible_with(&actual, &HashMap::new()));
}

#[test]
fn nested_aliases_resolve_before_compatibility_checks() {
    let mut aliases = HashMap::new();
    aliases.insert("Alias1".to_string(), Type::Alias("Alias2".to_string()));
    aliases.insert("Alias2".to_string(), Type::Scalar(ScalarType::Real));

    let expected = Type::Alias("Alias1".to_string());
    let actual = Type::Scalar(ScalarType::Real);
    assert!(expected.is_compatible_with(&actual, &aliases));
}

#[test]
fn records_and_procedures_are_compared_by_structure() {
    let expected = Type::Record {
        fields: vec![
            ("x".to_string(), Type::Scalar(ScalarType::Integer)),
            ("y".to_string(), Type::Scalar(ScalarType::Real)),
        ],
    };
    let actual = Type::Record {
        fields: vec![
            ("x".to_string(), Type::Scalar(ScalarType::Integer)),
            ("y".to_string(), Type::Scalar(ScalarType::Real)),
        ],
    };
    assert!(expected.is_compatible_with(&actual, &HashMap::new()));

    let procedure = Type::Procedure {
        parameters: vec![super::ProcedureParameter {
            name: Some("value".to_string()),
            ty: Type::Scalar(ScalarType::Integer),
            is_var: true,
        }],
        result: Some(Box::new(Type::Scalar(ScalarType::Boolean))),
    };
    let matching_procedure = Type::Procedure {
        parameters: vec![super::ProcedureParameter {
            name: Some("other".to_string()),
            ty: Type::Scalar(ScalarType::Integer),
            is_var: true,
        }],
        result: Some(Box::new(Type::Scalar(ScalarType::Boolean))),
    };
    assert!(procedure.is_compatible_with(&matching_procedure, &HashMap::new()));
}

#[test]
fn from_ast_type_ref_handles_named_qualified_and_invalid_array_lengths() {
    let named = Type::from_ast_type_ref(&TypeRef::Named("Meters".to_string()));
    assert_eq!(named, Type::Alias("Meters".to_string()));

    let qualified = Type::from_ast_type_ref(&TypeRef::Qualified {
        module: "Math".to_string(),
        name: "Real".to_string(),
    });
    assert_eq!(qualified, Type::Alias("<qualified>".to_string()));

    let array_with_negative_len = Type::from_ast_type_ref(&TypeRef::Array {
        element_type: Box::new(TypeRef::Integer),
        length: Expr::Integer(-1),
    });
    assert_eq!(
        array_with_negative_len,
        Type::Array {
            element_type: Box::new(Type::Scalar(ScalarType::Integer)),
            length: None,
        }
    );

    let array_with_non_integer_len = Type::from_ast_type_ref(&TypeRef::Array {
        element_type: Box::new(TypeRef::Boolean),
        length: Expr::Boolean(true),
    });
    assert_eq!(
        array_with_non_integer_len,
        Type::Array {
            element_type: Box::new(Type::Scalar(ScalarType::Boolean)),
            length: None,
        }
    );

    let array_with_valid_len = Type::from_ast_type_ref(&TypeRef::Array {
        element_type: Box::new(TypeRef::LongReal),
        length: Expr::Integer(3),
    });
    assert_eq!(
        array_with_valid_len,
        Type::Array {
            element_type: Box::new(Type::Scalar(ScalarType::LongReal)),
            length: Some(3),
        }
    );
}

#[test]
fn is_numeric_matches_documented_scalar_kinds() {
    assert!(Type::Scalar(ScalarType::Integer).is_numeric());
    assert!(Type::Scalar(ScalarType::Real).is_numeric());
    assert!(Type::Scalar(ScalarType::LongReal).is_numeric());
    assert!(!Type::Scalar(ScalarType::Boolean).is_numeric());
    assert!(!Type::Alias("NumberLike".to_string()).is_numeric());
}

#[test]
fn resolve_aliases_recurses_through_composite_types() {
    let mut aliases = HashMap::new();
    aliases.insert("Elem".to_string(), Type::Scalar(ScalarType::LongReal));
    aliases.insert("Ret".to_string(), Type::Scalar(ScalarType::Boolean));

    let source = Type::Procedure {
        parameters: vec![super::ProcedureParameter {
            name: Some("p".to_string()),
            ty: Type::Array {
                element_type: Box::new(Type::Alias("Elem".to_string())),
                length: Some(2),
            },
            is_var: false,
        }],
        result: Some(Box::new(Type::Alias("Ret".to_string()))),
    };

    let resolved = source.resolve_aliases(&aliases);
    let expected = Type::Procedure {
        parameters: vec![super::ProcedureParameter {
            name: Some("p".to_string()),
            ty: Type::Array {
                element_type: Box::new(Type::Scalar(ScalarType::LongReal)),
                length: Some(2),
            },
            is_var: false,
        }],
        result: Some(Box::new(Type::Scalar(ScalarType::Boolean))),
    };
    assert_eq!(resolved, expected);
}

#[test]
fn unresolved_aliases_fall_back_to_the_original_type() {
    let alias = Type::Alias("UnknownAlias".to_string());
    assert_eq!(alias.resolve_aliases(&HashMap::new()), alias);
}

#[test]
fn array_and_record_compatibility_reject_structural_mismatches() {
    let array_len_expected = Type::Array {
        element_type: Box::new(Type::Scalar(ScalarType::Integer)),
        length: Some(4),
    };
    let array_len_actual = Type::Array {
        element_type: Box::new(Type::Scalar(ScalarType::Integer)),
        length: Some(5),
    };
    assert!(!array_len_expected.is_compatible_with(&array_len_actual, &HashMap::new()));

    let array_element_expected = Type::Array {
        element_type: Box::new(Type::Scalar(ScalarType::Real)),
        length: Some(4),
    };
    let array_element_actual = Type::Array {
        element_type: Box::new(Type::Scalar(ScalarType::Integer)),
        length: Some(4),
    };
    assert!(!array_element_expected.is_compatible_with(&array_element_actual, &HashMap::new()));

    let record_name_expected = Type::Record {
        fields: vec![("x".to_string(), Type::Scalar(ScalarType::Integer))],
    };
    let record_name_actual = Type::Record {
        fields: vec![("y".to_string(), Type::Scalar(ScalarType::Integer))],
    };
    assert!(!record_name_expected.is_compatible_with(&record_name_actual, &HashMap::new()));

    let record_len_expected = Type::Record {
        fields: vec![("x".to_string(), Type::Scalar(ScalarType::Integer))],
    };
    let record_len_actual = Type::Record {
        fields: vec![
            ("x".to_string(), Type::Scalar(ScalarType::Integer)),
            ("y".to_string(), Type::Scalar(ScalarType::Integer)),
        ],
    };
    assert!(!record_len_expected.is_compatible_with(&record_len_actual, &HashMap::new()));
}

#[test]
fn procedure_compatibility_rejects_parameter_and_result_mismatches() {
    let base = Type::Procedure {
        parameters: vec![super::ProcedureParameter {
            name: Some("x".to_string()),
            ty: Type::Scalar(ScalarType::Integer),
            is_var: true,
        }],
        result: Some(Box::new(Type::Scalar(ScalarType::Real))),
    };

    let diff_var_mode = Type::Procedure {
        parameters: vec![super::ProcedureParameter {
            name: Some("x".to_string()),
            ty: Type::Scalar(ScalarType::Integer),
            is_var: false,
        }],
        result: Some(Box::new(Type::Scalar(ScalarType::Real))),
    };
    assert!(!base.is_compatible_with(&diff_var_mode, &HashMap::new()));

    let diff_param_type = Type::Procedure {
        parameters: vec![super::ProcedureParameter {
            name: Some("x".to_string()),
            ty: Type::Scalar(ScalarType::Boolean),
            is_var: true,
        }],
        result: Some(Box::new(Type::Scalar(ScalarType::Real))),
    };
    assert!(!base.is_compatible_with(&diff_param_type, &HashMap::new()));

    let diff_param_len = Type::Procedure {
        parameters: vec![],
        result: Some(Box::new(Type::Scalar(ScalarType::Real))),
    };
    assert!(!base.is_compatible_with(&diff_param_len, &HashMap::new()));

    let diff_result_presence = Type::Procedure {
        parameters: vec![super::ProcedureParameter {
            name: Some("x".to_string()),
            ty: Type::Scalar(ScalarType::Integer),
            is_var: true,
        }],
        result: None,
    };
    assert!(!base.is_compatible_with(&diff_result_presence, &HashMap::new()));

    let no_result_left = Type::Procedure {
        parameters: vec![super::ProcedureParameter {
            name: Some("x".to_string()),
            ty: Type::Scalar(ScalarType::Integer),
            is_var: true,
        }],
        result: None,
    };
    let no_result_right = Type::Procedure {
        parameters: vec![super::ProcedureParameter {
            name: Some("y".to_string()),
            ty: Type::Scalar(ScalarType::Integer),
            is_var: true,
        }],
        result: None,
    };
    assert!(no_result_left.is_compatible_with(&no_result_right, &HashMap::new()));
}

#[test]
fn unresolved_aliases_compare_by_fallback_equality() {
    let same_alias_left = Type::Alias("Unknown".to_string());
    let same_alias_right = Type::Alias("Unknown".to_string());
    assert!(same_alias_left.is_compatible_with(&same_alias_right, &HashMap::new()));

    let different_alias = Type::Alias("Other".to_string());
    assert!(!same_alias_left.is_compatible_with(&different_alias, &HashMap::new()));
}
