//! Semantic checks for name resolution, declaration validity, and call arity.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use anyhow::Result;

use crate::ast::{Declaration, Expr, LocalVarDecl, Module, ParamDecl, Statement, TypeRef};
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
    InvalidVarArgument {
        name: String,
        position: usize,
        detail: String,
    },
    TypeMismatch {
        detail: String,
    },
    UnknownType { name: String },
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
            SemanticError::InvalidVarArgument { .. } => "E011",
            SemanticError::TypeMismatch { .. } => "E012",
            SemanticError::UnknownType { .. } => "E013",
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
            SemanticError::InvalidVarArgument {
                name,
                position,
                detail,
            } => {
                write!(
                    f,
                    "[{}] Procedure '{}' received an invalid VAR argument at position {}: {}",
                    self.code(),
                    name,
                    position,
                    detail
                )
            }
            SemanticError::TypeMismatch { detail } => {
                write!(f, "[{}] Type mismatch: {}", self.code(), detail)
            }
            SemanticError::UnknownType { name } => {
                write!(f, "[{}] Unknown type reference: '{}'", self.code(), name)
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

fn validate_declared_type(type_ref: &TypeRef, types: &HashMap<String, TypeRef>) -> Result<()> {
    if resolve_type_ref(type_ref, types).is_none() {
        return Err(SemanticError::UnknownType {
            name: match type_ref {
                TypeRef::Integer => "INTEGER".to_string(),
                TypeRef::Boolean => "BOOLEAN".to_string(),
                TypeRef::Real => "REAL".to_string(),
                TypeRef::LongReal => "LONGREAL".to_string(),
                TypeRef::Named(name) => name.clone(),
            },
        }
        .into());
    }

    Ok(())
}

fn validate_declaration_name(name: &str, types: &HashMap<String, TypeRef>) -> Result<()> {
    if types.contains_key(name) {
        return Err(SemanticError::DuplicateSymbol {
            name: name.to_string(),
        }
        .into());
    }

    Ok(())
}

fn is_builtin_type_name(name: &str) -> bool {
    matches!(name, "INTEGER" | "BOOLEAN" | "REAL" | "LONGREAL")
}

fn validate_local_binding_name(
    name: &str,
    declared_type: Option<&TypeRef>,
    types: &HashMap<String, TypeRef>,
) -> Result<()> {
    if is_builtin_type_name(name) {
        return Err(SemanticError::DuplicateSymbol {
            name: name.to_string(),
        }
        .into());
    }

    if let Some(TypeRef::Named(type_name)) = declared_type
        && type_name == name
        && types.contains_key(type_name)
    {
        return Err(SemanticError::DuplicateSymbol {
            name: name.to_string(),
        }
        .into());
    }

    Ok(())
}

fn validate_parameter_name(param: &ParamDecl, types: &HashMap<String, TypeRef>) -> Result<()> {
    validate_local_binding_name(&param.name, param.declared_type.as_ref(), types)
}

fn validate_local_var_name(local_var: &LocalVarDecl, types: &HashMap<String, TypeRef>) -> Result<()> {
    validate_local_binding_name(&local_var.name, local_var.declared_type.as_ref(), types)
}

fn resolve_type_ref(type_ref: &TypeRef, types: &HashMap<String, TypeRef>) -> Option<TypeRef> {
    match type_ref {
        TypeRef::Integer => Some(TypeRef::Integer),
        TypeRef::Boolean => Some(TypeRef::Boolean),
        TypeRef::Real => Some(TypeRef::Real),
        TypeRef::LongReal => Some(TypeRef::LongReal),
        TypeRef::Named(name) => match types.get(name) {
            Some(target) => resolve_type_ref(target, types),
            None => None,
        },
    }
}

fn is_numeric_type(type_ref: &TypeRef) -> bool {
    matches!(type_ref, TypeRef::Integer | TypeRef::Real | TypeRef::LongReal)
}

fn assignment_compatible(expected: &TypeRef, actual: &TypeRef) -> bool {
    match (expected, actual) {
        (TypeRef::Integer, TypeRef::Integer) => true,
        (TypeRef::Real, TypeRef::Integer | TypeRef::Real) => true,
        (TypeRef::LongReal, TypeRef::Integer | TypeRef::Real | TypeRef::LongReal) => true,
        (TypeRef::Boolean, TypeRef::Boolean) => true,
        _ => expected == actual,
    }
}

fn format_type_name(type_ref: &TypeRef) -> &'static str {
    match type_ref {
        TypeRef::Integer => "INTEGER",
        TypeRef::Boolean => "BOOLEAN",
        TypeRef::Real => "REAL",
        TypeRef::LongReal => "LONGREAL",
        TypeRef::Named(_) => "<named>",
    }
}

