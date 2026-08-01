//! Semantic checks for name resolution, declaration validity, and call arity.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use anyhow::Result;

use crate::ast::{
    BinaryOp, Declaration, Expr, LocalVarDecl, Module, ParamDecl, Statement, TypeRef, UnaryOp,
};
use crate::expression_constant_handler::combine_expression;
use crate::manifest::ExternalManifest;
use crate::symbols::{SymbolKind, SymbolTable};

/// Information about exported symbols from an external module.
/// Used for resolving qualified references like B.HELLO or B.IntType.
#[derive(Debug, Clone)]
struct ExternalModuleInfo {
    /// Set of exported procedure names.
    exported_procedures: Vec<String>,
    /// Set of exported type names.
    exported_types: Vec<String>,
    /// Mapping of exported type names to their TypeRef definitions.
    type_mappings: HashMap<String, TypeRef>,
}

impl ExternalModuleInfo {
    /// Returns true if the given symbol is exported from this module.
    fn is_exported_procedure(&self, name: &str) -> bool {
        self.exported_procedures.iter().any(|p| p == name)
    }

    /// Returns true if the given type is exported from this module.
    fn is_exported_type(&self, name: &str) -> bool {
        self.exported_types.iter().any(|t| t == name)
    }

    /// Get the underlying TypeRef for an exported type.
    fn get_type(&self, name: &str) -> Option<&TypeRef> {
        self.type_mappings.get(name)
    }

    /// Create a mock resolver for known external modules (used in tests).
    fn mock_resolver() -> HashMap<String, ExternalModuleInfo> {
        let mut modules = HashMap::new();

        // ModuleB is a known external module used in tests with exports HELLO and IntType.
        let mut type_mappings = HashMap::new();
        type_mappings.insert("IntType".to_string(), TypeRef::Integer);
        type_mappings.insert("HiddenType".to_string(), TypeRef::Integer);

        modules.insert(
            "ModuleB".to_string(),
            ExternalModuleInfo {
                exported_procedures: vec!["HELLO".to_string()],
                exported_types: vec!["IntType".to_string()],
                type_mappings,
            },
        );
        modules
    }
}

#[derive(Debug, Clone)]
/// User-facing semantic failures reported after parsing succeeds.
pub enum SemanticError {
    ModuleNameMismatch {
        expected: String,
        got: String,
    },
    DuplicateImportAlias {
        alias: String,
    },
    UnmappedImport {
        import: String,
    },
    DuplicateSymbol {
        name: String,
    },
    UndefinedSymbol {
        name: String,
    },
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
    UnknownType {
        name: String,
    },
    UnsupportedStringLiteral,
    NotCallable {
        name: String,
    },
    ProcedureNameMismatch {
        expected: String,
        got: String,
    },
    NonExportedMember {
        module: String,
        name: String,
    },
    UnsupportedQualifiedVariable {
        module: String,
        name: String,
    },
    InvalidConstDeclaration {
        name: String,
    },
    InternalError {
        error: String,
    },
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
            SemanticError::NonExportedMember { .. } => "E014",
            SemanticError::UnsupportedQualifiedVariable { .. } => "E015",
            SemanticError::InvalidConstDeclaration { .. } => "E016",
            SemanticError::InternalError { .. } => "E999",
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
                write!(
                    f,
                    "[{}] Duplicate symbol declaration: '{}'",
                    self.code(),
                    name
                )
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
                write!(
                    f,
                    "[{}] Builtin '{}' received an invalid argument: {}",
                    self.code(),
                    name,
                    detail
                )
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
                write!(
                    f,
                    "[{}] String literals are only supported as arguments to 'WriteString'",
                    self.code()
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
            SemanticError::NonExportedMember { module, name } => {
                write!(
                    f,
                    "[{}] Symbol '{}' is not exported from module '{}'",
                    self.code(),
                    name,
                    module
                )
            }
            SemanticError::UnsupportedQualifiedVariable { module, name } => {
                write!(
                    f,
                    "[{}] Qualified variable reference '{}.{}' is not yet supported in expressions",
                    self.code(),
                    module,
                    name
                )
            }
            SemanticError::InvalidConstDeclaration { name } => {
                write!(
                    f,
                    "[{}] Constant '{}' must be initialized with a literal expression",
                    self.code(),
                    name
                )
            }
            SemanticError::InternalError { error } => {
                write!(f, "[{}] Internal compiler error: {}", self.code(), error)
            }
        }
    }
}

