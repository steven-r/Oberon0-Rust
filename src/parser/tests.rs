
use std::fs;
use std::path::Path;

use super::parse_module;
use crate::ast::{AssignTarget, BinaryOp, Expr, Statement, UnaryOp};
use crate::manifest::ExternalManifest;
use crate::scanner::scan;
use crate::semantic::analyze;

fn read_dir_sources(dir: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);

    for entry in fs::read_dir(&base).expect("failed to read parser corpus directory") {
        let entry = entry.expect("failed to read parser corpus entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ob0") {
            continue;
        }

        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .expect("invalid filename")
            .to_string();
        let source = fs::read_to_string(&path).expect("failed to read parser corpus file");
        out.push((name, source));
    }

    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn replace_required(source: &str, from: &str, to: &str) -> String {
    assert!(
        source.contains(from),
        "expected source to contain '{from}' before replacement"
    );
    source.replacen(from, to, 1)
}

fn strip_invalid_header_comment(source: &str) -> String {
    let mut lines = source.lines();
    if let Some(first) = lines.next()
        && first.trim_start().starts_with("(* INVALID:")
    {
        return lines.collect::<Vec<_>>().join("\n");
    }

    source.to_string()
}

fn repair_parser_invalid_case(name: &str, source: &str) -> String {
    match name {
        "bad_assign.ob0" => replace_required(source, "x = 1;", "x := 1;"),
        "bad_const_decl.ob0" => replace_required(source, "CONST answer 42;", "CONST answer = 42;"),
        "bad_string_literal.ob0" => {
            replace_required(source, "WriteString(\"Hello)", "WriteString(\"Hello\")")
        }
        "bad_string_literal_embedded_quote.ob0" => replace_required(
            source,
            "WriteString(\"Hello \"Oberon\"\")",
            "WriteString(\"Hello \"\"Oberon\"\"\")",
        ),
        "bad_string_literal_multiline.ob0" => replace_required(
            source,
            "WriteString(\"Hello,\nOberon\")",
            "WriteString(\"Hello, Oberon\")",
        ),
        "import_leading_dot_module.ob0" => {
            replace_required(source, "IMPORT .Module;", "IMPORT ModuleB;")
        }
        "if_call_condition.ob0" => replace_required(source, "IF WriteInt(1 THEN", "IF 1 THEN"),
        "if_missing_end.ob0" => replace_required(
            source,
            "WriteInt(1)\nEND Main.",
            "WriteInt(1)\n  END\nEND Main.",
        ),
        "missing_module_dot.ob0" => format!("{}.", source.trim_end()),
        "operator_div_missing_rhs.ob0" => replace_required(source, "x := 7 DIV", "x := 7 DIV 2"),
        "operator_not_missing_operand.ob0" => {
            replace_required(source, "flag := ~", "flag := ~flag")
        }
        "qualified_member_missing_name.ob0" => replace_required(source, "B.", "B"),
        "relational_missing_rhs.ob0" => replace_required(source, "b := 1 =", "b := 1 = 1"),
        "procedure_missing_semicolon.ob0" => replace_required(
            source,
            "PROCEDURE P(x: INTEGER)\nBEGIN",
            "PROCEDURE P(x: INTEGER);\nBEGIN",
        ),
        other => panic!("missing parser invalid repair mapping for {other}"),
    }
}

