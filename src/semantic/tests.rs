use std::collections::HashMap;
use std::path::PathBuf;

use rstest::rstest;

use super::{
    ExternalModuleInfo, SemanticError, analyze, analyze_expr, analyze_statement,
    assignment_compatible_extended, format_expr_for_error, infer_expr_type,
    resolve_array_length_expr, resolve_builtin_with_module_validation, substitute_const_expr,
    type_ref_name_for_error, validate_boolean_condition, validate_const_expression_literal,
    validate_declared_type, validate_declared_type_with_imports,
};
use crate::ast::{
    AssignTarget, BinaryOp, Declaration, Expr, ParamDecl, Statement, TypeRef, UnaryOp,
};
use crate::manifest::ExternalManifest;
use crate::parser::parse_module;
use crate::symbols::{SymbolKind, SymbolTable};

#[derive(serde::Deserialize)]
struct SemanticTestCase {
    name: String,
    #[serde(alias = "code")]
    source: String,
    result: Option<SemanticTestCaseResult>,
    #[serde(default)]
    is_success: bool,
    manifest: Option<ExternalManifest>,
}
#[derive(serde::Deserialize)]
struct SemanticTestCaseResult {
    code: String,
    messages: Vec<String>,
}

fn semantic_compile_test(source: &str) -> Result<(), SemanticError> {
    semantic_compile_test_with_manifest(source, None)
}

fn semantic_compile_test_with_manifest(
    source: &str,
    manifest: Option<&ExternalManifest>,
) -> Result<(), SemanticError> {
    let module = parse_module(source).expect("source should parse for semantic test");
    let result = analyze(&module, manifest);
    match result {
        Err(err) => Err(err.downcast::<SemanticError>().unwrap()),
        Ok(_) => Ok(()),
    }
}

#[rstest]
fn semantic_error_cases1(#[files("tests/semantic_cases/general/*.toml")] path: PathBuf) {
    let content = std::fs::read_to_string(&path).expect("error case file should be readable");
    let case: SemanticTestCase =
        toml::from_str(&content).expect("error case file should be valid TOML");
    let should_be_success = case.result.is_none() || case.is_success;
    let response = if let Some(manifest) = &case.manifest {
        semantic_compile_test_with_manifest(case.source.as_str(), Some(manifest))
    } else {
        semantic_compile_test(case.source.as_str())
    };
    if should_be_success {
        if response.is_err() {
            panic!(
                "case '{}': expected success, got error {:?}",
                case.name,
                response.err()
            );
        }
        return;
    }
    if let Err(err) = response {
        let result = case.result.unwrap();
        assert_eq!(
            err.code(),
            result.code,
            "case '{}': error code mismatch",
            case.name
        );
        for fragment in &result.messages {
            assert!(
                err.to_string().contains(fragment),
                "case '{}': display string should contain '{fragment}', got '{}'",
                case.name,
                err
            );
        }
    } else {
        if !should_be_success {
            panic!("case '{}': expected error, got success", case.name);
        }
    }
}

#[test]
fn semantic_error_code_and_display_cover_all_variants() {
    let cases = vec![
        (
            SemanticError::ModuleNameMismatch {
                expected: "Main".to_string(),
                got: "Wrong".to_string(),
            },
            "E001",
            "Module name mismatch at END",
        ),
        (
            SemanticError::DuplicateImportAlias {
                alias: "B".to_string(),
            },
            "E002",
            "Duplicate import alias",
        ),
        (
            SemanticError::UnmappedImport {
                import: "ModuleB".to_string(),
            },
            "E003",
            "not mapped to a crate",
        ),
        (
            SemanticError::DuplicateSymbol {
                name: "Count".to_string(),
            },
            "E004",
            "Duplicate symbol declaration",
        ),
        (
            SemanticError::UndefinedSymbol {
                name: "x".to_string(),
            },
            "E005",
            "Undefined symbol usage",
        ),
        (
            SemanticError::ArityMismatch {
                name: "P".to_string(),
                expected: 2,
                got: 1,
            },
            "E006",
            "called with wrong arity",
        ),
        (
            SemanticError::InvalidBuiltinArgument {
                name: "WriteString".to_string(),
                detail: "expected a string literal".to_string(),
            },
            "E007",
            "received an invalid argument",
        ),
        (
            SemanticError::UnsupportedStringLiteral,
            "E008",
            "String literals are only supported",
        ),
        (
            SemanticError::NotCallable {
                name: "x".to_string(),
            },
            "E009",
            "is not callable",
        ),
        (
            SemanticError::ProcedureNameMismatch {
                expected: "P".to_string(),
                got: "Wrong".to_string(),
            },
            "E010",
            "Procedure END name mismatch",
        ),
        (
            SemanticError::InvalidVarArgument {
                name: "Bump".to_string(),
                position: 1,
                detail: "expected a variable designator".to_string(),
            },
            "E011",
            "invalid VAR argument",
        ),
        (
            SemanticError::TypeMismatch {
                detail: "cannot assign REAL to INTEGER 'x'".to_string(),
            },
            "E012",
            "Type mismatch",
        ),
        (
            SemanticError::UnknownType {
                name: "Missing".to_string(),
            },
            "E013",
            "Unknown type reference",
        ),
        (
            SemanticError::NonExportedMember {
                module: "ModuleB".to_string(),
                name: "HiddenType".to_string(),
            },
            "E014",
            "is not exported from module",
        ),
        (
            SemanticError::UnsupportedQualifiedVariable {
                module: "B".to_string(),
                name: "value".to_string(),
            },
            "E015",
            "Qualified variable reference",
        ),
        (
            SemanticError::InternalError {
                error: "TestError".to_string(),
            },
            "E999",
            "TestError",
        ),
    ];

    for (err, expected_code, expected_fragment) in cases {
        assert_eq!(err.code(), expected_code);
        assert!(
            err.to_string().contains(expected_fragment),
            "display string should contain '{expected_fragment}', got '{}'",
            err
        );
    }
}

#[test]
fn semantic_var_parameter_rejects_non_assignable_binding() {
    let err = semantic_compile_test(
        r#"
MODULE Main;
CONST c = 1;
PROCEDURE Bump(VAR target: INTEGER);
BEGIN
END Bump;
BEGIN
    Bump(c)
END Main.
"#,
    )
    .unwrap_err();

    assert_eq!(err.code(), "E011");
    assert!(
        err.to_string()
            .contains("is not an assignable variable binding"),
        "expected assignable-binding VAR diagnostic, got '{err}'"
    );
}

#[test]
fn semantic_accepts_indexed_assignment_and_expression_usage() {
    let module = parse_module(
        r#"
MODULE Main;
VAR arr: ARRAY 4 OF INTEGER;
    i: INTEGER;
    x: INTEGER;
BEGIN
    i := 1;
    arr[i] := 7;
    x := arr[0]
END Main.
"#,
    )
    .expect("source should parse");

    analyze(&module, None).expect("indexed array access should pass semantic analysis");
}

