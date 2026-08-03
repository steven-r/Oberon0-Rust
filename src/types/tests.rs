use super::{ScalarType, Type};
use crate::ast::TypeRef;
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
