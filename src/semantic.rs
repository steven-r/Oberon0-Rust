//! Semantic checks for name resolution, declaration validity, and call arity.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use anyhow::Result;

use crate::ast::{Declaration, Expr, Module, Statement};
use crate::manifest::ExternalManifest;
use crate::symbols::{SymbolKind, SymbolTable};

#[derive(Debug, Clone)]
/// User-facing semantic failures reported after parsing succeeds.
pub enum SemanticError {
    ModuleNameMismatch { expected: String, got: String },
    DuplicateImportAlias { alias: String },
    UnmappedImport { import: String },
    DuplicateSymbol { name: String },
    UndefinedSymbol { name: String },
    ArityMismatch {
        name: String,
        expected: usize,
        got: usize,
    },
    InvalidBuiltinArgument {
        name: String,
        detail: String,
    },
    UnsupportedStringLiteral,
    NotCallable { name: String },
    ProcedureNameMismatch { expected: String, got: String },
}

impl SemanticError {
    /// Stable diagnostic code used in error messages and tests.
    pub fn code(&self) -> &'static str {
        match self {
            SemanticError::ModuleNameMismatch { .. } => "E001",
            SemanticError::DuplicateImportAlias { .. } => "E002",
            SemanticError::UnmappedImport { .. } => "E003",
            SemanticError::DuplicateSymbol { .. } => "E004",
            SemanticError::UndefinedSymbol { .. } => "E005",
            SemanticError::ArityMismatch { .. } => "E006",
            SemanticError::InvalidBuiltinArgument { .. } => "E007",
            SemanticError::UnsupportedStringLiteral => "E008",
            SemanticError::NotCallable { .. } => "E009",
            SemanticError::ProcedureNameMismatch { .. } => "E010",
        }
    }
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticError::ModuleNameMismatch { expected, got } => {
                write!(
                    f,
                    "[{}] Module name mismatch at END: expected '{}', got '{}'",
                    self.code(),
                    expected,
                    got
                )
            }
            SemanticError::DuplicateImportAlias { alias } => {
                write!(f, "[{}] Duplicate import alias: '{}'", self.code(), alias)
            }
            SemanticError::UnmappedImport { import } => {
                write!(
                    f,
                    "[{}] Import '{}' is not mapped to a crate in the manifest",
                    self.code(),
                    import
                )
            }
            SemanticError::DuplicateSymbol { name } => {
                write!(f, "[{}] Duplicate symbol declaration: '{}'", self.code(), name)
            }
            SemanticError::UndefinedSymbol { name } => {
                write!(f, "[{}] Undefined symbol usage: '{}'", self.code(), name)
            }
            SemanticError::ArityMismatch {
                name,
                expected,
                got,
            } => {
                write!(
                    f,
                    "[{}] Procedure '{}' called with wrong arity: expected {}, got {}",
                    self.code(),
                    name,
                    expected,
                    got
                )
            }
            SemanticError::InvalidBuiltinArgument { name, detail } => {
                write!(f, "[{}] Builtin '{}' received an invalid argument: {}", self.code(), name, detail)
            }
            SemanticError::UnsupportedStringLiteral => {
                write!(f, "[{}] String literals are only supported as arguments to 'WriteString'", self.code())
            }
            SemanticError::NotCallable { name } => {
                write!(f, "[{}] Symbol '{}' is not callable", self.code(), name)
            }
            SemanticError::ProcedureNameMismatch { expected, got } => {
                write!(
                    f,
                    "[{}] Procedure END name mismatch: expected '{}', got '{}'",
                    self.code(),
                    expected,
                    got
                )
            }
        }
    }
}

impl Error for SemanticError {}

