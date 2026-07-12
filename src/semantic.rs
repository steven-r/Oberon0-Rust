use std::error::Error;
use std::fmt;
use std::collections::HashMap;

use anyhow::Result;

use crate::ast::{Declaration, Expr, Module, Statement};
use crate::manifest::ExternalManifest;
use crate::symbols::{SymbolKind, SymbolTable};

#[derive(Debug, Clone)]
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
    NotCallable { name: String },
    ProcedureNameMismatch { expected: String, got: String },
}

impl SemanticError {
    pub fn code(&self) -> &'static str {
        match self {
            SemanticError::ModuleNameMismatch { .. } => "E001",
            SemanticError::DuplicateImportAlias { .. } => "E002",
            SemanticError::UnmappedImport { .. } => "E003",
            SemanticError::DuplicateSymbol { .. } => "E004",
            SemanticError::UndefinedSymbol { .. } => "E005",
            SemanticError::ArityMismatch { .. } => "E006",
            SemanticError::NotCallable { .. } => "E007",
            SemanticError::ProcedureNameMismatch { .. } => "E008",
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
    let mut proc_arity: HashMap<String, Option<usize>> = HashMap::new();
    proc_arity.insert("WriteInt".to_string(), None);

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

fn analyze_expr(expr: &Expr, symbols: &SymbolTable) -> Result<()> {
    match expr {
        Expr::Integer(_) => Ok(()),
        Expr::Variable(name) => {
            if symbols.resolve(name).is_none() {
                return Err(SemanticError::UndefinedSymbol { name: name.clone() }.into());
            }
            Ok(())
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
}