fn resolve_symbol_type(symbols: &SymbolTable, name: &str) -> Option<TypeRef> {
    symbols
        .resolve(name)
        .and_then(|symbol| symbol.declared_type.clone())
}

fn infer_expr_type(
    expr: &Expr,
    symbols: &SymbolTable,
    types: &HashMap<String, TypeRef>,
) -> Result<Option<TypeRef>> {
    match expr {
        Expr::Integer(_) => Ok(Some(TypeRef::Integer)),
        Expr::String(_) => Err(SemanticError::UnsupportedStringLiteral.into()),
        Expr::Variable(name) => {
            if symbols.resolve(name).is_none() {
                return Err(SemanticError::UndefinedSymbol { name: name.clone() }.into());
            }

            Ok(resolve_symbol_type(symbols, name).and_then(|type_ref| resolve_type_ref(&type_ref, types)))
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
                return Ok(Some(TypeRef::Integer));
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
            let left_type = infer_expr_type(left, symbols, types)?;
            let right_type = infer_expr_type(right, symbols, types)?;

            match (left_type, right_type) {
                (Some(left_type), Some(right_type)) => {
                    if !is_numeric_type(&left_type) || !is_numeric_type(&right_type) {
                        return Err(SemanticError::TypeMismatch {
                            detail: format!(
                                "arithmetic expressions require numeric operands, got {} and {}",
                                format_type_name(&left_type),
                                format_type_name(&right_type)
                            ),
                        }
                        .into());
                    }

                    if left_type == TypeRef::LongReal || right_type == TypeRef::LongReal {
                        Ok(Some(TypeRef::LongReal))
                    } else if left_type == TypeRef::Real || right_type == TypeRef::Real {
                        Ok(Some(TypeRef::Real))
                    } else {
                        Ok(Some(TypeRef::Integer))
                    }
                }
                _ => Ok(None),
            }
        }
    }
}

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
    let mut proc_params: HashMap<String, Vec<ParamDecl>> = HashMap::new();
    proc_arity.insert("WriteInt".to_string(), None);
    proc_arity.insert("WriteString".to_string(), Some(1));
    proc_arity.insert("WriteLn".to_string(), Some(0));
    proc_arity.insert("ReadInt".to_string(), Some(0));
    proc_arity.insert("EOF".to_string(), Some(0));
    let mut types: HashMap<String, TypeRef> = HashMap::new();
    types.insert("INTEGER".to_string(), TypeRef::Integer);
    types.insert("BOOLEAN".to_string(), TypeRef::Boolean);
    types.insert("REAL".to_string(), TypeRef::Real);
    types.insert("LONGREAL".to_string(), TypeRef::LongReal);

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
                validate_declaration_name(name, &types)?;
                symbols.declare(name, SymbolKind::Constant)?;
            }
            Declaration::Type { name, target } => {
                validate_declaration_name(name, &types)?;
                validate_declared_type(target, &types)?;
                symbols.declare_with_type(name, SymbolKind::TypeName, Some(target.clone()))?;
                types.insert(name.clone(), target.clone());
            }
            Declaration::Var {
                name,
                declared_type,
            } => {
                validate_declaration_name(name, &types)?;
                if let Some(type_ref) = declared_type
                {
                    validate_declared_type(type_ref, &types)?;
                }
                symbols.declare_with_type(name, SymbolKind::Variable, declared_type.clone())?;
            }
            Declaration::Procedure {
                name,
                params,
                local_vars,
                ..
            } => {
                validate_declaration_name(name, &types)?;
                for param in params {
                    validate_parameter_name(param, &types)?;
                    if let Some(type_ref) = &param.declared_type {
                        validate_declared_type(type_ref, &types)?;
                    }
                }
                for local_var in local_vars {
                    validate_local_var_name(local_var, &types)?;
                    if let Some(type_ref) = &local_var.declared_type {
                        validate_declared_type(type_ref, &types)?;
                    }
                }
                symbols.declare(name, SymbolKind::Procedure)?;
                proc_arity.insert(name.clone(), Some(params.len()));
                proc_params.insert(name.clone(), params.clone());
            }
        }
    }

    for declaration in &module.declarations {
        if let Declaration::Procedure {
            name,
            params,
            local_vars,
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
                symbols.declare_with_type(
                    &param.name,
                    SymbolKind::Parameter,
                    param.declared_type.clone(),
                )?;
            }
            for local_var in local_vars {
                symbols.declare_with_type(
                    &local_var.name,
                    SymbolKind::Variable,
                    local_var.declared_type.clone(),
                )?;
            }
            for statement in body {
                analyze_statement(statement, &mut symbols, &proc_arity, &proc_params, &types)?;
            }
            symbols.exit_scope();
        }
    }

    for statement in &module.statements {
        analyze_statement(statement, &mut symbols, &proc_arity, &proc_params, &types)?;
    }

    Ok(())
}