/// Validates module structure, scope rules, and procedure calls before lowering.
pub fn analyze(module: &Module, manifest: Option<&ExternalManifest>) -> Result<()> {
    if module.name != module.end_name {
        return Err(SemanticError::ModuleNameMismatch {
            expected: module.name.clone(),
            got: module.end_name.clone(),
        }
        .into());
    }

    let mut symbols = SymbolTable::new();
    symbols.declare("WriteInt", SymbolKind::Procedure)?;
    symbols.declare("WriteString", SymbolKind::Procedure)?;
    symbols.declare("WriteLn", SymbolKind::Procedure)?;
    symbols.declare("ReadInt", SymbolKind::Procedure)?;
    symbols.declare("EOF", SymbolKind::Procedure)?;
    let mut proc_arity: HashMap<String, Option<usize>> = HashMap::new();
    proc_arity.insert("WriteInt".to_string(), None);
    proc_arity.insert("WriteString".to_string(), Some(1));
    proc_arity.insert("WriteLn".to_string(), Some(0));
    proc_arity.insert("ReadInt".to_string(), Some(0));
    proc_arity.insert("EOF".to_string(), Some(0));

    for import in &module.imports {
        if symbols
            .declare(&import.local_name, SymbolKind::Procedure)
            .is_err()
        {
            return Err(SemanticError::DuplicateImportAlias {
                alias: import.local_name.clone(),
            }
            .into());
        }

        if let Some(m) = manifest
            && m.resolve(&import.external_name).is_none()
        {
            return Err(SemanticError::UnmappedImport {
                import: import.external_name.clone(),
            }
            .into());
        }
    }

    for declaration in &module.declarations {
        match declaration {
            Declaration::Const { name, .. } => {
                symbols.declare(name, SymbolKind::Constant)?;
            }
            Declaration::Var { name } => {
                symbols.declare(name, SymbolKind::Variable)?;
            }
            Declaration::Procedure { name, params, .. } => {
                symbols.declare(name, SymbolKind::Procedure)?;
                proc_arity.insert(name.clone(), Some(params.len()));
            }
        }
    }

    for declaration in &module.declarations {
        if let Declaration::Procedure {
            name,
            params,
            body,
            end_name,
        } = declaration
        {
            if name != end_name {
                return Err(SemanticError::ProcedureNameMismatch {
                    expected: name.clone(),
                    got: end_name.clone(),
                }
                .into());
            }

            symbols.enter_scope();
            for param in params {
                symbols.declare(param, SymbolKind::Parameter)?;
            }
            for statement in body {
                analyze_statement(statement, &mut symbols, &proc_arity)?;
            }
            symbols.exit_scope();
        }
    }

    for statement in &module.statements {
        analyze_statement(statement, &mut symbols, &proc_arity)?;
    }

    Ok(())
}

/// Validates one statement within the current symbol-table scope.
fn analyze_statement(
    stmt: &Statement,
    symbols: &mut SymbolTable,
    proc_arity: &HashMap<String, Option<usize>>,
) -> Result<()> {
    match stmt {
        Statement::Assign { target, value } => {
            analyze_expr(value, symbols)?;
            if symbols.resolve(target).is_none() {
                return Err(SemanticError::UndefinedSymbol {
                    name: target.clone(),
                }
                .into());
            }
            Ok(())
        }
        Statement::Call { name, args } => {
            if name == "ReadInt" || name == "EOF" {
                return Err(SemanticError::InvalidBuiltinArgument {
                    name: name.clone(),
                    detail: "must be used as a call expression (e.g. x := ReadInt(), IF EOF() THEN ...)"
                        .to_string(),
                }
                .into());
            }

            if name == "WriteString" {
                if args.len() != 1 {
                    return Err(SemanticError::ArityMismatch {
                        name: name.clone(),
                        expected: 1,
                        got: args.len(),
                    }
                    .into());
                }

                return match args.first() {
                    Some(Expr::String(_)) => Ok(()),
                    Some(_) => Err(SemanticError::InvalidBuiltinArgument {
                        name: name.clone(),
                        detail: "expected a string literal".to_string(),
                    }
                    .into()),
                    None => unreachable!("arity checked above"),
                };
            }

            let symbol = symbols.resolve(name).ok_or_else(|| SemanticError::UndefinedSymbol {
                name: name.clone(),
            })?;

            if symbol.kind != SymbolKind::Procedure {
                return Err(SemanticError::NotCallable { name: name.clone() }.into());
            }

            if let Some(Some(expected)) = proc_arity.get(name)
                && args.len() != *expected
            {
                return Err(SemanticError::ArityMismatch {
                    name: name.clone(),
                    expected: *expected,
                    got: args.len(),
                }
                .into());
            }

            for arg in args {
                analyze_expr(arg, symbols)?;
            }

            Ok(())
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            analyze_expr(condition, symbols)?;
            for stmt in then_branch {
                analyze_statement(stmt, symbols, proc_arity)?;
            }
            if let Some(else_branch) = else_branch {
                for stmt in else_branch {
                    analyze_statement(stmt, symbols, proc_arity)?;
                }
            }
            Ok(())
        }
        Statement::While { condition, body } => {
            analyze_expr(condition, symbols)?;
            for stmt in body {
                analyze_statement(stmt, symbols, proc_arity)?;
            }
            Ok(())
        }
    }
}