#[test]
fn semantic_accepts_array_length_constant_expression() {
    let module = parse_module(
        r#"
MODULE Main;
CONST n = 2;
TYPE
    Vec = ARRAY n + 3 OF INTEGER;
VAR x: Vec;
BEGIN
    x[0] := 1
END Main.
"#,
    )
    .expect("source should parse");

    analyze(&module, None).expect("constant-expression array lengths should be accepted");
}

#[test]
fn semantic_rejects_non_constant_array_length_expression() {
    let err = semantic_compile_test(
        r#"
MODULE Main;
VAR n: INTEGER;
    arr: ARRAY n + 1 OF INTEGER;
BEGIN
END Main.
"#,
    )
    .expect_err("non-constant array length should fail semantic analysis");

    assert_eq!(err.code(), "E013");
    assert!(
        err.to_string().contains("Unknown type reference"),
        "expected unknown-type diagnostic for non-constant array length, got '{err}'"
    );
}

#[test]
fn semantic_rejects_non_integer_array_index() {
    let err = semantic_compile_test(
        r#"
MODULE Main;
VAR arr: ARRAY 4 OF INTEGER;
BEGIN
    arr[TRUE] := 1
END Main.
"#,
    )
    .expect_err("non-integer array index should fail semantic analysis");

    assert_eq!(err.code(), "E012");
    assert!(
        err.to_string()
            .contains("array index for 'arr' must be INTEGER")
    );
}

#[test]
fn semantic_rejects_indexing_non_array_variable() {
    let err = semantic_compile_test(
        r#"
MODULE Main;
VAR x: INTEGER;
BEGIN
    x := x[0]
END Main.
"#,
    )
    .expect_err("indexing a scalar variable should fail semantic analysis");

    assert_eq!(err.code(), "E012");
    assert!(err.to_string().contains("is not an array variable"));
}

#[test]
fn semantic_accepts_flt_floor_and_eof_conditions() {
    let module = parse_module(
        r#"
MODULE Main;
IMPORT IO, MATH;
VAR x: REAL;
    y: INTEGER;
BEGIN
    x := MATH.FLT(y);
    y := MATH.FLOOR(x);
    IF IO.EOF() THEN
    END;
    WHILE IO.EOF() DO
    END
END Main.
"#,
    )
    .expect("source should parse");

    analyze(&module, None).expect("FLT/FLOOR and EOF conditions should be accepted");
}

#[test]
fn semantic_accepts_qualified_io_and_math_builtins() {
    let module = parse_module(
        r#"
MODULE Main;
IMPORT IO, MATH;
VAR x: INTEGER;
    r: REAL;
BEGIN
    x := IO.ReadInt();
    IF IO.EOF() THEN
      IO.WriteLn()
    END;
    r := MATH.FLT(x);
    x := MATH.FLOOR(r);
    IO.WriteInt(x)
END Main.
"#,
    )
    .expect("source should parse");

    analyze(&module, None).expect("qualified IO/MATH builtins should be accepted");
}

#[test]
fn semantic_resolve_builtin_with_module_validation_covers_all_branches() {
    let io_readint = resolve_builtin_with_module_validation(Some("IO"), "ReadInt")
        .expect("IO.ReadInt should resolve as internal builtin");
    assert!(io_readint.is_some(), "IO.ReadInt should map to a builtin id");

    let math_floor = resolve_builtin_with_module_validation(Some("MATH"), "FLOOR")
        .expect("MATH.FLOOR should resolve as internal builtin");
    assert!(
        math_floor.is_some(),
        "MATH.FLOOR should map to a builtin id"
    );

    let err = resolve_builtin_with_module_validation(Some("IO"), "NoSuchBuiltin")
        .expect_err("unknown IO builtin member should fail");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E007");
    assert!(
        err.to_string().contains("unknown builtin member"),
        "expected unknown-member diagnostic, got '{err}'"
    );

    let err = resolve_builtin_with_module_validation(Some("MATH"), "ReadInt")
        .expect_err("unknown MATH builtin member should fail");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E007");
    assert!(
        err.to_string().contains("unknown builtin member"),
        "expected unknown-member diagnostic, got '{err}'"
    );

    let external_member =
        resolve_builtin_with_module_validation(Some("ModuleB"), "HELLO")
            .expect("non-internal modules should bypass builtin member validation");
    assert!(
        external_member.is_none(),
        "non-internal modules should not resolve as builtins"
    );

    let unqualified_builtin = resolve_builtin_with_module_validation(None, "ReadInt")
        .expect("unqualified call should bypass internal-module validation");
    assert!(
        unqualified_builtin.is_none(),
        "unqualified builtin names must not resolve in strict qualified mode"
    );
}

#[test]
fn semantic_directly_covers_inference_and_boolean_validation_paths() {
    let mut symbols = SymbolTable::new();
    symbols
        .declare_with_type("x", SymbolKind::Variable, Some(TypeRef::Integer))
        .expect("integer variable should be declared");
    symbols
        .declare_with_type("r", SymbolKind::Variable, Some(TypeRef::Real))
        .expect("real variable should be declared");
    symbols
        .declare_with_type("b", SymbolKind::Variable, Some(TypeRef::Boolean))
        .expect("boolean variable should be declared");
    symbols
        .declare("IO", SymbolKind::Procedure)
        .expect("IO import alias should be declared");
    symbols
        .declare("MATH", SymbolKind::Procedure)
        .expect("MATH import alias should be declared");

    let mut types = HashMap::new();
    types.insert("INTEGER".to_string(), TypeRef::Integer);
    types.insert("BOOLEAN".to_string(), TypeRef::Boolean);
    types.insert("REAL".to_string(), TypeRef::Real);
    types.insert("LONGREAL".to_string(), TypeRef::LongReal);

    assert_eq!(
        infer_expr_type(&Expr::Variable("x".to_string()), &symbols, &types)
            .expect("integer variable should infer")
            .unwrap(),
        TypeRef::Integer
    );

    let err = infer_expr_type(&Expr::Variable("missing".to_string()), &symbols, &types)
        .expect_err("unknown variable should fail inference");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E005");

    let err = infer_expr_type(
        &Expr::QualifiedVariable {
            module: "B".to_string(),
            name: "value".to_string(),
        },
        &symbols,
        &types,
    )
    .expect_err("qualified variable should fail inference");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E015");

    let err = infer_expr_type(
        &Expr::Call {
            module: Some("IO".to_string()),
            name: "ReadInt".to_string(),
            args: vec![Expr::Integer(1)],
        },
        &symbols,
        &types,
    )
    .expect_err("ReadInt arity should fail inference");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E006");

    let err = infer_expr_type(
        &Expr::Call {
            module: Some("MATH".to_string()),
            name: "FLT".to_string(),
            args: vec![Expr::Boolean(true)],
        },
        &symbols,
        &types,
    )
    .expect_err("FLT should reject non-integer types");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E012");

    let err = infer_expr_type(
        &Expr::Call {
            module: Some("MATH".to_string()),
            name: "FLOOR".to_string(),
            args: vec![Expr::Integer(1)],
        },
        &symbols,
        &types,
    )
    .expect_err("FLOOR should reject non-real input");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E012");

    let err = infer_expr_type(
        &Expr::Call {
            module: None,
            name: "Unknown".to_string(),
            args: vec![],
        },
        &symbols,
        &types,
    )
    .expect_err("unknown function should fail inference");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E005");

    let err = validate_boolean_condition(&Expr::Integer(1), &symbols, &types)
        .expect_err("numeric condition should fail validation");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E012");

    assert!(
        validate_boolean_condition(
            &Expr::Call {
                module: Some("IO".to_string()),
                name: "EOF".to_string(),
                args: vec![],
            },
            &symbols,
            &types,
        )
        .is_ok()
    );
}