fn repair_semantic_invalid_case(name: &str, source: &str) -> String {
    match name {
        "assignment_string_literal.ob0" => replace_required(source, "\"Hello\"", "42"),
        "duplicate_import_alias.ob0" => replace_required(source, "IMPORT IO, IO;", "IMPORT IO;"),
        "duplicate_type_decl.ob0" => replace_required(
            source,
            "TYPE Count = INTEGER;\nTYPE Count = INTEGER;",
            "TYPE Count = INTEGER;\nTYPE CountAlias = INTEGER;",
        ),
        "duplicate_var_decl.ob0" => {
            replace_required(source, "VAR x, x: INTEGER;", "VAR x, y: INTEGER;")
        }
        "end_name_mismatch.ob0" => replace_required(source, "END NotMain.", "END Main."),
        "eof_with_arg.ob0" => replace_required(source, "EOF(1)", "EOF()"),
        "if_undefined_condition.ob0" => replace_required(source, "IF unknown THEN", "IF TRUE THEN"),
        "boolean_assignment_type_mismatch.ob0" => replace_required(source, "x := flag", "x := 1"),
        "boolean_parameter_type_mismatch.ob0" => {
            replace_required(source, "ExpectFlag(TRUE)", "ExpectFlag(FALSE)")
        }
        "boolean_repeat_condition_mismatch.ob0" => {
            replace_required(source, "WHILE 1 DO", "WHILE TRUE DO")
        }
        "condition_requires_boolean.ob0" => replace_required(source, "IF x THEN", "IF TRUE THEN"),
        "operator_div_requires_integer.ob0" => {
            replace_required(source, "x := src DIV 2", "x := 7 DIV 2")
        }
        "operator_not_requires_boolean.ob0" => replace_required(source, "b := ~1", "b := ~b"),
        "operator_or_requires_boolean.ob0" => {
            replace_required(source, "b := 1 OR 0", "b := b OR b")
        }
        "operator_unary_sign_requires_numeric.ob0" => {
            replace_required(source, "x := +flag", "x := +1")
        }
        "relational_requires_numeric.ob0" => {
            replace_required(source, "b1 := b1 < b2", "b1 := 1 < 1")
        }
        "procedure_call_arity_mismatch.ob0" => {
            replace_required(source, "AddAndPrint(2)", "AddAndPrint(2, 3)")
        }
        "procedure_end_name_mismatch.ob0" => {
            replace_required(source, "END WrongName;", "END Echo;")
        }
        "procedure_local_var_self_shadows_type_alias.ob0" => {
            replace_required(source, "VAR Count: Count;", "VAR value: Count;")
        }
        "procedure_local_var_shadows_builtin_type.ob0" => {
            replace_required(source, "VAR INTEGER: INTEGER;", "VAR value: INTEGER;")
        }
        "qualified_call_member_unresolved.ob0" => replace_required(source, "B.HELLO", "WriteLn()"),
        "qualified_call_non_exported.ob0" => {
            replace_required(source, "B.NonExportedProcedure()", "WriteLn()")
        }
        "qualified_type_reference_unsupported.ob0" => {
            replace_required(source, "VAR x: B.IntType;", "VAR x: INTEGER;")
        }
        "readint_statement_call.ob0" | "ReadInt_statement_call.ob0" => r#"
MODULE Main;
VAR x: INTEGER;
BEGIN
  x := ReadInt()
END Main.
"#
        .to_string(),
        "typed_assignment_bool_to_integer.ob0" => replace_required(source, "x := flag", "x := 1"),
        "typed_assignment_real_to_integer.ob0" => replace_required(source, "x := src", "x := 1"),
        "typed_boolean_arithmetic.ob0" => replace_required(source, "x := flag + 1", "x := 1 + 1"),
        "typed_param_self_shadows_type_alias.ob0" => {
            let repaired = replace_required(
                source,
                "PROCEDURE P(Count: Count);",
                "PROCEDURE P(value: Count);",
            );
            replace_required(&repaired, "WriteInt(Count)", "WriteInt(value)")
        }
        "typed_param_shadows_builtin_type.ob0" => {
            let repaired = replace_required(
                source,
                "PROCEDURE P(INTEGER: INTEGER);",
                "PROCEDURE P(value: INTEGER);",
            );
            replace_required(&repaired, "WriteInt(INTEGER)", "WriteInt(value)")
        }
        "typed_param_type_mismatch.ob0" => replace_required(source, "UseInt(x)", "UseInt(1)"),
        "typed_var_unknown_type.ob0" => {
            replace_required(source, "VAR x: Missing;", "VAR x: INTEGER;")
        }
        "undeclared_assignment_target.ob0" => r#"
MODULE Main;
VAR y: INTEGER;
BEGIN
  y := 1;
END Main.
"#
        .to_string(),
        "var_param_requires_variable.ob0" => r#"
MODULE Main;
VAR x: INTEGER;
PROCEDURE Bump(VAR target: INTEGER; amount: INTEGER);
BEGIN
END Bump;
BEGIN
  Bump(x, 2)
END Main.
"#
        .to_string(),
        "while_undefined_in_body.ob0" => {
            let repaired = replace_required(source, "WHILE x DO", "WHILE x > 0 DO");
            replace_required(&repaired, "x := y - 1", "x := x - 1")
        }
        "writeint_string_literal.ob0" | "WriteInt_string_literal.ob0" => {
            replace_required(source, "WriteInt(\"Hello\")", "WriteInt(1)")
        }
        "writeln_with_arg.ob0" => replace_required(source, "WriteLn(1)", "WriteLn()"),
        "writestring_missing_arg.ob0" | "WriteString_missing_arg.ob0" => {
            replace_required(source, "WriteString", "WriteString(\"Hello\")")
        }
        "writestring_non_string_arg.ob0" | "WriteString_non_string_arg.ob0" => {
            replace_required(source, "WriteString(1)", "WriteString(\"1\")")
        }
        "writestring_too_many_args.ob0" | "WriteString_too_many_args.ob0" => replace_required(
            source,
            "WriteString(\"Hello\", \"World\")",
            "WriteString(\"Hello\")",
        ),
        "qualified_call_unknown_alias.ob0" => replace_required(source, "C.HELLO()", "B.HELLO()"),
        "qualified_type_reference_non_exported.ob0" => {
            replace_required(source, "B.HiddenType", "B.IntType")
        }
        "qualified_variable_reference_unsupported.ob0" => {
            replace_required(source, "x := B.value", "x := 1")
        }
        "array_index_non_integer.ob0" => replace_required(source, "i: REAL;", "i: INTEGER;"),
        "array_index_non_array.ob0" => replace_required(source, "x[0] := 1", "x := 1"),
        "array_type_non_constant_expr.ob0" => {
            let repaired = replace_required(source, "VAR n: INTEGER;", "CONST n = 4;");
            replace_required(&repaired, "ARRAY n + 1 OF", "ARRAY n OF")
        }
        "readreal_with_arg.ob0" => replace_required(source, "ReadReal(1)", "ReadReal()"),
        "readlongreal_with_arg.ob0" => {
            replace_required(source, "ReadLongReal(1)", "ReadLongReal()")
        }
        "floor_integer_arg.ob0" => replace_required(source, "FLOOR(1)", "FLOOR(1.0)"),
        "flt_real_arg.ob0" => replace_required(source, "FLT(1.0)", "FLT(1)"),
        other => panic!("missing semantic invalid repair mapping for {other}"),
    }
}