/// Validates an expression and ensures every referenced symbol is defined.
fn analyze_expr(expr: &Expr, symbols: &SymbolTable) -> Result<()> {
    match expr {
        Expr::Integer(_) => Ok(()),
        Expr::String(_) => Err(SemanticError::UnsupportedStringLiteral.into()),
        Expr::Variable(name) => {
            if symbols.resolve(name).is_none() {
                return Err(SemanticError::UndefinedSymbol { name: name.clone() }.into());
            }
            Ok(())
        }
        Expr::Call { name, args } => {
            if name == "ReadInt" || name == "EOF" {
                if !args.is_empty() {
                    return Err(SemanticError::ArityMismatch {
                        name: name.clone(),
                        expected: 0,
                        got: args.len(),
                    }
                    .into());
                }
                return Ok(());
            }

            if symbols.resolve(name).is_none() {
                return Err(SemanticError::UndefinedSymbol { name: name.clone() }.into());
            }

            Err(SemanticError::InvalidBuiltinArgument {
                name: name.clone(),
                detail: "call expressions currently support only ReadInt() and EOF()".to_string(),
            }
            .into())
        }
        Expr::Binary { left, right, .. } => {
            analyze_expr(left, symbols)?;
            analyze_expr(right, symbols)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SemanticError, analyze};
    use crate::parser::parse_module;

    fn semantic_error(source: &str) -> SemanticError {
        let module = parse_module(source).expect("source should parse for semantic test");
        let err = analyze(&module, None).expect_err("semantic analysis should fail");
        err.downcast::<SemanticError>()
            .expect("error should downcast to SemanticError")
    }

    #[test]
    fn reports_not_callable_for_variable_call() {
        let source = r#"
MODULE Main;
VAR x;
BEGIN
  x := 1;
  x()
END Main.
"#;
        let err = semantic_error(source);
        match err {
            SemanticError::NotCallable { name } => assert_eq!(name, "x"),
            other => panic!("expected NotCallable, got {other:?}"),
        }
    }

    #[test]
    fn reports_arity_mismatch_for_procedure_call() {
        let source = r#"
MODULE Main;
PROCEDURE P(a, b);
BEGIN
  WriteInt(a + b)
END P;
BEGIN
  P(1)
END Main.
"#;
        let err = semantic_error(source);
        match err {
            SemanticError::ArityMismatch {
                name,
                expected,
                got,
            } => {
                assert_eq!(name, "P");
                assert_eq!(expected, 2);
                assert_eq!(got, 1);
            }
            other => panic!("expected ArityMismatch, got {other:?}"),
        }
    }

    #[test]
    fn reports_procedure_end_name_mismatch() {
        let source = r#"
MODULE Main;
PROCEDURE P(a);
BEGIN
  WriteInt(a)
END Wrong;
BEGIN
END Main.
"#;
        let err = semantic_error(source);
        match err {
            SemanticError::ProcedureNameMismatch { expected, got } => {
                assert_eq!(expected, "P");
                assert_eq!(got, "Wrong");
            }
            other => panic!("expected ProcedureNameMismatch, got {other:?}"),
        }
    }

    #[test]
    fn reports_undefined_symbol_for_undeclared_assignment_target() {
        let source = r#"
MODULE Main;
BEGIN
  y := 1
END Main.
"#;
        let err = semantic_error(source);
        match err {
            SemanticError::UndefinedSymbol { name } => assert_eq!(name, "y"),
            other => panic!("expected UndefinedSymbol, got {other:?}"),
        }
    }

        #[test]
        fn reports_stable_error_code_for_undeclared_assignment_target() {
            let source = r#"
MODULE Main;
BEGIN
    y := 1
END Main.
"#;
            let err = semantic_error(source);
            assert_eq!(err.code(), "E005");
        }

        #[test]
        fn accepts_write_string_with_pascal_style_literal() {
            let source = r#"
    MODULE Main;
    BEGIN
      WriteString("Hello, ""Oberon""")
    END Main.
    "#;

            let module = parse_module(source).expect("source should parse");
            analyze(&module, None).expect("WriteString with a string literal should pass semantic analysis");
        }

        #[test]
        fn rejects_string_literal_outside_write_string() {
            let source = r#"
    MODULE Main;
    VAR x;
    BEGIN
      x := "Hello"
    END Main.
    "#;

            let err = semantic_error(source);
            match err {
                SemanticError::UnsupportedStringLiteral => {}
                other => panic!("expected UnsupportedStringLiteral, got {other:?}"),
            }
        }

        #[test]
        fn rejects_non_string_argument_to_write_string() {
            let source = r#"
    MODULE Main;
    BEGIN
      WriteString(1)
    END Main.
    "#;

            let err = semantic_error(source);
            match err {
                SemanticError::InvalidBuiltinArgument { name, detail } => {
                    assert_eq!(name, "WriteString");
                    assert_eq!(detail, "expected a string literal");
                }
                other => panic!("expected InvalidBuiltinArgument, got {other:?}"),
            }
        }

        #[test]
        fn accepts_writeln_without_arguments() {
            let source = r#"
    MODULE Main;
    BEGIN
      WriteLn()
    END Main.
    "#;

            let module = parse_module(source).expect("source should parse");
            analyze(&module, None).expect("WriteLn without arguments should pass semantic analysis");
        }

        #[test]
        fn rejects_writeln_with_arguments() {
            let source = r#"
    MODULE Main;
    BEGIN
      WriteLn(1)
    END Main.
    "#;

            let err = semantic_error(source);
            match err {
                SemanticError::ArityMismatch {
                    name,
                    expected,
                    got,
                } => {
                    assert_eq!(name, "WriteLn");
                    assert_eq!(expected, 0);
                    assert_eq!(got, 1);
                }
                other => panic!("expected ArityMismatch, got {other:?}"),
            }
        }

                #[test]
                fn accepts_readint_call_expression_in_assignment() {
                        let source = r#"
        MODULE Main;
        VAR x;
        BEGIN
            x := ReadInt()
        END Main.
        "#;

                        let module = parse_module(source).expect("source should parse");
                        analyze(&module, None).expect("ReadInt call expression should pass semantic analysis");
                }

                #[test]
                fn accepts_eof_call_expression_in_if_condition() {
                        let source = r#"
        MODULE Main;
        BEGIN
            IF EOF() THEN
                WriteLn()
            END
        END Main.
        "#;

                        let module = parse_module(source).expect("source should parse");
                        analyze(&module, None).expect("EOF call expression should pass semantic analysis");
                }

                #[test]
                fn rejects_readint_as_statement_call() {
                        let source = r#"
        MODULE Main;
        BEGIN
            ReadInt()
        END Main.
        "#;

                        let err = semantic_error(source);
                        match err {
                                SemanticError::InvalidBuiltinArgument { name, detail } => {
                                        assert_eq!(name, "ReadInt");
                                        assert!(detail.contains("must be used as a call expression"));
                                }
                                other => panic!("expected InvalidBuiltinArgument, got {other:?}"),
                        }
                }

                #[test]
                fn rejects_call_expression_for_non_function_builtin() {
                        let source = r#"
        MODULE Main;
        VAR x;
        BEGIN
            x := WriteInt(1)
        END Main.
        "#;

                        let err = semantic_error(source);
                        match err {
                                SemanticError::InvalidBuiltinArgument { name, detail } => {
                                        assert_eq!(name, "WriteInt");
                                        assert!(detail.contains("currently support only ReadInt() and EOF()"));
                                }
                                other => panic!("expected InvalidBuiltinArgument, got {other:?}"),
                        }
                }
}