#[test]
fn semantic_infer_expr_type_covers_remaining_call_and_operator_branches() {
    let mut symbols = SymbolTable::new();
    symbols
        .declare("IO", SymbolKind::Procedure)
        .expect("IO import alias should be declared");
    symbols
        .declare("MATH", SymbolKind::Procedure)
        .expect("MATH import alias should be declared");
    symbols
        .declare("custom", SymbolKind::Procedure)
        .expect("custom procedure should be declared");
    symbols
        .declare_with_type("maybe", SymbolKind::Variable, Some(TypeRef::Named("MissingType".to_string())))
        .expect("variable with unresolved named type should be declared");

    let mut types = HashMap::new();
    types.insert("INTEGER".to_string(), TypeRef::Integer);
    types.insert("BOOLEAN".to_string(), TypeRef::Boolean);
    types.insert("REAL".to_string(), TypeRef::Real);
    types.insert("LONGREAL".to_string(), TypeRef::LongReal);

    let err = infer_expr_type(
        &Expr::Call {
            module: None,
            name: "ReadInt".to_string(),
            args: vec![],
        },
        &symbols,
        &types,
    )
    .expect_err("unqualified internal builtin should be rejected");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E007");
    assert!(
        err.to_string().contains("must be qualified as IO.ReadInt(...)")
    );

    let err = infer_expr_type(
        &Expr::Call {
            module: Some("IO".to_string()),
            name: "WriteLn".to_string(),
            args: vec![],
        },
        &symbols,
        &types,
    )
    .expect_err("statement-only builtin should be rejected in expression context");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E007");
    assert!(err.to_string().contains("must be used as a statement call"));

    let err = infer_expr_type(
        &Expr::Call {
            module: Some("ModuleB".to_string()),
            name: "HELLO".to_string(),
            args: vec![],
        },
        &symbols,
        &types,
    )
    .expect_err("non-internal qualified call should fail with undefined symbol");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E005");
    assert!(err.to_string().contains("Undefined symbol usage: 'ModuleB.HELLO'"));

    let err = infer_expr_type(
        &Expr::Call {
            module: None,
            name: "custom".to_string(),
            args: vec![],
        },
        &symbols,
        &types,
    )
    .expect_err("non-builtin procedures should not infer expression types");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E007");

    let div_err = infer_expr_type(
        &Expr::Binary {
            op: BinaryOp::IntDiv,
            left: Box::new(Expr::Boolean(true)),
            right: Box::new(Expr::Integer(1)),
        },
        &symbols,
        &types,
    )
    .expect_err("DIV with non-integer operands should fail");
    let div_err = div_err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(div_err.code(), "E012");
    assert!(div_err.to_string().contains("operator 'DIV' requires INTEGER operands"));

    let mod_err = infer_expr_type(
        &Expr::Binary {
            op: BinaryOp::Mod,
            left: Box::new(Expr::Boolean(true)),
            right: Box::new(Expr::Integer(1)),
        },
        &symbols,
        &types,
    )
    .expect_err("MOD with non-integer operands should fail");
    let mod_err = mod_err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(mod_err.code(), "E012");
    assert!(mod_err.to_string().contains("operator 'MOD' requires INTEGER operands"));

    let unary_none = infer_expr_type(
        &Expr::Unary {
            op: UnaryOp::Plus,
            value: Box::new(Expr::Variable("maybe".to_string())),
        },
        &symbols,
        &types,
    )
    .expect("unknown named type should propagate as None in unary inference");
    assert_eq!(unary_none, None);

    let binary_none = infer_expr_type(
        &Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Variable("maybe".to_string())),
            right: Box::new(Expr::Integer(1)),
        },
        &symbols,
        &types,
    )
    .expect("unknown named type should propagate as None in binary inference");
    assert_eq!(binary_none, None);
}