#[test]
fn parses_real_literals_with_scale_factors() {
    let module = parse_module(
        r#"
MODULE Main;
BEGIN
    x := 12.3;
    y := 4.567E8;
    z := 0.57712566D-6
END Main.
"#,
    )
    .expect("real literals should parse");

    let statements = module.statements;
    assert_eq!(statements.len(), 3);

    match &statements[0] {
        Statement::Assign { value, .. } => match value {
            Expr::Real(value) => assert!((value - 12.3).abs() < 0.0001),
            other => panic!("expected first assignment to parse as REAL, got {other:?}"),
        },
        other => panic!("expected assignment statement, got {other:?}"),
    }

    match &statements[1] {
        Statement::Assign { value, .. } => match value {
            Expr::Real(value) => assert!((value - 456700000.0).abs() < 1.0),
            other => panic!("expected scientific REAL literal to parse, got {other:?}"),
        },
        other => panic!("expected assignment statement, got {other:?}"),
    }

    match &statements[2] {
        Statement::Assign { value, .. } => match value {
            Expr::LongReal(value) => assert!((value - 0.00000057712566).abs() < 1e-16),
            other => panic!("expected D-scale literal to parse as LONGREAL, got {other:?}"),
        },
        other => panic!("expected assignment statement, got {other:?}"),
    }
}

#[test]
fn valid_corpus_parses() {
    for (name, source) in read_dir_sources("tests/parser_cases/valid") {
        scan(&source)
            .unwrap_or_else(|err| panic!("expected valid scan for {name}, got error: {err}"));
        parse_module(&source)
            .unwrap_or_else(|err| panic!("expected valid parse for {name}, got error: {err}"));
    }
}