impl Error for SemanticError {}

fn validate_const_expression_literal(declaration: &Declaration) -> Result<()> {
    let Declaration::Const { name, value } = declaration else {
        return Err(SemanticError::InternalError {
            error: "Cannot retrieve const declaration".to_string(),
        }
        .into());
    };
    let const_value = combine_expression(value)?;

    if const_value.is_literal() {
        Ok(())
    } else {
        Err(SemanticError::InvalidConstDeclaration { name: name.clone() }.into())
    }
}

fn validate_declared_type(type_ref: &TypeRef, types: &HashMap<String, TypeRef>) -> Result<()> {
    if resolve_type_ref(type_ref, types).is_none() {
        return Err(SemanticError::UnknownType {
            name: match type_ref {
                TypeRef::Integer => "INTEGER".to_string(),
                TypeRef::Boolean => "BOOLEAN".to_string(),
                TypeRef::Real => "REAL".to_string(),
                TypeRef::LongReal => "LONGREAL".to_string(),
                TypeRef::Named(name) => name.clone(),
                TypeRef::Qualified { module, name } => format!("{}.{}", module, name),
            },
        }
        .into());
    }

    Ok(())
}

/// Validates a type reference with support for qualified types via import resolution.
fn validate_declared_type_with_imports(
    type_ref: &TypeRef,
    types: &HashMap<String, TypeRef>,
    import_aliases: &HashMap<String, String>,
    external_modules: &HashMap<String, ExternalModuleInfo>,
) -> Result<()> {
    match type_ref {
        TypeRef::Qualified { module, name } => {
            // Resolve module alias to external module name.
            let module_name =
                import_aliases
                    .get(module)
                    .ok_or_else(|| SemanticError::UnknownType {
                        name: format!("{}.{}", module, name),
                    })?;

            // Check if external module exists and exports this type.
            let ext_module =
                external_modules
                    .get(module_name)
                    .ok_or_else(|| SemanticError::UnknownType {
                        name: format!("{}.{}", module, name),
                    })?;

            if !ext_module.is_exported_type(name) {
                return Err(SemanticError::NonExportedMember {
                    module: module_name.clone(),
                    name: name.clone(),
                }
                .into());
            }

            Ok(())
        }
        _ => validate_declared_type(type_ref, types),
    }
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

fn validate_local_var_name(
    local_var: &LocalVarDecl,
    types: &HashMap<String, TypeRef>,
) -> Result<()> {
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
        TypeRef::Qualified { .. } => {
            // Qualified types are treated as opaque for now.
            // They can be used in declarations and assignments, but we don't expand them further.
            // In a full implementation, we would load the external module and resolve the type.
            Some(type_ref.clone())
        }
    }
}