#[test]
fn semantic_covers_remaining_branch_paths() {
    let mut symbols = SymbolTable::new();
    symbols
        .declare_with_type("x", SymbolKind::Variable, Some(TypeRef::Integer))
        .expect("integer variable should be declared");
    symbols
        .declare_with_type("r", SymbolKind::Variable, Some(TypeRef::Real))
        .expect("real variable should be declared");
    symbols
        .declare("custom", SymbolKind::Procedure)
        .expect("custom procedure should be declared");
    symbols
        .declare_with_type("flag", SymbolKind::Variable, Some(TypeRef::Boolean))
        .expect("boolean variable should be declared");
    symbols
        .declare("IO", SymbolKind::Procedure)
        .expect("IO import alias should be declared");
    symbols
        .declare("MATH", SymbolKind::Procedure)
        .expect("MATH import alias should be declared");

    let mut types = HashMap::new();
    types.insert("INTEGER".to_string(), TypeRef::Integer);
    types.insert("BOOLEAN".to_string(), TypeRef::Boolean);
    types.insert("REAL".to_string(), TypeRef::Real);
    types.insert("LONGREAL".to_string(), TypeRef::LongReal);

    let non_const_decl = Declaration::Var {
        name: "bad".to_string(),
        declared_type: Some(TypeRef::Integer),
    };
    let err = validate_const_expression_literal(&non_const_decl)
        .expect_err("non-const declaration should fail");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E999");

    for type_ref in [
        TypeRef::Integer,
        TypeRef::Boolean,
        TypeRef::Real,
        TypeRef::LongReal,
    ] {
        assert!(validate_declared_type(&type_ref, &types).is_ok());
    }

    assert!(
        validate_declared_type(
            &TypeRef::Qualified {
                module: "B".to_string(),
                name: "IntType".to_string(),
            },
            &types,
        )
        .is_ok()
    );

    assert_eq!(type_ref_name_for_error(&TypeRef::Integer), "INTEGER");
    assert_eq!(type_ref_name_for_error(&TypeRef::Boolean), "BOOLEAN");
    assert_eq!(type_ref_name_for_error(&TypeRef::Real), "REAL");
    assert_eq!(type_ref_name_for_error(&TypeRef::LongReal), "LONGREAL");
    assert_eq!(
        type_ref_name_for_error(&TypeRef::Named("Alias".to_string())),
        "Alias"
    );
    assert_eq!(
        type_ref_name_for_error(&TypeRef::Qualified {
            module: "B".to_string(),
            name: "IntType".to_string(),
        }),
        "B.IntType"
    );

    let err = validate_declared_type(&TypeRef::Named("Alias".to_string()), &types)
        .expect_err("unknown named type should fail");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E013");

    let import_aliases = HashMap::from([("B".to_string(), "ModuleB".to_string())]);
    let external_modules = ExternalModuleInfo::mock_resolver();
    let const_values = HashMap::new();
    assert!(
        validate_declared_type_with_imports(
            &TypeRef::Qualified {
                module: "B".to_string(),
                name: "IntType".to_string(),
            },
            &types,
            &import_aliases,
            &external_modules,
            &const_values,
        )
        .is_ok()
    );

    let err = validate_declared_type_with_imports(
        &TypeRef::Qualified {
            module: "B".to_string(),
            name: "HiddenType".to_string(),
        },
        &types,
        &import_aliases,
        &external_modules,
        &const_values,
    )
    .expect_err("non-exported qualified type should fail");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E014");

    let err = validate_declared_type_with_imports(
        &TypeRef::Qualified {
            module: "Z".to_string(),
            name: "IntType".to_string(),
        },
        &types,
        &import_aliases,
        &external_modules,
        &const_values,
    )
    .expect_err("unknown module-qualified type should fail");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E013");
    assert!(assignment_compatible_extended(
        &TypeRef::Qualified {
            module: "B".to_string(),
            name: "IntType".to_string(),
        },
        &TypeRef::Integer,
        &import_aliases,
        &external_modules,
    ));
    assert!(!assignment_compatible_extended(
        &TypeRef::Real,
        &TypeRef::Qualified {
            module: "B".to_string(),
            name: "IntType".to_string(),
        },
        &import_aliases,
        &external_modules,
    ));
    assert!(!assignment_compatible_extended(
        &TypeRef::Qualified {
            module: "Z".to_string(),
            name: "IntType".to_string(),
        },
        &TypeRef::Integer,
        &import_aliases,
        &external_modules,
    ));

    assert!(validate_boolean_condition(&Expr::Boolean(true), &symbols, &types).is_ok());

    let err = validate_boolean_condition(&Expr::Real(1.0), &symbols, &types)
        .expect_err("non-boolean condition should fail");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E012");

    let err = infer_expr_type(&Expr::String("x".to_string()), &symbols, &types)
        .expect_err("string literals should fail inference");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E008");

    let err = infer_expr_type(
        &Expr::Call {
            module: Some("MATH".to_string()),
            name: "FLT".to_string(),
            args: vec![],
        },
        &symbols,
        &types,
    )
    .expect_err("FLT arity should fail inference");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E006");

    let err = infer_expr_type(
        &Expr::Call {
            module: Some("MATH".to_string()),
            name: "FLOOR".to_string(),
            args: vec![],
        },
        &symbols,
        &types,
    )
    .expect_err("FLOOR arity should fail inference");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E006");

    let err = infer_expr_type(
        &Expr::Call {
            module: None,
            name: "custom".to_string(),
            args: vec![],
        },
        &symbols,
        &types,
    )
    .expect_err("custom builtin-like calls should fail inference");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E007");

    let err = infer_expr_type(
        &Expr::Unary {
            op: UnaryOp::Minus,
            value: Box::new(Expr::Boolean(true)),
        },
        &symbols,
        &types,
    )
    .expect_err("numeric unary errors should fail inference");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E012");

    let err = infer_expr_type(
        &Expr::Unary {
            op: UnaryOp::Not,
            value: Box::new(Expr::Integer(1)),
        },
        &symbols,
        &types,
    )
    .expect_err("boolean unary errors should fail inference");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E012");

    let err = infer_expr_type(
        &Expr::Binary {
            op: BinaryOp::And,
            left: Box::new(Expr::Integer(1)),
            right: Box::new(Expr::Boolean(true)),
        },
        &symbols,
        &types,
    )
    .expect_err("logical binary errors should fail inference");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E012");

    let err = analyze_expr(&Expr::Variable("missing".to_string()), &symbols)
        .expect_err("undefined expressions should fail analysis");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E005");

    let proc_arity = HashMap::new();
    let mut proc_params = HashMap::new();
    symbols
        .declare("p", SymbolKind::Procedure)
        .expect("procedure should be declared");
    symbols
        .declare("q", SymbolKind::Procedure)
        .expect("procedure should be declared");
    proc_params.insert(
        "p".to_string(),
        vec![ParamDecl {
            name: "target".to_string(),
            declared_type: Some(TypeRef::Integer),
            is_var: true,
        }],
    );
    let err = analyze_statement(
        &Statement::Call {
            module: None,
            name: "p".to_string(),
            args: vec![Expr::Variable("r".to_string())],
        },
        &mut symbols,
        &proc_arity,
        &proc_params,
        &types,
        &import_aliases,
        &external_modules,
    )
    .expect_err("VAR parameter mismatches should fail analysis");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E012");

    proc_params.insert(
        "q".to_string(),
        vec![ParamDecl {
            name: "flag".to_string(),
            declared_type: Some(TypeRef::Boolean),
            is_var: false,
        }],
    );
    let err = analyze_statement(
        &Statement::Call {
            module: None,
            name: "q".to_string(),
            args: vec![Expr::Integer(1)],
        },
        &mut symbols,
        &proc_arity,
        &proc_params,
        &types,
        &import_aliases,
        &external_modules,
    )
    .expect_err("parameter mismatches should fail analysis");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E012");

    let if_stmt = Statement::If {
        condition: Expr::Variable("flag".to_string()),
        then_branch: vec![Statement::Assign {
            target: AssignTarget::Name("flag".to_string()),
            value: Expr::Boolean(true),
        }],
        else_branch: Some(vec![Statement::Assign {
            target: AssignTarget::Name("flag".to_string()),
            value: Expr::Boolean(false),
        }]),
    };
    analyze_statement(
        &if_stmt,
        &mut symbols,
        &proc_arity,
        &proc_params,
        &types,
        &import_aliases,
        &external_modules,
    )
    .expect("if statement analysis should succeed");

    let while_stmt = Statement::While {
        condition: Expr::Variable("flag".to_string()),
        body: vec![Statement::Assign {
            target: AssignTarget::Name("flag".to_string()),
            value: Expr::Boolean(false),
        }],
    };
    analyze_statement(
        &while_stmt,
        &mut symbols,
        &proc_arity,
        &proc_params,
        &types,
        &import_aliases,
        &external_modules,
    )
    .expect("while statement analysis should succeed");
}

