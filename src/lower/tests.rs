
use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use super::{Resolver, lower_expr, lower_module, lower_statement};
use crate::ast::{Expr, Statement, TypeRef};
use crate::hir::{HDeclaration, HExpr, HStatement, HTarget};
use crate::parser::parse_module;
use crate::semantic::analyze;
use crate::symbols::SymbolKind;

fn lower_from_source(source: &str) -> Result<crate::hir::HModule> {
    let module = parse_module(source)?;
    analyze(&module, None)?;
    lower_module(&module)
}

fn lower_from_fixture(name: &str) -> Result<crate::hir::HModule> {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("lower")
        .join(name);
    let source = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|err| panic!("fixture should be readable: {err}"));
    lower_from_source(&source)
}

fn collect_assign_target_ids(stmts: &[HStatement], out: &mut Vec<usize>) {
    for stmt in stmts {
        match stmt {
            HStatement::Assign { target, .. } => match target {
                crate::hir::HTarget::Name(ident) => out.push(ident.id),
                crate::hir::HTarget::Indexed { name, .. } => out.push(name.id),
            },
            HStatement::If {
                then_branch,
                else_branch,
                ..
            } => {
                collect_assign_target_ids(then_branch, out);
                if let Some(else_branch) = else_branch {
                    collect_assign_target_ids(else_branch, out);
                }
            }
            HStatement::While { body, .. } => collect_assign_target_ids(body, out),
            HStatement::Call { .. } => {}
        }
    }
}

#[test]
fn procedure_locals_and_params_have_stable_ids_in_nested_flow() {
    let source = r#"
MODULE Main;
VAR x: INTEGER;
PROCEDURE P(p: INTEGER);
BEGIN
  IF p > 0 THEN
    x := p;
    WHILE p > 0 DO
      x := x + 1
    END
  END
END P;
BEGIN
  P(1)
END Main.
"#;

    let hir = lower_from_source(source).expect("lowering should succeed");

    let module_x_id = hir
        .declarations
        .iter()
        .find_map(|decl| match decl {
            HDeclaration::Var { id, name, .. } if name == "x" => Some(*id),
            _ => None,
        })
        .expect("module variable x must exist");

    let proc = hir
        .declarations
        .iter()
        .find_map(|decl| match decl {
            HDeclaration::Procedure {
                name,
                params,
                local_vars,
                body,
                ..
            } if name == "P" => Some((params, local_vars, body)),
            _ => None,
        })
        .expect("procedure P must exist");

    assert_eq!(proc.0.len(), 1, "expected exactly one parameter");
    assert_eq!(
        proc.1.len(),
        0,
        "expected no implicit procedure local variable"
    );

    let mut assign_ids = Vec::new();
    collect_assign_target_ids(proc.2, &mut assign_ids);
    assert_eq!(assign_ids.len(), 2, "expected two assignments to x");
    assert_eq!(
        assign_ids[0], module_x_id,
        "first assignment must target declared module var x"
    );
    assert_eq!(
        assign_ids[1], module_x_id,
        "nested assignment must reuse declared module var id"
    );

    if let HStatement::If { condition, .. } = &proc.2[0] {
        match condition {
            HExpr::Binary { left, .. } => match left.as_ref() {
                HExpr::Name(ident) => assert_eq!(ident.id, proc.0[0].id),
                _ => panic!("IF condition must resolve to parameter identifier"),
            },
            _ => panic!("IF condition must resolve to parameter identifier"),
        }
    } else {
        panic!("expected IF as first procedure statement");
    }
}

#[test]
fn typed_declarations_survive_lowering_with_preserved_type_info() {
    let source = r#"
MODULE Main;
TYPE Count = REAL;
VAR x: Count;
BEGIN
  x := 1.0
END Main.
"#;

    let hir = lower_from_source(source).expect("lowering should succeed");

    let type_decl = hir
        .declarations
        .iter()
        .find_map(|decl| match decl {
            HDeclaration::Type { name, target, .. } if name == "Count" => Some(target.clone()),
            _ => None,
        })
        .expect("type declaration Count must exist in HIR");
    assert!(matches!(type_decl, TypeRef::Real));

    let var_type = hir
        .declarations
        .iter()
        .find_map(|decl| match decl {
            HDeclaration::Var {
                name,
                declared_type,
                ..
            } if name == "x" => declared_type.clone(),
            _ => None,
        })
        .expect("variable x must carry declared type info in HIR");
    assert!(matches!(var_type, TypeRef::Named(name) if name == "Count"));
}