#[test]
fn invalid_corpus_fails() {
    for (name, source) in read_dir_sources("tests/parser_cases/invalid") {
        let result = parse_module(&source);
        assert!(
            result.is_err(),
            "expected invalid parse for {name}, but parsing succeeded"
        );
    }
}

#[test]
fn parser_invalid_corpus_has_single_fault_repairs() {
    for (name, source) in read_dir_sources("tests/parser_cases/invalid") {
        let base = strip_invalid_header_comment(&source);
        let repaired = repair_parser_invalid_case(&name, &base);
        scan(&repaired).unwrap_or_else(|err| {
            panic!("expected repaired parser case {name} to scan successfully, got: {err}")
        });
        parse_module(&repaired).unwrap_or_else(|err| {
            panic!("expected repaired parser case {name} to parse successfully, got: {err}")
        });
    }
}

#[test]
fn semantic_valid_corpus_passes() {
    for (name, source) in read_dir_sources("tests/semantic_cases/valid") {
        scan(&source).unwrap_or_else(|err| {
            panic!("expected valid scan for semantic case {name}, got: {err}")
        });
        let module = parse_module(&source)
            .unwrap_or_else(|err| panic!("expected parse for semantic case {name}, got: {err}"));
        analyze(&module, None)
            .unwrap_or_else(|err| panic!("expected semantic success for {name}, got error: {err}"));
    }
}

#[test]
fn semantic_invalid_corpus_fails() {
    for (name, source) in read_dir_sources("tests/semantic_cases/invalid") {
        let module = parse_module(&source)
            .unwrap_or_else(|err| panic!("expected parse for semantic case {name}, got: {err}"));
        let result = analyze(&module, None);
        assert!(
            result.is_err(),
            "expected semantic failure for {name}, but analysis succeeded"
        );
    }
}

#[test]
fn semantic_invalid_corpus_has_single_fault_repairs() {
    for (name, source) in read_dir_sources("tests/semantic_cases/invalid") {
        let base = strip_invalid_header_comment(&source);
        let repaired = repair_semantic_invalid_case(&name, &base);
        scan(&repaired).unwrap_or_else(|err| {
            panic!("expected repaired semantic case {name} to scan successfully, got: {err}")
        });
        let module = parse_module(&repaired).unwrap_or_else(|err| {
            panic!("expected repaired semantic case {name} to parse successfully, got: {err}")
        });
        analyze(&module, None).unwrap_or_else(|err| {
            panic!("expected repaired semantic case {name} to pass semantic analysis, got: {err}")
        });
    }
}

#[test]
#[cfg_attr(coverage_nightly, coverage(off))]
fn all_examples_parse_and_analyze() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");

    for entry in fs::read_dir(&base).expect("failed to read examples directory") {
        let entry = entry.expect("failed to read examples directory entry");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .expect("example directory name should be valid utf-8")
            .to_string();
        let source_path = path.join("src").join("Main.ob0");
        if !source_path.is_file() {
            continue;
        }

        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|err| panic!("failed to read example source for {name}: {err}"));
        scan(&source)
            .unwrap_or_else(|err| panic!("example {name} should scan successfully: {err}"));
        let module = parse_module(&source)
            .unwrap_or_else(|err| panic!("example {name} should parse successfully: {err}"));

        let manifest_path = path.join("oberon.toml");
        let manifest = if manifest_path.is_file() {
            Some(
                ExternalManifest::from_file(&manifest_path)
                    .unwrap_or_else(|err| panic!("example {name} manifest should load: {err:#}")),
            )
        } else {
            None
        };

        analyze(&module, manifest.as_ref())
            .unwrap_or_else(|err| panic!("example {name} should pass semantic analysis: {err}"));
    }
}

#[test]
fn parses_pascal_style_string_literal_argument() {
    let module = parse_module(
        r#"
MODULE Main;
BEGIN
  WriteString("Hello, ""Oberon""")
END Main.
"#,
    )
    .expect("string literal program should parse");

    let Statement::Call { args, .. } = &module.statements[0] else {
        panic!("expected top-level call statement");
    };

    assert!(matches!(args.first(), Some(Expr::String(value)) if value == "Hello, \"Oberon\""));
}