#[test]
fn semantic_covers_const_validation_and_error_display_paths() {
    let decl = Declaration::Const {
        name: "bad".to_string(),
        value: Expr::Call {
            module: None,
            name: "ReadInt".to_string(),
            args: vec![],
        },
    };

    let err =
        validate_const_expression_literal(&decl).expect_err("non-literal const should be rejected");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E016");

    let errors = vec![
        SemanticError::InvalidConstDeclaration {
            name: "bad".to_string(),
        },
        SemanticError::InternalError {
            error: "boom".to_string(),
        },
    ];

    for err in errors {
        assert!(
            err.to_string().contains("Constant")
                || err.to_string().contains("Internal compiler error")
        );
    }
}

#[test]
fn semantic_analyze_expr_covers_remaining_branches() {
    let mut symbols = SymbolTable::new();
    symbols
        .declare("IO", SymbolKind::Procedure)
        .expect("IO import alias should be declared");
    symbols
        .declare("MATH", SymbolKind::Procedure)
        .expect("MATH import alias should be declared");
    symbols
        .declare("custom", SymbolKind::Procedure)
        .expect("custom procedure should be declared");
    symbols
        .declare_with_type("arr", SymbolKind::Variable, Some(TypeRef::Integer))
        .expect("array-like symbol should be declared for index path validation");

    assert!(analyze_expr(&Expr::Integer(1), &symbols).is_ok());
    assert!(analyze_expr(&Expr::Real(1.5), &symbols).is_ok());
    assert!(analyze_expr(&Expr::LongReal(2.5), &symbols).is_ok());
    assert!(analyze_expr(&Expr::Boolean(true), &symbols).is_ok());

    let err = analyze_expr(&Expr::String("s".to_string()), &symbols)
        .expect_err("string literal expressions should be rejected");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E008");

    let err = analyze_expr(
        &Expr::Indexed {
            name: "arr".to_string(),
            index: Box::new(Expr::Variable("missing_index".to_string())),
        },
        &symbols,
    )
    .expect_err("indexed expressions should validate index expressions recursively");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E005");

    let err = analyze_expr(
        &Expr::Indexed {
            name: "missing_arr".to_string(),
            index: Box::new(Expr::Integer(0)),
        },
        &symbols,
    )
    .expect_err("indexed expressions should validate the base symbol");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E005");

    let err = analyze_expr(
        &Expr::QualifiedVariable {
            module: "B".to_string(),
            name: "v".to_string(),
        },
        &symbols,
    )
    .expect_err("qualified variable expressions are not supported");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E015");

    let err = analyze_expr(
        &Expr::Call {
            module: Some("UnknownInternal".to_string()),
            name: "ReadInt".to_string(),
            args: vec![],
        },
        &symbols,
    )
    .expect_err("internal builtins should require imported internal modules");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E005");

    let err = analyze_expr(
        &Expr::Call {
            module: None,
            name: "ReadInt".to_string(),
            args: vec![],
        },
        &symbols,
    )
    .expect_err("unqualified internal builtins should be rejected");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E007");

    let err = analyze_expr(
        &Expr::Call {
            module: Some("IO".to_string()),
            name: "WriteLn".to_string(),
            args: vec![],
        },
        &symbols,
    )
    .expect_err("statement-only builtins should be rejected in expression context");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E007");

    let err = analyze_expr(
        &Expr::Call {
            module: Some("IO".to_string()),
            name: "ReadInt".to_string(),
            args: vec![Expr::Integer(1)],
        },
        &symbols,
    )
    .expect_err("builtin arity mismatches should be rejected");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E006");

    let err = analyze_expr(
        &Expr::Call {
            module: Some("MATH".to_string()),
            name: "FLT".to_string(),
            args: vec![Expr::Variable("missing_arg".to_string())],
        },
        &symbols,
    )
    .expect_err("builtin arguments should be validated recursively");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E005");

    let err = analyze_expr(
        &Expr::Call {
            module: Some("ModuleB".to_string()),
            name: "HELLO".to_string(),
            args: vec![],
        },
        &symbols,
    )
    .expect_err("non-internal qualified call expressions should fail");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E005");

    let err = analyze_expr(
        &Expr::Call {
            module: None,
            name: "missing_proc".to_string(),
            args: vec![],
        },
        &symbols,
    )
    .expect_err("unknown unqualified calls should fail");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E005");

    let err = analyze_expr(
        &Expr::Call {
            module: None,
            name: "custom".to_string(),
            args: vec![],
        },
        &symbols,
    )
    .expect_err("non-builtin procedures are not valid expression calls");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E007");

    assert!(
        analyze_expr(
            &Expr::Unary {
                op: UnaryOp::Minus,
                value: Box::new(Expr::Integer(1)),
            },
            &symbols,
        )
        .is_ok()
    );

    assert!(
        analyze_expr(
            &Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::Integer(1)),
                right: Box::new(Expr::Integer(2)),
            },
            &symbols,
        )
        .is_ok()
    );
}

#[test]
fn semantic_validate_boolean_condition_accepts_qualified_eof() {
    let mut symbols = SymbolTable::new();
    symbols
        .declare("IO", SymbolKind::Procedure)
        .expect("IO import alias should be declared");

    let mut types = HashMap::new();
    types.insert("INTEGER".to_string(), TypeRef::Integer);
    types.insert("BOOLEAN".to_string(), TypeRef::Boolean);
    types.insert("REAL".to_string(), TypeRef::Real);
    types.insert("LONGREAL".to_string(), TypeRef::LongReal);

    validate_boolean_condition(
        &Expr::Call {
            module: Some("IO".to_string()),
            name: "EOF".to_string(),
            args: vec![],
        },
        &symbols,
        &types,
    )
    .expect("IO.EOF() should be accepted as a condition");
}

#[test]
fn semantic_analyze_covers_procedure_control_flow_and_param_success_paths() {
    semantic_compile_test(
        r#"
MODULE Main;
IMPORT IO := IO;

PROCEDURE P(flag: BOOLEAN; VAR n: INTEGER);
VAR local: INTEGER;
BEGIN
    IF flag THEN
        n := n + 1
    ELSE
        n := n + 2
    END;

    WHILE IO.EOF() DO
        n := n + local
    END;

    IO.WriteInt(n)
END P;

VAR value: INTEGER;

BEGIN
    value := 0;
    P(FALSE, value)
END Main.
"#,
    )
    .expect("procedure flow with typed params and locals should analyze successfully");
}