fn validate_var_argument(
    proc_name: &str,
    position: usize,
    arg: &Expr,
    symbols: &SymbolTable,
) -> Result<()> {
    match arg {
        Expr::Variable(name) => {
            let symbol = symbols.resolve(name).ok_or_else(|| SemanticError::UndefinedSymbol {
                name: name.clone(),
            })?;

            match symbol.kind {
                SymbolKind::Variable | SymbolKind::Parameter => Ok(()),
                _ => Err(SemanticError::InvalidVarArgument {
                    name: proc_name.to_string(),
                    position,
                    detail: format!("'{}' is not an assignable variable binding", name),
                }
                .into()),
            }
        }
        _ => Err(SemanticError::InvalidVarArgument {
            name: proc_name.to_string(),
            position,
            detail: "expected a variable designator".to_string(),
        }
        .into()),
    }
}

/// Validates one statement within the current symbol-table scope.
fn analyze_statement(
    stmt: &Statement,
    symbols: &mut SymbolTable,
    proc_arity: &HashMap<String, Option<usize>>,
    proc_params: &HashMap<String, Vec<ParamDecl>>,
    types: &HashMap<String, TypeRef>,
) -> Result<()> {
    match stmt {
        Statement::Assign { target, value } => {
            analyze_expr(value, symbols)?;
            let symbol = symbols.resolve(target).ok_or_else(|| SemanticError::UndefinedSymbol {
                name: target.clone(),
            })?;

            if let Some(expected_type) = &symbol.declared_type
                && let Some(actual_type) = infer_expr_type(value, symbols, types)?
            {
                let expected_type = resolve_type_ref(expected_type, types)
                    .expect("declared target type should resolve after semantic validation");
                if !assignment_compatible(&expected_type, &actual_type) {
                    return Err(SemanticError::TypeMismatch {
                        detail: format!(
                            "cannot assign {} to {} '{}'",
                            format_type_name(&actual_type),
                            format_type_name(&expected_type),
                            target
                        ),
                    }
                    .into());
                }
            }

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

            if let Some(params) = proc_params.get(name) {
                for (index, (param, arg)) in params.iter().zip(args.iter()).enumerate() {
                    if param.is_var {
                        validate_var_argument(name, index + 1, arg, symbols)?;
                        if let Some(expected_type) = &param.declared_type
                            && let Some(actual_type) = infer_expr_type(arg, symbols, types)?
                        {
                            let expected_type = resolve_type_ref(expected_type, types)
                                .expect("VAR parameter type should resolve after semantic validation");
                            if expected_type != actual_type {
                                return Err(SemanticError::TypeMismatch {
                                    detail: format!(
                                        "VAR parameter '{}' expects exact type {}, got {}",
                                        param.name,
                                        format_type_name(&expected_type),
                                        format_type_name(&actual_type)
                                    ),
                                }
                                .into());
                            }
                        }
                    } else if let Some(expected_type) = &param.declared_type
                        && let Some(actual_type) = infer_expr_type(arg, symbols, types)?
                    {
                        let expected_type = resolve_type_ref(expected_type, types)
                            .expect("parameter type should resolve after semantic validation");
                        if !assignment_compatible(&expected_type, &actual_type) {
                            return Err(SemanticError::TypeMismatch {
                                detail: format!(
                                    "parameter '{}' expects {}, got {}",
                                    param.name,
                                    format_type_name(&expected_type),
                                    format_type_name(&actual_type)
                                ),
                            }
                            .into());
                        }
                    }
                }
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
                analyze_statement(stmt, symbols, proc_arity, proc_params, types)?;
            }
            if let Some(else_branch) = else_branch {
                for stmt in else_branch {
                    analyze_statement(stmt, symbols, proc_arity, proc_params, types)?;
                }
            }
            Ok(())
        }
        Statement::While { condition, body } => {
            analyze_expr(condition, symbols)?;
            for stmt in body {
                analyze_statement(stmt, symbols, proc_arity, proc_params, types)?;
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

        struct SuccessCase {
                name: &'static str,
                source: &'static str,
        }

        struct ErrorCase {
                name: &'static str,
                source: &'static str,
                code: &'static str,
                message_contains: &'static [&'static str],
        }

    fn semantic_error(source: &str) -> SemanticError {
        let module = parse_module(source).expect("source should parse for semantic test");
        let err = analyze(&module, None).expect_err("semantic analysis should fail");
        err.downcast::<SemanticError>()
            .expect("error should downcast to SemanticError")
    }

    #[test]
        fn semantic_success_cases() {
                let cases = [
                        SuccessCase {
                                name: "writestring literal",
                                source: r#"
MODULE Main;
BEGIN
    WriteString("Hello, ""Oberon""")
END Main.
"#,
                        },
                        SuccessCase {
                                name: "writeln without args",
                                source: r#"
MODULE Main;
BEGIN
    WriteLn()
END Main.
"#,
                        },
                        SuccessCase {
                                name: "typed integer variable",
                                source: r#"
MODULE Main;
VAR x: INTEGER;
BEGIN
    x := 1
END Main.
"#,
                        },
                        SuccessCase {
                                name: "builtin scalar declarations",
                                source: r#"
MODULE Main;
VAR flag: BOOLEAN;
VAR x: REAL;
VAR y: LONGREAL;
BEGIN
END Main.
"#,
                        },
                        SuccessCase {
                                name: "named type alias declaration",
                                source: r#"
MODULE Main;
TYPE Count = REAL;
VAR x: Count;
BEGIN
    x := 1
END Main.
"#,
                        },
                        SuccessCase {
                                name: "parameter shadows global type alias",
                                source: r#"
MODULE Main;
TYPE Count = INTEGER;
PROCEDURE P(Count: INTEGER);
BEGIN
    WriteInt(Count)
END P;
BEGIN
END Main.
"#,
                        },
                        SuccessCase {
                                name: "typed formal params with VAR mode",
                                source: r#"
MODULE Main;
VAR x;
PROCEDURE Bump(VAR target: INTEGER; amount: INTEGER);
BEGIN
    target := target + amount
END Bump;
BEGIN
    x := 1;
    Bump(x, 2)
END Main.
"#,
                        },
                        SuccessCase {
                                name: "procedure-local var shadows user-defined type name",
                                source: r#"
MODULE Main;
TYPE Count = REAL;
PROCEDURE P;
VAR Count: INTEGER;
BEGIN
    Count := 1;
    WriteInt(Count)
END P;
BEGIN
    P
END Main.
"#,
                        },
                        SuccessCase {
                                name: "readint call expression",
                                source: r#"
MODULE Main;
VAR x;
BEGIN
    x := ReadInt()
END Main.
"#,
                        },
                        SuccessCase {
                                name: "eof call expression in if",
                                source: r#"
MODULE Main;
BEGIN
    IF EOF() THEN
        WriteLn()
    END
END Main.
"#,
                        },
                ];

                for case in cases {
                    let module = parse_module(case.source)
                        .unwrap_or_else(|err| panic!("case '{}' should parse, got: {err}", case.name));
                    analyze(&module, None)
                        .unwrap_or_else(|err| panic!("case '{}' should pass semantic analysis, got: {err}", case.name));
                }
        }

        #[test]
        fn semantic_error_cases() {
                let cases = [
                        ErrorCase {
                                name: "not callable variable",
                                source: r#"
MODULE Main;
VAR x;
BEGIN
    x := 1;
    x()
END Main.
"#,
                                code: "E009",
                                message_contains: &["Symbol 'x' is not callable"],
                        },
                        ErrorCase {
                                name: "procedure arity mismatch",
                                source: r#"
MODULE Main;
PROCEDURE P(a, b);
BEGIN
    WriteInt(a + b)
END P;
BEGIN
    P(1)
END Main.
"#,
                                code: "E006",
                                message_contains: &["Procedure 'P' called with wrong arity", "expected 2, got 1"],
                        },
                        ErrorCase {
                                name: "procedure end name mismatch",
                                source: r#"
MODULE Main;
PROCEDURE P(a);
BEGIN
    WriteInt(a)
END Wrong;
BEGIN
END Main.
"#,
                                code: "E010",
                                message_contains: &["Procedure END name mismatch", "expected 'P', got 'Wrong'"],
                        },
                        ErrorCase {
                                name: "undefined assignment target",
                                source: r#"
MODULE Main;
BEGIN
    y := 1
END Main.
"#,
                                code: "E005",
                                message_contains: &["Undefined symbol usage: 'y'"],
                        },
                        ErrorCase {
                                name: "string outside writestring",
                                source: r#"
MODULE Main;
VAR x;
BEGIN
    x := "Hello"
END Main.
"#,
                                code: "E008",
                                message_contains: &["String literals are only supported"],
                        },
                        ErrorCase {
                                name: "non-string writestring argument",
                                source: r#"
MODULE Main;
BEGIN
    WriteString(1)
END Main.
"#,
                                code: "E007",
                                message_contains: &["Builtin 'WriteString' received an invalid argument", "expected a string literal"],
                        },
                        ErrorCase {
                                name: "writeln with args",
                                source: r#"
MODULE Main;
BEGIN
    WriteLn(1)
END Main.
"#,
                                code: "E006",
                                message_contains: &["Procedure 'WriteLn' called with wrong arity", "expected 0, got 1"],
                        },
                        ErrorCase {
                                name: "unknown type reference",
                                source: r#"
MODULE Main;
VAR x: Missing;
BEGIN
    x := 1
END Main.
"#,
                                code: "E013",
                                message_contains: &["Unknown type reference: 'Missing'"],
                        },
                        ErrorCase {
                                name: "duplicate type declaration",
                                source: r#"
MODULE Main;
TYPE Count = INTEGER;
TYPE Count = INTEGER;
BEGIN
END Main.
"#,
                                code: "E004",
                                message_contains: &["Duplicate symbol declaration: 'Count'"],
                        },
                        ErrorCase {
                                name: "parameter shadows builtin type",
                                source: r#"
MODULE Main;
PROCEDURE P(INTEGER: INTEGER);
BEGIN
    WriteInt(INTEGER)
END P;
BEGIN
END Main.
"#,
                                code: "E004",
                                message_contains: &["Duplicate symbol declaration: 'INTEGER'"],
                        },
                        ErrorCase {
                                name: "parameter self-shadows type alias in declaration",
                                source: r#"
MODULE Main;
TYPE Count = INTEGER;
PROCEDURE P(Count: Count);
BEGIN
    WriteInt(Count)
END P;
BEGIN
END Main.
"#,
                                code: "E004",
                                message_contains: &["Duplicate symbol declaration: 'Count'"],
                        },
                        ErrorCase {
                                name: "procedure-local var shadows builtin type name",
                                source: r#"
MODULE Main;
PROCEDURE P;
VAR INTEGER: INTEGER;
BEGIN
END P;
BEGIN
    P
END Main.
"#,
                                code: "E004",
                                message_contains: &["Duplicate symbol declaration: 'INTEGER'"],
                        },
                        ErrorCase {
                                name: "procedure-local var self-shadows type alias in declaration",
                                source: r#"
MODULE Main;
TYPE Count = INTEGER;
PROCEDURE P;
VAR Count: Count;
BEGIN
END P;
BEGIN
    P
END Main.
"#,
                                code: "E004",
                                message_contains: &["Duplicate symbol declaration: 'Count'"],
                        },
                        ErrorCase {
                                name: "literal passed to VAR parameter",
                                source: r#"
MODULE Main;
PROCEDURE Bump(VAR target: INTEGER; amount: INTEGER);
BEGIN
END Bump;
BEGIN
    Bump(1, 2)
END Main.
"#,
                                code: "E011",
                                message_contains: &["Procedure 'Bump' received an invalid VAR argument", "position 1", "expected a variable designator"],
                        },
                        ErrorCase {
                                name: "assign boolean to integer",
                                source: r#"
MODULE Main;
VAR flag: BOOLEAN;
VAR x: INTEGER;
BEGIN
    x := flag
END Main.
"#,
                                code: "E012",
                                message_contains: &["cannot assign BOOLEAN to INTEGER 'x'"],
                        },
                        ErrorCase {
                                name: "assign real to integer",
                                source: r#"
MODULE Main;
VAR src: REAL;
VAR x: INTEGER;
BEGIN
    x := src
END Main.
"#,
                                code: "E012",
                                message_contains: &["cannot assign REAL to INTEGER 'x'"],
                        },
                        ErrorCase {
                                name: "boolean arithmetic",
                                source: r#"
MODULE Main;
VAR flag: BOOLEAN;
VAR x: INTEGER;
BEGIN
    x := flag + 1
END Main.
"#,
                                code: "E012",
                                message_contains: &["arithmetic expressions require numeric operands"],
                        },
                        ErrorCase {
                                name: "parameter type mismatch",
                                source: r#"
MODULE Main;
VAR x: REAL;
PROCEDURE UseInt(value: INTEGER);
BEGIN
END UseInt;
BEGIN
    UseInt(x)
END Main.
"#,
                                code: "E012",
                                message_contains: &["parameter 'value' expects INTEGER, got REAL"],
                        },
                        ErrorCase {
                                name: "readint statement call",
                                source: r#"
MODULE Main;
BEGIN
    ReadInt()
END Main.
"#,
                                code: "E007",
                                message_contains: &["Builtin 'ReadInt' received an invalid argument", "must be used as a call expression"],
                        },
                        ErrorCase {
                                name: "non-function builtin in call expression",
                                source: r#"
MODULE Main;
VAR x;
BEGIN
    x := WriteInt(1)
END Main.
"#,
                                code: "E007",
                                message_contains: &["Builtin 'WriteInt' received an invalid argument", "currently support only ReadInt() and EOF()"],
                        },
                ];

                for case in cases {
                        let err = semantic_error(case.source);
                        assert_eq!(err.code(), case.code, "unexpected code for case '{}'", case.name);
                        let rendered = err.to_string();
                        for needle in case.message_contains {
                                assert!(
                                        rendered.contains(needle),
                                        "case '{}' expected message to contain '{}', got '{}'",
                                        case.name,
                                        needle,
                                        rendered
                                );
                        }
                }
        }
}