#[test]
fn parses_export_markers_and_qualified_type_refs() {
    let module = parse_module(
        r#"
MODULE Main;
IMPORT B := ModuleB;
TYPE
  LocalType* = B.IntType;
PROCEDURE Hello*;
BEGIN
  WriteLn()
END Hello;
BEGIN
END Main.
"#,
    )
    .expect("module with export markers and qualified type refs should parse");

    assert_eq!(module.imports.len(), 1);
    assert_eq!(module.imports[0].local_name, "B");
    assert_eq!(module.imports[0].external_name, "ModuleB");

    let type_decl = module
        .declarations
        .iter()
        .find_map(|decl| match decl {
            crate::ast::Declaration::Type {
                name,
                target,
                is_exported,
            } => Some((name, target, is_exported)),
            _ => None,
        })
        .expect("expected a type declaration");
    assert_eq!(type_decl.0, "LocalType");
    assert!(*type_decl.2);
    assert!(matches!(
        type_decl.1,
        crate::ast::TypeRef::Qualified { module, name }
        if module == "B" && name == "IntType"
    ));

    let proc_decl = module
        .declarations
        .iter()
        .find_map(|decl| match decl {
            crate::ast::Declaration::Procedure {
                name, is_exported, ..
            } => Some((name, is_exported)),
            _ => None,
        })
        .expect("expected a procedure declaration");
    assert_eq!(proc_decl.0, "Hello");
    assert!(*proc_decl.1);
}

#[test]
fn parses_qualified_call_and_qualified_variable_expression() {
    let parsed = parse_module(
        r#"
MODULE Main;
VAR x: INTEGER;
BEGIN
  B.HELLO;
  x := B.value
END Main.
"#,
    )
    .expect("module with qualified names should parse");

    let Statement::Call { module, name, args } = &parsed.statements[0] else {
        panic!("expected first statement to be a call");
    };
    assert_eq!(module.as_deref(), Some("B"));
    assert_eq!(name, "HELLO");
    assert!(args.is_empty());

    let Statement::Assign { target, value } = &parsed.statements[1] else {
        panic!("expected second statement to be an assignment");
    };
    assert_eq!(target, &AssignTarget::Name("x".to_string()));
    assert!(matches!(
        value,
        Expr::QualifiedVariable { module, name }
        if module == "B" && name == "value"
    ));
}

#[test]
fn parses_indexed_designators_in_assignments_and_expressions() {
    let parsed = parse_module(
        r#"
MODULE Main;
VAR arr: ARRAY 4 OF INTEGER;
    i: INTEGER;
    x: INTEGER;
BEGIN
  arr[i] := 1;
  x := arr[0]
END Main.
"#,
    )
    .expect("module with indexed designators should parse");

    let Statement::Assign { target, value } = &parsed.statements[0] else {
        panic!("expected first statement to be an assignment");
    };

    assert!(matches!(
        target,
        AssignTarget::Indexed { name, index }
        if name == "arr" && matches!(index, Expr::Variable(var) if var == "i")
    ));
    assert!(matches!(value, Expr::Integer(1)));

    let Statement::Assign { target, value } = &parsed.statements[1] else {
        panic!("expected second statement to be an assignment");
    };

    assert_eq!(target, &AssignTarget::Name("x".to_string()));
    assert!(matches!(
        value,
        Expr::Indexed { name, index }
        if name == "arr" && matches!(index.as_ref(), Expr::Integer(0))
    ));
}

#[test]
fn parses_array_length_constant_expression() {
    let module = parse_module(
        r#"
MODULE Main;
CONST n = 2;
TYPE
  Vec = ARRAY n + 3 OF INTEGER;
BEGIN
END Main.
"#,
    )
    .expect("array length constant expression should parse");

    let decl = module
        .declarations
        .iter()
        .find_map(|decl| match decl {
            crate::ast::Declaration::Type { name, target, .. } if name == "Vec" => Some(target),
            _ => None,
        })
        .expect("type declaration Vec should exist");

    assert!(matches!(
        decl,
        crate::ast::TypeRef::Array { length, element_type }
            if matches!(
                length,
                Expr::Binary { op: BinaryOp::Add, left, right }
                    if matches!(left.as_ref(), Expr::Variable(name) if name == "n")
                        && matches!(right.as_ref(), Expr::Integer(3))
            )
            && matches!(element_type.as_ref(), crate::ast::TypeRef::Integer)
    ));
}