#[test]
fn semantic_rejects_non_boolean_if_and_while_conditions() {
    let module = parse_module(
        r#"
MODULE Main;
BEGIN
    IF 1 THEN
    END;
    WHILE 0 DO
    END
END Main.
"#,
    )
    .expect("source should parse");

    let err = analyze(&module, None).expect_err("non-boolean conditions should be rejected");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");

    assert_eq!(err.code(), "E012");
    assert!(
        err.to_string().contains("condition must be BOOLEAN"),
        "expected boolean-condition diagnostic, got '{err}'"
    );
}

#[test]
fn semantic_rejects_invalid_flt_and_floor_argument_types() {
    let module = parse_module(
        r#"
MODULE Main;
IMPORT MATH;
VAR x: INTEGER;
BEGIN
    x := MATH.FLT(TRUE);
    x := MATH.FLOOR(1)
END Main.
"#,
    )
    .expect("source should parse");

    let err =
        analyze(&module, None).expect_err("invalid FLT/FLOOR argument types should be rejected");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");

    assert_eq!(err.code(), "E012");
    assert!(
        err.to_string()
            .contains("FLT() requires an INTEGER argument")
            || err
                .to_string()
                .contains("FLOOR() requires a REAL or LONGREAL argument"),
        "expected FLT/FLOOR type diagnostic, got '{err}'"
    );
}

#[test]
fn issue26_expected_qualified_exported_procedure_call_passes() {
    let module = parse_module(
        r#"
MODULE Main;
IMPORT B := ModuleB;
BEGIN
    B.HELLO
END Main.
"#,
    )
    .expect("source should parse");

    // Expected behavior once issue #26 is implemented:
    // - resolve B.HELLO via imported module alias B
    // - require HELLO to be exported from ModuleB
    analyze(&module, None).expect("qualified exported procedure call should be accepted");
}

#[test]
fn issue26_expected_qualified_non_exported_procedure_call_fails() {
    let module = parse_module(
        r#"
MODULE Main;
IMPORT B := ModuleB;
BEGIN
    B.HiddenProc
END Main.
"#,
    )
    .expect("source should parse");

    let err = analyze(&module, None)
        .expect_err("qualified non-exported procedure call should be rejected");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");

    assert_eq!(err.code(), "E014");
    assert!(
        matches!(
            &err,
            SemanticError::NonExportedMember { module, name }
                if module == "ModuleB" && name == "HiddenProc"
        ),
        "expected NonExportedMember diagnostic, got '{err}'"
    );
}

#[test]
fn issue26_expected_qualified_missing_module_fails() {
    let module = parse_module(
        r#"
MODULE Main;
IMPORT B := MissingModule;
BEGIN
    B.HELLO
END Main.
"#,
    )
    .expect("source should parse");

    let err = analyze(&module, None)
        .expect_err("qualified call to a missing external module should be rejected");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");

    assert_eq!(err.code(), "E005");
    assert!(
        matches!(&err, SemanticError::UndefinedSymbol { name } if name == "B.HELLO"),
        "expected undefined external-module diagnostic, got '{err}'"
    );
}

#[test]
fn issue26_expected_qualified_exported_type_reference_passes() {
    let module = parse_module(
        r#"
MODULE Main;
IMPORT B := ModuleB;
VAR x: B.IntType;
BEGIN
    x := 1
END Main.
"#,
    )
    .expect("source should parse");

    // Expected behavior once issue #26 is implemented:
    // - resolve B.IntType via imported module alias B
    // - require IntType to be exported from ModuleB
    analyze(&module, None).expect("qualified exported type reference should be accepted");
}

#[test]
fn semantic_helpers_cover_expression_format_and_const_substitution_paths() {
    let mut const_values = HashMap::new();
    const_values.insert("N".to_string(), Expr::Integer(4));

    let expr = Expr::Binary {
        op: BinaryOp::Ge,
        left: Box::new(Expr::Indexed {
            name: "arr".to_string(),
            index: Box::new(Expr::Variable("N".to_string())),
        }),
        right: Box::new(Expr::Call {
            module: Some("M".to_string()),
            name: "Proc".to_string(),
            args: vec![
                Expr::Unary {
                    op: UnaryOp::Not,
                    value: Box::new(Expr::Boolean(false)),
                },
                Expr::QualifiedVariable {
                    module: "Q".to_string(),
                    name: "x".to_string(),
                },
                Expr::String("s".to_string()),
                Expr::Real(1.5),
                Expr::LongReal(2.5),
            ],
        }),
    };

    let rendered = format_expr_for_error(&expr);
    assert!(rendered.contains("arr[N]"));
    assert!(rendered.contains("M.Proc(~FALSE, Q.x, \"s\", 1.5, 2.5)"));
    assert!(rendered.contains(">="));

    let substituted = substitute_const_expr(&expr, &const_values);
    let substituted_rendered = format_expr_for_error(&substituted);
    assert!(substituted_rendered.contains("arr[4]"));

    let resolved_len = resolve_array_length_expr(
        &Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Variable("N".to_string())),
            right: Box::new(Expr::Integer(1)),
        },
        &const_values,
    );
    assert_eq!(resolved_len, Some(5));

    let unresolved_len = resolve_array_length_expr(&Expr::Boolean(true), &const_values);
    assert_eq!(unresolved_len, None);
}

#[derive(Clone, Copy)]
enum ScalarType {
    Integer,
    Real,
    LongReal,
    Boolean,
}

impl ScalarType {
    fn name(self) -> &'static str {
        match self {
            ScalarType::Integer => "INTEGER",
            ScalarType::Real => "REAL",
            ScalarType::LongReal => "LONGREAL",
            ScalarType::Boolean => "BOOLEAN",
        }
    }

    fn var_name(self) -> &'static str {
        match self {
            ScalarType::Integer => "i",
            ScalarType::Real => "r",
            ScalarType::LongReal => "lr",
            ScalarType::Boolean => "b",
        }
    }

    fn is_numeric(self) -> bool {
        matches!(
            self,
            ScalarType::Integer | ScalarType::Real | ScalarType::LongReal
        )
    }
}

fn arithmetic_result_type(lhs: ScalarType, rhs: ScalarType) -> ScalarType {
    if matches!(lhs, ScalarType::LongReal) || matches!(rhs, ScalarType::LongReal) {
        ScalarType::LongReal
    } else if matches!(lhs, ScalarType::Real) || matches!(rhs, ScalarType::Real) {
        ScalarType::Real
    } else {
        ScalarType::Integer
    }
}

fn matrix_source(result_type: ScalarType, expr: &str) -> String {
    format!(
        "MODULE Main;\nVAR i: INTEGER;\nVAR r: REAL;\nVAR lr: LONGREAL;\nVAR b: BOOLEAN;\nVAR out: {};\nBEGIN\n  out := {}\nEND Main.\n",
        result_type.name(),
        expr
    )
}