#[test]
fn typed_formal_parameters_survive_lowering_with_var_mode() {
    let source = r#"
MODULE Main;
PROCEDURE Bump(VAR target: INTEGER; amount: LONGREAL);
BEGIN
END Bump;
BEGIN
END Main.
"#;

    let hir = lower_from_source(source).expect("lowering should succeed");

    let params = hir
        .declarations
        .iter()
        .find_map(|decl| match decl {
            HDeclaration::Procedure { name, params, .. } if name == "Bump" => Some(params.clone()),
            _ => None,
        })
        .expect("procedure Bump must exist in HIR");

    assert_eq!(params.len(), 2);
    assert_eq!(params[0].name, "target");
    assert!(params[0].is_var);
    assert!(matches!(params[0].declared_type, Some(TypeRef::Integer)));

    assert_eq!(params[1].name, "amount");
    assert!(!params[1].is_var);
    assert!(matches!(params[1].declared_type, Some(TypeRef::LongReal)));
}

#[test]
fn fixture_const_and_call_lowering_keeps_constants_and_calls_in_hir() {
    let hir = lower_from_fixture("const_and_call_lowering.ob0")
        .expect("fixture-based lowering should succeed");

    let const_values = hir
        .declarations
        .iter()
        .filter_map(|decl| match decl {
            HDeclaration::Const { name, value, .. } => Some((name.as_str(), value)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(const_values.len(), 2);
    assert!(matches!(const_values[0].1, HExpr::Integer(2)));
    assert!(matches!(const_values[1].1, HExpr::Integer(3)));

    let call_stmt = hir
        .statements
        .iter()
        .find_map(|stmt| match stmt {
            HStatement::Call { name, .. } => Some(name.clone()),
            _ => None,
        })
        .expect("expected a call statement in the lowered HIR");
    assert_eq!(call_stmt.name, "WriteInt");
}

#[test]
fn fixture_loop_and_if_lowering_preserves_nested_control_flow() {
    let hir = lower_from_fixture("loop_and_if_lowering.ob0")
        .expect("fixture-based lowering should succeed");

    let if_stmt = hir
        .statements
        .iter()
        .find_map(|stmt| match stmt {
            HStatement::If {
                condition,
                then_branch,
                else_branch,
            } => Some((condition, then_branch, else_branch)),
            _ => None,
        })
        .expect("expected an IF statement in the lowered HIR");

    assert!(matches!(if_stmt.0, HExpr::Binary { .. }));
    assert_eq!(if_stmt.1.len(), 1);
    assert!(if_stmt.2.is_none());

    let while_stmt = hir
        .statements
        .iter()
        .find_map(|stmt| match stmt {
            HStatement::While { condition, body } => Some((condition, body)),
            _ => None,
        })
        .expect("expected a WHILE statement in the lowered HIR");

    assert!(matches!(while_stmt.0, HExpr::Binary { .. }));
    assert_eq!(while_stmt.1.len(), 1);
}

#[test]
fn fixture_qualified_expression_lowering_reports_unsupported_input() {
    let err = lower_from_fixture("qualified_imports.ob0").unwrap_err();
    let message = err.to_string();

    assert!(
        message.contains("Qualified variables are not yet supported in code generation")
            || message.contains("unknown call target"),
        "unexpected lowering error: {message}"
    );
}

#[test]
fn fixture_literal_variants_lower_to_expected_hir_nodes() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("lower")
            .join("expression_literal_variants.ob0"),
    )
    .expect("fixture should be readable");
    let module = parse_module(&source).expect("fixture should parse");
    let hir = lower_module(&module).expect("fixture-based lowering should succeed");

    let statements = &hir.statements;
    assert_eq!(statements.len(), 3);

    let mut saw_real = false;
    let mut saw_boolean = false;

    for stmt in statements {
        if let HStatement::Assign { value, .. } = stmt {
            match value {
                HExpr::Real(_) => saw_real = true,
                HExpr::Boolean(_) => saw_boolean = true,
                _ => {}
            }
        }
    }

    let resolver = Resolver::new();
    let long_real_expr = lower_expr(&Expr::LongReal(2.5), &resolver)
        .expect("longreal literal lowering should succeed");
    let string_expr = lower_expr(&Expr::String("hello".to_string()), &resolver)
        .expect("string literal lowering should succeed");

    assert!(saw_real);
    assert!(matches!(long_real_expr, HExpr::LongReal(2.5)));
    assert!(saw_boolean);
    assert!(matches!(string_expr, HExpr::String(value) if value == "hello"));
}

#[test]
fn fixture_qualified_variable_expression_is_rejected_by_lowerer() {
    let resolver = Resolver::new();
    let err = lower_expr(
        &Expr::QualifiedVariable {
            module: "B".to_string(),
            name: "value".to_string(),
        },
        &resolver,
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("Qualified variables are not yet supported in code generation")
    );
}

#[test]
fn lower_statement_preserves_if_else_branches() {
    let source = r#"
MODULE Main;
BEGIN
  IF TRUE THEN
    WriteLn
  ELSE
    WriteInt(1)
  END
END Main.
"#;

    let module = parse_module(source).expect("source should parse");
    let hir = lower_module(&module).expect("lowering should succeed");

    let if_stmt = hir.statements.iter().find_map(|stmt| match stmt {
        HStatement::If {
            then_branch,
            else_branch,
            ..
        } => Some((then_branch, else_branch)),
        _ => None,
    });

    let (then_branch, else_branch) = if_stmt.expect("expected IF statement");
    assert_eq!(then_branch.len(), 1);
    assert!(else_branch.is_some());
    assert_eq!(else_branch.as_ref().unwrap().len(), 1);
}

#[test]
fn lower_statement_preserves_call_module_metadata() {
    let mut resolver = Resolver::new();
    resolver
        .declare("WriteInt", SymbolKind::Procedure)
        .expect("builtin procedure should be declared");

    let stmt = lower_statement(
        &Statement::Call {
            module: Some("B".to_string()),
            name: "WriteInt".to_string(),
            args: vec![],
        },
        &mut resolver,
    )
    .expect("call statement should lower");

    match stmt {
        HStatement::Call { module, name, .. } => {
            assert_eq!(module, Some("B".to_string()));
            assert_eq!(name.name, "WriteInt");
        }
        other => panic!("unexpected lowered statement: {other:?}"),
    }
}

#[test]
fn lowering_reports_duplicate_symbol_declarations() {
    let source = r#"
MODULE Main;
VAR x: INTEGER;
VAR x: INTEGER;
BEGIN
END Main.
"#;

    let module = parse_module(source).expect("source should parse");
    let err = lower_module(&module).unwrap_err();
    assert!(err.to_string().contains("duplicate symbol declaration"));
}

#[test]
fn lowering_reports_unknown_identifier_in_expression() {
    let source = r#"
MODULE Main;
BEGIN
  x := 1
END Main.
"#;

    let module = parse_module(source).expect("source should parse");
    let err = lower_module(&module).unwrap_err();
    assert!(
        err.to_string()
            .contains("Lowering failed: unknown identifier")
            || err.to_string().contains("unresolved assignment target")
    );
}

#[test]
fn lowering_reports_unknown_call_target() {
    let source = r#"
MODULE Main;
BEGIN
  Missing(1)
END Main.
"#;

    let module = parse_module(source).expect("source should parse");
    let err = lower_module(&module).unwrap_err();
    assert!(err.to_string().contains("unknown call target"));
}

#[test]
fn lower_expr_reports_unknown_variable_indexed_and_call_targets() {
    let resolver = Resolver::new();

    let indexed_err = lower_expr(
        &Expr::Indexed {
            name: "missing".to_string(),
            index: Box::new(Expr::Integer(0)),
        },
        &resolver,
    )
    .expect_err("unknown indexed identifier should fail lowering");
    assert!(
        indexed_err
            .to_string()
            .contains("Lowering failed: unknown identifier 'missing'.")
    );

    let variable_err = lower_expr(&Expr::Variable("missing".to_string()), &resolver)
        .expect_err("unknown variable should fail lowering");
    assert!(
        variable_err
            .to_string()
            .contains("Lowering failed: unknown identifier 'missing'.")
    );

    let call_err = lower_expr(
        &Expr::Call {
            module: None,
            name: "MissingProc".to_string(),
            args: vec![],
        },
        &resolver,
    )
    .expect_err("unknown call target should fail lowering");
    assert!(
        call_err
            .to_string()
            .contains("Lowering failed: unknown call target 'MissingProc'.")
    );
}

#[test]
fn lower_assign_target_indexed_covers_success_and_error_paths() {
    let mut resolver = Resolver::new();
    let declared = resolver
        .declare("arr", SymbolKind::Variable)
        .expect("array symbol should be declared");

    let lowered = super::lower_assign_target(
        &crate::ast::AssignTarget::Indexed {
            name: "arr".to_string(),
            index: Expr::Integer(2),
        },
        &resolver,
    )
    .expect("indexed assignment target should lower");

    assert!(matches!(
        lowered,
        HTarget::Indexed { name, index }
            if name.id == declared.id && matches!(index, HExpr::Integer(2))
    ));

    let missing_err = super::lower_assign_target(
        &crate::ast::AssignTarget::Indexed {
            name: "missing".to_string(),
            index: Expr::Integer(0),
        },
        &resolver,
    )
    .expect_err("unresolved indexed assignment target should fail");

    assert!(
        missing_err
            .to_string()
            .contains("Lowering invariant violated: unresolved assignment target 'missing'.")
    );
}

#[test]
fn lowering_preserves_indexed_targets_and_indexed_reads() {
    let source = r#"
MODULE Main;
VAR arr: ARRAY 4 OF INTEGER;
    i: INTEGER;
    x: INTEGER;
BEGIN
  arr[i] := 7;
  x := arr[0]
END Main.
"#;

    let hir = lower_from_source(source).expect("lowering should succeed");

    let arr_id = hir
        .declarations
        .iter()
        .find_map(|decl| match decl {
            HDeclaration::Var { id, name, .. } if name == "arr" => Some(*id),
            _ => None,
        })
        .expect("array declaration should exist");

    let first = &hir.statements[0];
    match first {
        HStatement::Assign { target, value } => {
            assert!(matches!(value, HExpr::Integer(7)));
            assert!(matches!(
                target,
                HTarget::Indexed { name, index }
                    if name.id == arr_id
                        && matches!(index, HExpr::Name(idx) if idx.name == "i")
            ));
        }
        other => panic!("expected first lowered statement to be assignment, got {other:?}"),
    }

    let second = &hir.statements[1];
    match second {
        HStatement::Assign { value, .. } => {
            assert!(matches!(
                value,
                HExpr::Indexed { name, index }
                    if name.id == arr_id && matches!(index.as_ref(), HExpr::Integer(0))
            ));
        }
        other => panic!("expected second lowered statement to be assignment, got {other:?}"),
    }
}

#[test]
fn procedure_local_vars_survive_lowering_with_stable_ids() {
    let source = r#"
MODULE Main;
PROCEDURE P;
VAR x: INTEGER;
BEGIN
  x := 1
END P;
BEGIN
  P
END Main.
"#;

    let hir = lower_from_source(source).expect("lowering should succeed");

    let (local_vars, body) = hir
        .declarations
        .iter()
        .find_map(|decl| match decl {
            HDeclaration::Procedure {
                name,
                local_vars,
                body,
                ..
            } if name == "P" => Some((local_vars.clone(), body.clone())),
            _ => None,
        })
        .expect("procedure P must exist in HIR");

    assert_eq!(
        local_vars.len(),
        1,
        "procedure P should have one local variable"
    );
    assert_eq!(local_vars[0].name, "x");

    let assigned_id = body
        .iter()
        .find_map(|stmt| match stmt {
            HStatement::Assign { target, .. } => match target {
                crate::hir::HTarget::Name(ident) => Some(ident.id),
                crate::hir::HTarget::Indexed { name, .. } => Some(name.id),
            },
            _ => None,
        })
        .expect("procedure body should assign to local variable x");

    assert_eq!(assigned_id, local_vars[0].id);
}

#[test]
fn lowering_fails_when_assignment_target_is_unresolved() {
    let source = r#"
MODULE Main;
BEGIN
  y := 1
END Main.
"#;

    let module = parse_module(source).expect("source should parse");
    let err =
        lower_module(&module).expect_err("lowering should fail on unresolved assignment target");
    let msg = err.to_string();
    assert!(
        msg.contains("Lowering invariant violated: unresolved assignment target 'y'."),
        "unexpected lowering error message: {msg}"
    );
}

#[test]
fn lowering_keep_const() {
    let source = r#"
MODULE Main;
CONST x = 42;
BEGIN
  WriteInt(x);
  WriteLn()
END Main.
"#;

    let module = parse_module(source).expect("source should parse");
    let hir = lower_module(&module).expect("lowering should succeed");
    let c = hir
        .declarations
        .iter()
        .find_map(|decl| match decl {
            HDeclaration::Const { name, value, .. } if name == "x" => Some(value.clone()),
            _ => None,
        })
        .expect("constant x must exist in HIR");
    assert!(
        matches!(c, HExpr::Integer(42)),
        "constant x should have value 42 in HIR"
    );
}

#[test]
fn lowering_const_negative() {
    let source = r#"
MODULE Main;
CONST x = -1;
BEGIN
  WriteInt(x);
  WriteLn()
END Main.
"#;

    let module = parse_module(source).expect("source should parse");
    let hir = lower_module(&module).expect("lowering should succeed");
    let c = hir
        .declarations
        .iter()
        .find_map(|decl| match decl {
            HDeclaration::Const { name, value, .. } if name == "x" => Some(value.clone()),
            _ => None,
        })
        .expect("constant x must exist in HIR");
    println!("Found constant x with value {:?}", c);
    assert!(
        matches!(c, HExpr::Integer(-1)),
        "constant x should have value -1 in HIR"
    );
}

#[test]
fn lowering_write_string() {
    let source = r#"
MODULE Main;
BEGIN
  WriteString("Hello World")
END Main.
"#;

    let module = parse_module(source).expect("source should parse");
    let hir = lower_module(&module).expect("lowering should succeed");
    let c = hir
        .statements
        .iter()
        .find_map(|decl| match decl {
            HStatement::Call {
                module: _,
                name,
                args,
            } if name.name == "WriteString" => {
                if let HExpr::String(s) = &args[0] {
                    Some(s.clone())
                } else {
                    None
                }
            }
            _ => None,
        })
        .expect("WriteString call must exist in HIR");
    assert_eq!(
        c, "Hello World",
        "WriteString argument should be 'Hello World' in HIR"
    );
}

#[test]
fn lowering_unary_expr() {
    let source = r#"
MODULE Main;
VAR x: INTEGER;
BEGIN
  x := -1;
  WriteInt(x);
  WriteLn()
END Main.
"#;

    let module = parse_module(source).expect("source should parse");
    let hir = lower_module(&module).expect("lowering should succeed");
    let c = hir
        .statements
        .iter()
        .find_map(|decl| match decl {
            HStatement::Assign { target, value, .. } => match target {
                crate::hir::HTarget::Name(ident) => {
                    println!("Found assignment to {} with value {:?}", ident.name, value);
                    if ident.name == "x" {
                        Some(value.clone())
                    } else {
                        None
                    }
                }
                crate::hir::HTarget::Indexed { .. } => None,
            },
            _ => None,
        })
        .expect("assignment to x must exist in HIR");
    if let HExpr::Unary { op, value } = c {
        assert_eq!(
            op,
            crate::ast::UnaryOp::Minus,
            "assignment to x should be a negation in HIR"
        );
        if let HExpr::Integer(i) = *value {
            assert_eq!(i, 1, "assignment to x should be negation of 1 in HIR");
        } else {
            panic!("assignment to x should be negation of an integer in HIR");
        }
    } else {
        panic!("assignment to x should be a unary expression in HIR");
    }
}

#[test]
fn lowering_import() {
    let source = r#"
MODULE Main;
IMPORT LocalMath := Math;
BEGIN
  WriteInt(42);
END Main.
"#;

    let module = parse_module(source).expect("source should parse");
    let hir = lower_module(&module).expect("lowering should succeed");
    let c = hir
        .imports
        .iter()
        .find_map(|decl| {
            Some(decl.clone()).filter(|d| d.local_name == "LocalMath" && d.external_name == "Math")
        })
        .expect("LocalMath import must exist in HIR");
    assert_eq!(
        c.local_name, "LocalMath",
        "import local name should be 'LocalMath' in HIR"
    );
    assert_eq!(
        c.external_name, "Math",
        "import external name should be 'Math' in HIR"
    );
}