#[test]
fn rejects_multiple_index_selectors_for_current_subset() {
    let err = parse_module(
        r#"
MODULE Main;
VAR matrix: ARRAY 2 OF INTEGER;
    x: INTEGER;
BEGIN
  x := matrix[0][1]
END Main.
"#,
    )
    .expect_err("multi-index designators should be rejected in current subset");

    assert!(
        err.to_string()
            .contains("Multiple index selectors are not yet supported")
    );
}

#[test]
fn parses_zero_arg_call_expressions_as_calls() {
    let parsed = parse_module(
        r#"
MODULE Main;
VAR x: INTEGER;
BEGIN
  x := ReadInt();
  IF EOF() THEN
    x := 1
  ELSE
    x := 2
  END
END Main.
"#,
    )
    .expect("module with zero-arg call expressions should parse");

    let Statement::Assign { value, .. } = &parsed.statements[0] else {
        panic!("expected first statement to be an assignment");
    };
    assert!(matches!(
        value,
        Expr::Call { module: None, name, args }
        if name == "ReadInt" && args.is_empty()
    ));

    let Statement::If { condition, .. } = &parsed.statements[1] else {
        panic!("expected second statement to be an IF statement");
    };
    assert!(matches!(
        condition,
        Expr::Call { module: None, name, args }
        if name == "EOF" && args.is_empty()
    ));
}

#[test]
fn parses_extended_operator_expression_tree() {
    let module = parse_module(
        r#"
MODULE Main;
VAR x: INTEGER;
VAR flag: BOOLEAN;
BEGIN
  x := -1 + 2 DIV 3 MOD 2;
  flag := ~(1 OR 0) & (1 OR 1)
END Main.
"#,
    )
    .expect("extended operators program should parse");

    let Statement::Assign { value, .. } = &module.statements[0] else {
        panic!("expected first statement to be assignment");
    };

    let Expr::Binary { op, left, right } = value else {
        panic!("expected top-level binary expression");
    };
    assert!(matches!(op, BinaryOp::Add));
    assert!(matches!(
        left.as_ref(),
        Expr::Unary {
            op: UnaryOp::Minus,
            ..
        }
    ));
    assert!(matches!(
        right.as_ref(),
        Expr::Binary {
            op: BinaryOp::Mod,
            ..
        }
    ));

    let Statement::Assign { value, .. } = &module.statements[1] else {
        panic!("expected second statement to be assignment");
    };
    let Expr::Binary { op, left, right } = value else {
        panic!("expected boolean binary expression");
    };
    assert!(matches!(op, BinaryOp::And));
    assert!(matches!(
        left.as_ref(),
        Expr::Unary {
            op: UnaryOp::Not,
            ..
        }
    ));
    assert!(matches!(
        right.as_ref(),
        Expr::Binary {
            op: BinaryOp::Or,
            ..
        }
    ));
}

#[test]
fn parses_relational_operator_expression_tree() {
    let module = parse_module(
        r#"
MODULE Main;
VAR b: BOOLEAN;
BEGIN
  b := (1 + 2) = 3;
  b := 4 # 5;
  b := 1 < 2;
  b := 2 <= 2;
  b := 3 > 2;
  b := 3 >= 3
END Main.
"#,
    )
    .expect("relational operators program should parse");

    let expected_ops = [
        BinaryOp::Eq,
        BinaryOp::Ne,
        BinaryOp::Lt,
        BinaryOp::Le,
        BinaryOp::Gt,
        BinaryOp::Ge,
    ];

    for (stmt, expected_op) in module.statements.iter().zip(expected_ops.iter()) {
        let Statement::Assign { value, .. } = stmt else {
            panic!("expected assignment statement");
        };
        let Expr::Binary { op, .. } = value else {
            panic!("expected relational binary expression");
        };
        assert!(std::mem::discriminant(op) == std::mem::discriminant(expected_op));
    }
}