#[test]
fn inferred_operator_types_follow_the_documented_matrix() {
    let mut symbols = SymbolTable::new();
    symbols
        .declare_with_type("i", SymbolKind::Variable, Some(TypeRef::Integer))
        .expect("integer variable should be declared");
    symbols
        .declare_with_type("r", SymbolKind::Variable, Some(TypeRef::Real))
        .expect("real variable should be declared");
    symbols
        .declare_with_type("lr", SymbolKind::Variable, Some(TypeRef::LongReal))
        .expect("longreal variable should be declared");
    symbols
        .declare_with_type("b", SymbolKind::Variable, Some(TypeRef::Boolean))
        .expect("boolean variable should be declared");

    let mut types = HashMap::new();
    types.insert("INTEGER".to_string(), TypeRef::Integer);
    types.insert("BOOLEAN".to_string(), TypeRef::Boolean);
    types.insert("REAL".to_string(), TypeRef::Real);
    types.insert("LONGREAL".to_string(), TypeRef::LongReal);

    let arithmetic = infer_expr_type(
        &Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Variable("i".to_string())),
            right: Box::new(Expr::Variable("r".to_string())),
        },
        &symbols,
        &types,
    )
    .expect("numeric addition should infer")
    .expect("numeric addition should have a result type");
    assert_eq!(arithmetic, TypeRef::Real);

    let relation = infer_expr_type(
        &Expr::Binary {
            op: BinaryOp::Lt,
            left: Box::new(Expr::Variable("i".to_string())),
            right: Box::new(Expr::Variable("lr".to_string())),
        },
        &symbols,
        &types,
    )
    .expect("ordering relation should infer")
    .expect("ordering relation should have a result type");
    assert_eq!(relation, TypeRef::Boolean);

    let boolean = infer_expr_type(
        &Expr::Binary {
            op: BinaryOp::Or,
            left: Box::new(Expr::Variable("b".to_string())),
            right: Box::new(Expr::Variable("b".to_string())),
        },
        &symbols,
        &types,
    )
    .expect("boolean logic should infer")
    .expect("boolean logic should have a result type");
    assert_eq!(boolean, TypeRef::Boolean);
}

#[test]
fn semantic_operator_type_matrix_is_fully_covered() {
    let all_types = [
        ScalarType::Integer,
        ScalarType::Real,
        ScalarType::LongReal,
        ScalarType::Boolean,
    ];

    for operand in all_types {
        let expr = format!("+{}", operand.var_name());
        let source = matrix_source(operand, &expr);
        let result = analyze(
            &parse_module(&source).expect("matrix unary + should parse"),
            None,
        );
        assert_eq!(
            result.is_ok(),
            operand.is_numeric(),
            "unary '+' compatibility mismatch for {}",
            operand.name()
        );
    }

    for operand in all_types {
        let expr = format!("-{}", operand.var_name());
        let source = matrix_source(operand, &expr);
        let result = analyze(
            &parse_module(&source).expect("matrix unary - should parse"),
            None,
        );
        assert_eq!(
            result.is_ok(),
            operand.is_numeric(),
            "unary '-' compatibility mismatch for {}",
            operand.name()
        );
    }

    for operand in all_types {
        let expr = format!("~{}", operand.var_name());
        let source = matrix_source(ScalarType::Boolean, &expr);
        let result = analyze(
            &parse_module(&source).expect("matrix unary ~ should parse"),
            None,
        );
        assert_eq!(
            result.is_ok(),
            matches!(operand, ScalarType::Boolean),
            "unary '~' compatibility mismatch for {}",
            operand.name()
        );
    }

    for lhs in all_types {
        for rhs in all_types {
            let lhs_name = lhs.var_name();
            let rhs_name = rhs.var_name();
            let both_numeric = lhs.is_numeric() && rhs.is_numeric();

            for op in ["+", "-", "*", "/"] {
                let expr = format!("{} {} {}", lhs_name, op, rhs_name);
                let source = matrix_source(arithmetic_result_type(lhs, rhs), &expr);
                let result = analyze(
                    &parse_module(&source).expect("matrix arithmetic should parse"),
                    None,
                );
                assert_eq!(
                    result.is_ok(),
                    both_numeric,
                    "arithmetic compatibility mismatch for {} {} {}",
                    lhs.name(),
                    op,
                    rhs.name()
                );
            }

            for op in ["DIV", "MOD"] {
                let expr = format!("{} {} {}", lhs_name, op, rhs_name);
                let source = matrix_source(ScalarType::Integer, &expr);
                let result = analyze(
                    &parse_module(&source).expect("matrix integer arithmetic should parse"),
                    None,
                );
                assert_eq!(
                    result.is_ok(),
                    matches!(lhs, ScalarType::Integer) && matches!(rhs, ScalarType::Integer),
                    "{} compatibility mismatch for {} and {}",
                    op,
                    lhs.name(),
                    rhs.name()
                );
            }

            for op in ["OR", "&"] {
                let expr = format!("{} {} {}", lhs_name, op, rhs_name);
                let source = matrix_source(ScalarType::Boolean, &expr);
                let result = analyze(
                    &parse_module(&source).expect("matrix boolean op should parse"),
                    None,
                );
                assert_eq!(
                    result.is_ok(),
                    matches!(lhs, ScalarType::Boolean) && matches!(rhs, ScalarType::Boolean),
                    "{} compatibility mismatch for {} and {}",
                    op,
                    lhs.name(),
                    rhs.name()
                );
            }

            for op in ["=", "#"] {
                let expr = format!("{} {} {}", lhs_name, op, rhs_name);
                let source = matrix_source(ScalarType::Boolean, &expr);
                let result = analyze(
                    &parse_module(&source).expect("matrix equality op should parse"),
                    None,
                );
                let expected = (lhs.is_numeric() && rhs.is_numeric())
                    || (matches!(lhs, ScalarType::Boolean) && matches!(rhs, ScalarType::Boolean));
                assert_eq!(
                    result.is_ok(),
                    expected,
                    "{} compatibility mismatch for {} and {}",
                    op,
                    lhs.name(),
                    rhs.name()
                );
            }

            for op in ["<", "<=", ">", ">="] {
                let expr = format!("{} {} {}", lhs_name, op, rhs_name);
                let source = matrix_source(ScalarType::Boolean, &expr);
                let result = analyze(
                    &parse_module(&source).expect("matrix ordering op should parse"),
                    None,
                );
                assert_eq!(
                    result.is_ok(),
                    both_numeric,
                    "{} compatibility mismatch for {} and {}",
                    op,
                    lhs.name(),
                    rhs.name()
                );
            }
        }
    }
}