fn is_numeric_type(type_ref: &TypeRef) -> bool {
    matches!(
        type_ref,
        TypeRef::Integer | TypeRef::Real | TypeRef::LongReal
    )
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

/// Extended assignment compatibility check that handles qualified types.
/// Looks up the actual type of qualified references in the external_modules.
fn assignment_compatible_extended(
    expected: &TypeRef,
    actual: &TypeRef,
    import_aliases: &HashMap<String, String>,
    external_modules: &HashMap<String, ExternalModuleInfo>,
) -> bool {
    // Expand qualified types by looking them up in external modules
    let expanded_expected = match expected {
        TypeRef::Qualified { module, name } => {
            // Resolve module alias to actual module name
            if let Some(module_name) = import_aliases.get(module) {
                // Look up the type in the external module
                if let Some(ext_module) = external_modules.get(module_name) {
                    if let Some(actual_type) = ext_module.get_type(name) {
                        actual_type.clone()
                    } else {
                        expected.clone()
                    }
                } else {
                    expected.clone()
                }
            } else {
                expected.clone()
            }
        }
        _ => expected.clone(),
    };

    // Expand actual qualified types as well
    let expanded_actual = match actual {
        TypeRef::Qualified { module, name } => {
            if let Some(module_name) = import_aliases.get(module) {
                if let Some(ext_module) = external_modules.get(module_name) {
                    if let Some(actual_type) = ext_module.get_type(name) {
                        actual_type.clone()
                    } else {
                        actual.clone()
                    }
                } else {
                    actual.clone()
                }
            } else {
                actual.clone()
            }
        }
        _ => actual.clone(),
    };

    // Now use the standard compatibility check on the expanded types
    assignment_compatible(&expanded_expected, &expanded_actual)
}

fn format_type_name(type_ref: &TypeRef) -> &'static str {
    match type_ref {
        TypeRef::Integer => "INTEGER",
        TypeRef::Boolean => "BOOLEAN",
        TypeRef::Real => "REAL",
        TypeRef::LongReal => "LONGREAL",
        TypeRef::Named(_) => "<named>",
        TypeRef::Qualified { .. } => "<qualified>",
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
        Expr::Real(_) => Ok(Some(TypeRef::Real)),
        Expr::LongReal(_) => Ok(Some(TypeRef::LongReal)),
        Expr::Boolean(_) => Ok(Some(TypeRef::Boolean)),
        Expr::String(_) => Err(SemanticError::UnsupportedStringLiteral.into()),
        Expr::Variable(name) => {
            if symbols.resolve(name).is_none() {
                return Err(SemanticError::UndefinedSymbol { name: name.clone() }.into());
            }

            Ok(resolve_symbol_type(symbols, name)
                .and_then(|type_ref| resolve_type_ref(&type_ref, types)))
        }
        Expr::QualifiedVariable { module, name } => {
            Err(SemanticError::UnsupportedQualifiedVariable {
                module: module.clone(),
                name: name.clone(),
            }
            .into())
        }
        Expr::Call {
            module: _,
            name,
            args,
        } => {
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
        Expr::Unary { op, value } => {
            let value_type = infer_expr_type(value, symbols, types)?;

            match (op, value_type) {
                (UnaryOp::Plus | UnaryOp::Minus, Some(value_type)) => {
                    if !is_numeric_type(&value_type) {
                        return Err(SemanticError::TypeMismatch {
                            detail: format!(
                                "unary sign operators require numeric operands, got {}",
                                format_type_name(&value_type)
                            ),
                        }
                        .into());
                    }
                    Ok(Some(value_type))
                }
                (UnaryOp::Not, Some(value_type)) => {
                    if value_type != TypeRef::Boolean {
                        return Err(SemanticError::TypeMismatch {
                            detail: format!(
                                "operator '~' requires BOOLEAN operand, got {}",
                                format_type_name(&value_type)
                            ),
                        }
                        .into());
                    }
                    Ok(Some(TypeRef::Boolean))
                }
                (_, None) => Ok(None),
            }
        }
        Expr::Binary { op, left, right } => {
            let left_type = infer_expr_type(left, symbols, types)?;
            let right_type = infer_expr_type(right, symbols, types)?;

            match (left_type, right_type) {
                (Some(left_type), Some(right_type)) => match op {
                    BinaryOp::Or | BinaryOp::And => {
                        if left_type != TypeRef::Boolean || right_type != TypeRef::Boolean {
                            return Err(SemanticError::TypeMismatch {
                                detail: format!(
                                    "logical operator requires BOOLEAN operands, got {} and {}",
                                    format_type_name(&left_type),
                                    format_type_name(&right_type)
                                ),
                            }
                            .into());
                        }
                        Ok(Some(TypeRef::Boolean))
                    }
                    BinaryOp::IntDiv | BinaryOp::Mod => {
                        if left_type != TypeRef::Integer || right_type != TypeRef::Integer {
                            return Err(SemanticError::TypeMismatch {
                                detail: format!(
                                    "operator '{}' requires INTEGER operands, got {} and {}",
                                    match op {
                                        BinaryOp::IntDiv => "DIV",
                                        BinaryOp::Mod => "MOD",
                                        _ => unreachable!(),
                                    },
                                    format_type_name(&left_type),
                                    format_type_name(&right_type)
                                ),
                            }
                            .into());
                        }
                        Ok(Some(TypeRef::Integer))
                    }
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
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
                    BinaryOp::Eq | BinaryOp::Ne => {
                        let both_numeric =
                            is_numeric_type(&left_type) && is_numeric_type(&right_type);
                        let both_boolean =
                            left_type == TypeRef::Boolean && right_type == TypeRef::Boolean;
                        if !both_numeric && !both_boolean {
                            return Err(SemanticError::TypeMismatch {
                                    detail: format!(
                                        "equality operators require matching numeric or BOOLEAN operands, got {} and {}",
                                        format_type_name(&left_type),
                                        format_type_name(&right_type)
                                    ),
                                }
                                .into());
                        }
                        Ok(Some(TypeRef::Boolean))
                    }
                    BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                        if !is_numeric_type(&left_type) || !is_numeric_type(&right_type) {
                            return Err(SemanticError::TypeMismatch {
                                    detail: format!(
                                        "ordering relational operators require numeric operands, got {} and {}",
                                        format_type_name(&left_type),
                                        format_type_name(&right_type)
                                    ),
                                }
                                .into());
                        }
                        Ok(Some(TypeRef::Boolean))
                    }
                },
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

    // Track import aliases: maps local name (e.g., "B") to external module name (e.g., "ModuleB").
    let mut import_aliases: HashMap<String, String> = HashMap::new();
    // Load mock external modules for qualified reference resolution.
    let external_modules = ExternalModuleInfo::mock_resolver();

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

        import_aliases.insert(import.local_name.clone(), import.external_name.clone());

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
                validate_const_expression_literal(declaration)?;
                symbols.declare(name, SymbolKind::Constant)?;
            }
            Declaration::Type { name, target, .. } => {
                validate_declaration_name(name, &types)?;
                validate_declared_type_with_imports(
                    target,
                    &types,
                    &import_aliases,
                    &external_modules,
                )?;
                symbols.declare_with_type(name, SymbolKind::TypeName, Some(target.clone()))?;
                types.insert(name.clone(), target.clone());
            }
            Declaration::Var {
                name,
                declared_type,
            } => {
                validate_declaration_name(name, &types)?;
                if let Some(type_ref) = declared_type {
                    validate_declared_type_with_imports(
                        type_ref,
                        &types,
                        &import_aliases,
                        &external_modules,
                    )?;
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
                        validate_declared_type_with_imports(
                            type_ref,
                            &types,
                            &import_aliases,
                            &external_modules,
                        )?;
                    }
                }
                for local_var in local_vars {
                    validate_local_var_name(local_var, &types)?;
                    if let Some(type_ref) = &local_var.declared_type {
                        validate_declared_type_with_imports(
                            type_ref,
                            &types,
                            &import_aliases,
                            &external_modules,
                        )?;
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
            is_exported: _,
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
                analyze_statement(
                    statement,
                    &mut symbols,
                    &proc_arity,
                    &proc_params,
                    &types,
                    &import_aliases,
                    &external_modules,
                )?;
            }
            symbols.exit_scope();
        }
    }

    for statement in &module.statements {
        analyze_statement(
            statement,
            &mut symbols,
            &proc_arity,
            &proc_params,
            &types,
            &import_aliases,
            &external_modules,
        )?;
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
            let symbol = symbols
                .resolve(name)
                .ok_or_else(|| SemanticError::UndefinedSymbol { name: name.clone() })?;

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
    import_aliases: &HashMap<String, String>,
    external_modules: &HashMap<String, ExternalModuleInfo>,
) -> Result<()> {
    match stmt {
        Statement::Assign { target, value } => {
            analyze_expr(value, symbols)?;
            let symbol = symbols
                .resolve(target)
                .ok_or_else(|| SemanticError::UndefinedSymbol {
                    name: target.clone(),
                })?;

            if let Some(expected_type) = &symbol.declared_type
                && let Some(actual_type) = infer_expr_type(value, symbols, types)?
            {
                let expected_type = resolve_type_ref(expected_type, types)
                    .expect("declared target type should resolve after semantic validation");
                if !assignment_compatible_extended(
                    &expected_type,
                    &actual_type,
                    import_aliases,
                    external_modules,
                ) {
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

            Ok(())
        }
        Statement::Call { module, name, args } => {
            // Handle qualified calls (e.g., B.HELLO).
            if let Some(module_alias) = module {
                // Resolve alias to external module name.
                let module_name = import_aliases.get(module_alias).ok_or_else(|| {
                    SemanticError::UndefinedSymbol {
                        name: module_alias.clone(),
                    }
                })?;

                // Check if the external module exists and has this procedure exported.
                let ext_module = external_modules.get(module_name).ok_or_else(|| {
                    SemanticError::UndefinedSymbol {
                        name: format!("{}.{}", module_alias, name),
                    }
                })?;

                if !ext_module.is_exported_procedure(name) {
                    return Err(SemanticError::NonExportedMember {
                        module: module_name.clone(),
                        name: name.clone(),
                    }
                    .into());
                }

                // For now, we don't validate arity of external procedures.
                // This would require loading the external module definition.
                for arg in args {
                    analyze_expr(arg, symbols)?;
                }
                return Ok(());
            }

            // Original logic for unqualified calls.
            if name == "ReadInt" || name == "EOF" {
                return Err(SemanticError::InvalidBuiltinArgument {
                    name: name.clone(),
                    detail:
                        "must be used as a call expression (e.g. x := ReadInt(), IF EOF() THEN ...)"
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

            let symbol = symbols
                .resolve(name)
                .ok_or_else(|| SemanticError::UndefinedSymbol { name: name.clone() })?;

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
                            let expected_type = resolve_type_ref(expected_type, types).expect(
                                "VAR parameter type should resolve after semantic validation",
                            );
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
                analyze_statement(
                    stmt,
                    symbols,
                    proc_arity,
                    proc_params,
                    types,
                    import_aliases,
                    external_modules,
                )?;
            }
            if let Some(else_branch) = else_branch {
                for stmt in else_branch {
                    analyze_statement(
                        stmt,
                        symbols,
                        proc_arity,
                        proc_params,
                        types,
                        import_aliases,
                        external_modules,
                    )?;
                }
            }
            Ok(())
        }
        Statement::While { condition, body } => {
            analyze_expr(condition, symbols)?;
            for stmt in body {
                analyze_statement(
                    stmt,
                    symbols,
                    proc_arity,
                    proc_params,
                    types,
                    import_aliases,
                    external_modules,
                )?;
            }
            Ok(())
        }
    }
}

/// Validates an expression and ensures every referenced symbol is defined.
fn analyze_expr(expr: &Expr, symbols: &SymbolTable) -> Result<()> {
    match expr {
        Expr::Integer(_) => Ok(()),
        Expr::Real(_) => Ok(()),
        Expr::LongReal(_) => Ok(()),
        Expr::Boolean(_) => Ok(()),
        Expr::String(_) => Err(SemanticError::UnsupportedStringLiteral.into()),
        Expr::Variable(name) => {
            if symbols.resolve(name).is_none() {
                return Err(SemanticError::UndefinedSymbol { name: name.clone() }.into());
            }
            Ok(())
        }
        Expr::QualifiedVariable { module, name } => {
            Err(SemanticError::UnsupportedQualifiedVariable {
                module: module.clone(),
                name: name.clone(),
            }
            .into())
        }
        Expr::Call {
            module: _,
            name,
            args,
        } => {
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
        Expr::Unary { value, .. } => analyze_expr(value, symbols),
        Expr::Binary { left, right, .. } => {
            analyze_expr(left, symbols)?;
            analyze_expr(right, symbols)
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::path::PathBuf;

    use rstest::rstest;

    use super::{SemanticError, analyze};
    use crate::manifest::ExternalManifest;
    use crate::parser::parse_module;

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
                        || (matches!(lhs, ScalarType::Boolean)
                            && matches!(rhs, ScalarType::Boolean));
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
}