#[test]
fn semantic_assignment_compatible_extended_covers_qualified_fallback_paths() {
    let import_aliases = HashMap::from([
        ("B".to_string(), "ModuleB".to_string()),
        ("X".to_string(), "MissingModule".to_string()),
    ]);
    let external_modules = ExternalModuleInfo::mock_resolver();

    assert!(!assignment_compatible_extended(
        &TypeRef::Qualified {
            module: "B".to_string(),
            name: "MissingType".to_string(),
        },
        &TypeRef::Integer,
        &import_aliases,
        &external_modules,
    ));

    assert!(!assignment_compatible_extended(
        &TypeRef::Qualified {
            module: "X".to_string(),
            name: "IntType".to_string(),
        },
        &TypeRef::Integer,
        &import_aliases,
        &external_modules,
    ));

    assert!(!assignment_compatible_extended(
        &TypeRef::Integer,
        &TypeRef::Qualified {
            module: "B".to_string(),
            name: "MissingType".to_string(),
        },
        &import_aliases,
        &external_modules,
    ));

    assert!(!assignment_compatible_extended(
        &TypeRef::Integer,
        &TypeRef::Qualified {
            module: "X".to_string(),
            name: "IntType".to_string(),
        },
        &import_aliases,
        &external_modules,
    ));

    assert!(!assignment_compatible_extended(
        &TypeRef::Integer,
        &TypeRef::Qualified {
            module: "Z".to_string(),
            name: "IntType".to_string(),
        },
        &import_aliases,
        &external_modules,
    ));
}

#[test]
fn semantic_infer_expr_type_covers_readreal_and_readlongreal_arity_paths() {
    let mut symbols = SymbolTable::new();
    symbols
        .declare("IO", SymbolKind::Procedure)
        .expect("IO import alias should be declared");
    let types = HashMap::from([
        ("INTEGER".to_string(), TypeRef::Integer),
        ("BOOLEAN".to_string(), TypeRef::Boolean),
        ("REAL".to_string(), TypeRef::Real),
        ("LONGREAL".to_string(), TypeRef::LongReal),
    ]);

    let err = infer_expr_type(
        &Expr::Call {
            module: Some("IO".to_string()),
            name: "ReadReal".to_string(),
            args: vec![Expr::Integer(1)],
        },
        &symbols,
        &types,
    )
    .expect_err("ReadReal with arguments should fail");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E006");

    let err = infer_expr_type(
        &Expr::Call {
            module: Some("IO".to_string()),
            name: "ReadLongReal".to_string(),
            args: vec![Expr::Integer(1)],
        },
        &symbols,
        &types,
    )
    .expect_err("ReadLongReal with arguments should fail");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E006");
}

#[test]
fn semantic_infer_expr_type_covers_unary_none_branch() {
    let mut symbols = SymbolTable::new();
    symbols
        .declare_with_type(
            "unknown_typed".to_string().as_str(),
            SymbolKind::Variable,
            Some(TypeRef::Named("MissingAlias".to_string())),
        )
        .expect("symbol should be declared");

    let types = HashMap::from([
        ("INTEGER".to_string(), TypeRef::Integer),
        ("BOOLEAN".to_string(), TypeRef::Boolean),
        ("REAL".to_string(), TypeRef::Real),
        ("LONGREAL".to_string(), TypeRef::LongReal),
    ]);

    let inferred = infer_expr_type(
        &Expr::Unary {
            op: UnaryOp::Plus,
            value: Box::new(Expr::Variable("unknown_typed".to_string())),
        },
        &symbols,
        &types,
    )
    .expect("unary inference should succeed for unresolved inner type");
    assert_eq!(inferred, None);
}

#[test]
fn semantic_rejects_indexed_assignment_on_non_array_bindings() {
    let err = semantic_compile_test(
        r#"
MODULE Main;
PROCEDURE P;
BEGIN
END P;
BEGIN
    P[0] := 1
END Main.
"#,
    )
    .expect_err("indexing a procedure binding should fail semantic analysis");
    assert_eq!(err.code(), "E012");
    assert!(err.to_string().contains("is not an array variable"));

    let err = semantic_compile_test(
        r#"
MODULE Main;
VAR x: INTEGER;
BEGIN
    x[0] := 1
END Main.
"#,
    )
    .expect_err("indexed assignment to scalar should fail semantic analysis");
    assert_eq!(err.code(), "E012");
    assert!(err.to_string().contains("is not an array variable"));
}

#[test]
fn semantic_rejects_indexed_assignment_value_type_mismatch() {
    let err = semantic_compile_test(
        r#"
MODULE Main;
VAR flags: ARRAY 2 OF BOOLEAN;
BEGIN
    flags[0] := 1
END Main.
"#,
    )
    .expect_err("indexed assignment with wrong element type should fail");

    assert_eq!(err.code(), "E012");
    assert!(
        err.to_string()
            .contains("cannot assign INTEGER to array element 'flags' of type BOOLEAN")
    );
}

#[test]
fn semantic_qualified_call_validates_argument_expressions() {
    let err = semantic_compile_test(
        r#"
MODULE Main;
IMPORT B := ModuleB;
BEGIN
    B.HELLO(missing)
END Main.
"#,
    )
    .expect_err("qualified call should still validate each argument expression");

    assert_eq!(err.code(), "E005");
    assert!(
        err.to_string()
            .contains("Undefined symbol usage: 'missing'")
    );
}

#[test]
fn semantic_infer_indexed_expr_type_covers_remaining_error_paths() {
    let mut symbols = SymbolTable::new();
    symbols
        .declare("proc_like", SymbolKind::Procedure)
        .expect("procedure should be declared");
    symbols
        .declare_with_type(
            "unknown_alias_var",
            SymbolKind::Variable,
            Some(TypeRef::Named("MissingAlias".to_string())),
        )
        .expect("variable should be declared");

    let types = HashMap::from([
        ("INTEGER".to_string(), TypeRef::Integer),
        ("BOOLEAN".to_string(), TypeRef::Boolean),
        ("REAL".to_string(), TypeRef::Real),
        ("LONGREAL".to_string(), TypeRef::LongReal),
    ]);

    let err = infer_expr_type(
        &Expr::Indexed {
            name: "arr".to_string(),
            index: Box::new(Expr::Boolean(true)),
        },
        &symbols,
        &types,
    )
    .expect_err("non-integer index should fail in indexed expression inference");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E012");

    let err = infer_expr_type(
        &Expr::Indexed {
            name: "missing".to_string(),
            index: Box::new(Expr::Integer(0)),
        },
        &symbols,
        &types,
    )
    .expect_err("unknown indexed symbol should fail");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E005");

    let err = infer_expr_type(
        &Expr::Indexed {
            name: "proc_like".to_string(),
            index: Box::new(Expr::Integer(0)),
        },
        &symbols,
        &types,
    )
    .expect_err("procedure binding cannot be indexed as array");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E012");

    let err = infer_expr_type(
        &Expr::Indexed {
            name: "unknown_alias_var".to_string(),
            index: Box::new(Expr::Integer(0)),
        },
        &symbols,
        &types,
    )
    .expect_err("unresolvable named type cannot be indexed as array");
    let err = err
        .downcast::<SemanticError>()
        .expect("semantic error should be returned");
    assert_eq!(err.code(), "E012");
}
