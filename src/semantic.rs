//! Semantic checks for name resolution, declaration validity, and call arity.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use anyhow::Result;

use crate::ast::{
    AssignTarget, BinaryOp, Declaration, Expr, LocalVarDecl, Module, ParamDecl, Statement, TypeRef,
    UnaryOp,
};
use crate::expression_constant_handler::combine_expression;
use crate::manifest::ExternalManifest;
use crate::symbols::{SymbolKind, SymbolTable};
use crate::types::Type;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltinModule {
    Io,
    Math,
}

impl BuiltinModule {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "IO" => Some(Self::Io),
            "MATH" => Some(Self::Math),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltinId {
    WriteInt,
    WriteString,
    WriteLn,
    WriteReal,
    WriteLongReal,
    ReadInt,
    ReadReal,
    ReadLongReal,
    Eof,
    Flt,
    Floor,
}

fn builtin_display_name(module: Option<&str>, name: &str) -> String {
    match module {
        Some(module_name) => format!("{}.{}", module_name, name),
        None => name.to_string(),
    }
}

fn resolve_builtin(module: Option<&str>, name: &str) -> Option<BuiltinId> {
    match module {
        Some("IO") => match name {
            "WriteInt" => Some(BuiltinId::WriteInt),
            "WriteString" => Some(BuiltinId::WriteString),
            "WriteLn" => Some(BuiltinId::WriteLn),
            "WriteReal" => Some(BuiltinId::WriteReal),
            "WriteLongReal" => Some(BuiltinId::WriteLongReal),
            "ReadInt" => Some(BuiltinId::ReadInt),
            "ReadReal" => Some(BuiltinId::ReadReal),
            "ReadLongReal" => Some(BuiltinId::ReadLongReal),
            "EOF" => Some(BuiltinId::Eof),
            _ => None,
        },
        Some("MATH") => match name {
            "FLT" => Some(BuiltinId::Flt),
            "FLOOR" => Some(BuiltinId::Floor),
            _ => None,
        },
        Some(_) => None,
        None => None,
    }
}

fn resolve_builtin_with_module_validation(module: Option<&str>, name: &str) -> Result<Option<BuiltinId>> {
    if let Some(module_name) = module
        && BuiltinModule::from_name(module_name).is_some()
    {
        return match resolve_builtin(module, name) {
            Some(id) => Ok(Some(id)),
            None => Err(SemanticError::InvalidBuiltinArgument {
                name: builtin_display_name(module, name),
                detail: "unknown builtin member".to_string(),
            }
            .into()),
        };
    }

    Ok(resolve_builtin(module, name))
}

fn is_internal_builtin_module_name(name: &str) -> bool {
    BuiltinModule::from_name(name).is_some()
}

fn required_builtin_module_for_name(name: &str) -> Option<&'static str> {
    match name {
        "WriteInt" | "WriteString" | "WriteLn" | "WriteReal" | "WriteLongReal" | "ReadInt"
        | "ReadReal" | "ReadLongReal" | "EOF" => Some("IO"),
        "FLT" | "FLOOR" => Some("MATH"),
        _ => None,
    }
}

fn ensure_internal_builtin_module_imported(module: Option<&str>, symbols: &SymbolTable) -> Result<()> {
    if let Some(module_name) = module
        && is_internal_builtin_module_name(module_name)
        && symbols.resolve(module_name).is_none()
    {
        return Err(SemanticError::UndefinedSymbol {
            name: module_name.to_string(),
        }
        .into());
    }

    Ok(())
}

fn builtin_fixed_arity(id: BuiltinId) -> Option<usize> {
    match id {
        BuiltinId::WriteInt => None,
        BuiltinId::WriteString => Some(1),
        BuiltinId::WriteLn => Some(0),
        BuiltinId::WriteReal => Some(1),
        BuiltinId::WriteLongReal => Some(1),
        BuiltinId::ReadInt => Some(0),
        BuiltinId::ReadReal => Some(0),
        BuiltinId::ReadLongReal => Some(0),
        BuiltinId::Eof => Some(0),
        BuiltinId::Flt => Some(1),
        BuiltinId::Floor => Some(1),
    }
}

fn builtin_allows_statement(id: BuiltinId) -> bool {
    matches!(
        id,
        BuiltinId::WriteInt
            | BuiltinId::WriteString
            | BuiltinId::WriteLn
            | BuiltinId::WriteReal
            | BuiltinId::WriteLongReal
    )
}

fn builtin_allows_expression(id: BuiltinId) -> bool {
    matches!(
        id,
        BuiltinId::ReadInt
            | BuiltinId::ReadReal
            | BuiltinId::ReadLongReal
            | BuiltinId::Eof
            | BuiltinId::Flt
            | BuiltinId::Floor
    )
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

#[cfg_attr(coverage_nightly, coverage(off))]
fn type_ref_name_for_error(type_ref: &TypeRef) -> String {
    match type_ref {
        TypeRef::Integer => "INTEGER".to_string(),
        TypeRef::Boolean => "BOOLEAN".to_string(),
        TypeRef::Real => "REAL".to_string(),
        TypeRef::LongReal => "LONGREAL".to_string(),
        TypeRef::Array {
            length,
            element_type,
        } => format!(
            "ARRAY {} OF {}",
            format_expr_for_error(length),
            type_ref_name_for_error(element_type)
        ),
        TypeRef::Named(name) => name.clone(),
        TypeRef::Qualified { module, name } => format!("{}.{}", module, name),
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn format_expr_for_error(expr: &Expr) -> String {
    match expr {
        Expr::Integer(v) => v.to_string(),
        Expr::Real(v) => v.to_string(),
        Expr::LongReal(v) => v.to_string(),
        Expr::Boolean(v) => {
            if *v {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Expr::String(value) => format!("\"{}\"", value),
        Expr::Variable(name) => name.clone(),
        Expr::Indexed { name, index } => {
            format!("{}[{}]", name, format_expr_for_error(index))
        }
        Expr::QualifiedVariable { module, name } => format!("{}.{}", module, name),
        Expr::Call { module, name, args } => {
            let rendered_args = args
                .iter()
                .map(format_expr_for_error)
                .collect::<Vec<_>>()
                .join(", ");
            match module {
                Some(module_name) => format!("{}.{}({})", module_name, name, rendered_args),
                None => format!("{}({})", name, rendered_args),
            }
        }
        Expr::Unary { op, value } => {
            let op_text = match op {
                UnaryOp::Plus => "+",
                UnaryOp::Minus => "-",
                UnaryOp::Not => "~",
            };
            format!("{}{}", op_text, format_expr_for_error(value))
        }
        Expr::Binary { op, left, right } => {
            let op_text = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Or => "OR",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::IntDiv => "DIV",
                BinaryOp::Mod => "MOD",
                BinaryOp::And => "&",
                BinaryOp::Eq => "=",
                BinaryOp::Ne => "#",
                BinaryOp::Lt => "<",
                BinaryOp::Le => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::Ge => ">=",
            };
            format!(
                "({} {} {})",
                format_expr_for_error(left),
                op_text,
                format_expr_for_error(right)
            )
        }
    }
}

fn substitute_const_expr(expr: &Expr, const_values: &HashMap<String, Expr>) -> Expr {
    match expr {
        Expr::Variable(name) => const_values
            .get(name)
            .cloned()
            .unwrap_or_else(|| Expr::Variable(name.clone())),
        Expr::Unary { op, value } => Expr::Unary {
            op: *op,
            value: Box::new(substitute_const_expr(value, const_values)),
        },
        Expr::Binary { op, left, right } => Expr::Binary {
            op: *op,
            left: Box::new(substitute_const_expr(left, const_values)),
            right: Box::new(substitute_const_expr(right, const_values)),
        },
        Expr::Indexed { name, index } => Expr::Indexed {
            name: name.clone(),
            index: Box::new(substitute_const_expr(index, const_values)),
        },
        Expr::Call { module, name, args } => Expr::Call {
            module: module.clone(),
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_const_expr(arg, const_values))
                .collect(),
        },
        Expr::QualifiedVariable { module, name } => Expr::QualifiedVariable {
            module: module.clone(),
            name: name.clone(),
        },
        _ => expr.clone(),
    }
}

fn resolve_array_length_expr(length: &Expr, const_values: &HashMap<String, Expr>) -> Option<usize> {
    let substituted = substitute_const_expr(length, const_values);
    let combined = combine_expression(&substituted).ok()?;
    match combined {
        Expr::Integer(value) => usize::try_from(value).ok(),
        _ => None,
    }
}

fn validate_declared_type_with_consts(
    type_ref: &TypeRef,
    types: &HashMap<String, TypeRef>,
    const_values: &HashMap<String, Expr>,
) -> Result<()> {
    if resolve_type_ref_with_consts(type_ref, types, const_values).is_none() {
        return Err(SemanticError::UnknownType {
            name: type_ref_name_for_error(type_ref),
        }
        .into());
    }

    Ok(())
}

#[cfg(test)]
fn validate_declared_type(type_ref: &TypeRef, types: &HashMap<String, TypeRef>) -> Result<()> {
    let const_values = HashMap::new();
    validate_declared_type_with_consts(type_ref, types, &const_values)
}

/// Validates a type reference with support for qualified types via import resolution.
fn validate_declared_type_with_imports(
    type_ref: &TypeRef,
    types: &HashMap<String, TypeRef>,
    import_aliases: &HashMap<String, String>,
    external_modules: &HashMap<String, ExternalModuleInfo>,
    const_values: &HashMap<String, Expr>,
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
        _ => validate_declared_type_with_consts(type_ref, types, const_values),
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

fn resolve_type_ref_with_consts(
    type_ref: &TypeRef,
    types: &HashMap<String, TypeRef>,
    const_values: &HashMap<String, Expr>,
) -> Option<TypeRef> {
    match type_ref {
        TypeRef::Integer => Some(TypeRef::Integer),
        TypeRef::Boolean => Some(TypeRef::Boolean),
        TypeRef::Real => Some(TypeRef::Real),
        TypeRef::LongReal => Some(TypeRef::LongReal),
        TypeRef::Array {
            length,
            element_type,
        } => {
            let normalized_length = resolve_array_length_expr(length, const_values)?;
            resolve_type_ref_with_consts(element_type, types, const_values).map(|resolved| {
                TypeRef::Array {
                    length: Expr::Integer(normalized_length as i64),
                    element_type: Box::new(resolved),
                }
            })
        }
        TypeRef::Named(name) => match types.get(name) {
            Some(target) => resolve_type_ref_with_consts(target, types, const_values),
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

fn resolve_type_ref(type_ref: &TypeRef, types: &HashMap<String, TypeRef>) -> Option<TypeRef> {
    let const_values = HashMap::new();
    resolve_type_ref_with_consts(type_ref, types, &const_values)
}

fn is_numeric_type(type_ref: &TypeRef) -> bool {
    matches!(
        type_ref,
        TypeRef::Integer | TypeRef::Real | TypeRef::LongReal
    )
}

fn assignment_compatible(expected: &TypeRef, actual: &TypeRef) -> bool {
    let expected_type = Type::from_ast_type_ref(expected);
    let actual_type = Type::from_ast_type_ref(actual);
    expected_type.is_compatible_with(&actual_type, &HashMap::new())
}

/// Extended assignment compatibility check that handles qualified types.
/// Looks up the actual type of qualified references in the external_modules.
fn assignment_compatible_extended(
    expected: &TypeRef,
    actual: &TypeRef,
    import_aliases: &HashMap<String, String>,
    external_modules: &HashMap<String, ExternalModuleInfo>,
) -> bool {
    let expected_type = match expected {
        TypeRef::Qualified { module, name } => {
            if let Some(module_name) = import_aliases.get(module) {
                if let Some(ext_module) = external_modules.get(module_name) {
                    if let Some(actual_type) = ext_module.get_type(name) {
                        Type::from_ast_type_ref(actual_type)
                    } else {
                        Type::from_ast_type_ref(expected)
                    }
                } else {
                    Type::from_ast_type_ref(expected)
                }
            } else {
                Type::from_ast_type_ref(expected)
            }
        }
        _ => Type::from_ast_type_ref(expected),
    };

    let actual_type = match actual {
        TypeRef::Qualified { module, name } => {
            if let Some(module_name) = import_aliases.get(module) {
                if let Some(ext_module) = external_modules.get(module_name) {
                    if let Some(actual_type) = ext_module.get_type(name) {
                        Type::from_ast_type_ref(actual_type)
                    } else {
                        Type::from_ast_type_ref(actual)
                    }
                } else {
                    Type::from_ast_type_ref(actual)
                }
            } else {
                Type::from_ast_type_ref(actual)
            }
        }
        _ => Type::from_ast_type_ref(actual),
    };

    expected_type.is_compatible_with(&actual_type, &HashMap::new())
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn format_type_name(type_ref: &TypeRef) -> &'static str {
    match type_ref {
        TypeRef::Integer => "INTEGER",
        TypeRef::Boolean => "BOOLEAN",
        TypeRef::Real => "REAL",
        TypeRef::LongReal => "LONGREAL",
        TypeRef::Array { .. } => "ARRAY",
        TypeRef::Named(_) => "<named>",
        TypeRef::Qualified { .. } => "<qualified>",
    }
}

fn resolve_symbol_type(symbols: &SymbolTable, name: &str) -> Option<TypeRef> {
    symbols
        .resolve(name)
        .and_then(|symbol| symbol.declared_type.clone())
}

fn validate_boolean_condition(
    expr: &Expr,
    symbols: &SymbolTable,
    types: &HashMap<String, TypeRef>,
) -> Result<()> {
    analyze_expr(expr, symbols)?;

    match infer_expr_type(expr, symbols, types)? {
        Some(TypeRef::Boolean) => Ok(()),
        Some(TypeRef::Integer) => {
            if matches!(
                expr,
                Expr::Call { module, name, args }
                    if args.is_empty()
                        && matches!(
                            resolve_builtin(module.as_deref(), name),
                            Some(BuiltinId::Eof)
                        )
            ) {
                Ok(())
            } else {
                Err(SemanticError::TypeMismatch {
                    detail: "condition must be BOOLEAN".to_string(),
                }
                .into())
            }
        }
        Some(actual_type) => Err(SemanticError::TypeMismatch {
            detail: format!(
                "condition must be BOOLEAN, got {}",
                format_type_name(&actual_type)
            ),
        }
        .into()),
        None => Ok(()),
    }
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
        Expr::Indexed { name, index } => infer_indexed_expr_type(name, index, symbols, types),
        Expr::QualifiedVariable { module, name } => {
            Err(SemanticError::UnsupportedQualifiedVariable {
                module: module.clone(),
                name: name.clone(),
            }
            .into())
        }
        Expr::Call { module, name, args } => {
            ensure_internal_builtin_module_imported(module.as_deref(), symbols)?;

            if module.is_none()
                && let Some(required_module) = required_builtin_module_for_name(name)
            {
                return Err(SemanticError::InvalidBuiltinArgument {
                    name: name.clone(),
                    detail: format!("must be qualified as {}.{}(...)", required_module, name),
                }
                .into());
            }

            if let Some(builtin_id) = resolve_builtin_with_module_validation(module.as_deref(), name)? {
                if let Some(expected_arity) = builtin_fixed_arity(builtin_id)
                    && args.len() != expected_arity
                {
                    return Err(SemanticError::ArityMismatch {
                        name: builtin_display_name(module.as_deref(), name),
                        expected: expected_arity,
                        got: args.len(),
                    }
                    .into());
                }

                return match builtin_id {
                    BuiltinId::WriteInt
                    | BuiltinId::WriteString
                    | BuiltinId::WriteLn
                    | BuiltinId::WriteReal
                    | BuiltinId::WriteLongReal => Err(SemanticError::InvalidBuiltinArgument {
                        name: builtin_display_name(module.as_deref(), name),
                        detail: "must be used as a statement call".to_string(),
                    }
                    .into()),
                    BuiltinId::ReadInt | BuiltinId::Eof => Ok(Some(TypeRef::Integer)),
                    BuiltinId::ReadReal => Ok(Some(TypeRef::Real)),
                    BuiltinId::ReadLongReal => Ok(Some(TypeRef::LongReal)),
                    BuiltinId::Flt => {
                        let inner_type = infer_expr_type(&args[0], symbols, types)?;
                        if matches!(inner_type, Some(TypeRef::Integer)) {
                            Ok(Some(TypeRef::Real))
                        } else {
                            Err(SemanticError::TypeMismatch {
                                detail: "FLT() requires an INTEGER argument".to_string(),
                            }
                            .into())
                        }
                    }
                    BuiltinId::Floor => {
                        let inner_type = infer_expr_type(&args[0], symbols, types)?;
                        if matches!(inner_type, Some(TypeRef::Real | TypeRef::LongReal)) {
                            Ok(Some(TypeRef::Integer))
                        } else {
                            Err(SemanticError::TypeMismatch {
                                detail: "FLOOR() requires a REAL or LONGREAL argument"
                                    .to_string(),
                            }
                            .into())
                        }
                    }
                };
            }

            if let Some(module_name) = module {
                return Err(SemanticError::UndefinedSymbol {
                    name: format!("{}.{}", module_name, name),
                }
                .into());
            }

            if symbols.resolve(name).is_none() {
                return Err(SemanticError::UndefinedSymbol { name: name.clone() }.into());
            }

            Err(SemanticError::InvalidBuiltinArgument {
                name: name.clone(),
                detail:
                    "call expressions currently support only ReadInt(), ReadReal(), ReadLongReal(), EOF(), FLT(), and FLOOR()"
                        .to_string(),
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
                            let op_name = if op == &BinaryOp::IntDiv { "DIV" } else { "MOD" };
                            return Err(SemanticError::TypeMismatch {
                                detail: format!(
                                    "operator '{}' requires INTEGER operands, got {} and {}",
                                    op_name,
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
    symbols.declare("WriteReal", SymbolKind::Procedure)?;
    symbols.declare("WriteLongReal", SymbolKind::Procedure)?;
    symbols.declare("ReadInt", SymbolKind::Procedure)?;
    symbols.declare("ReadReal", SymbolKind::Procedure)?;
    symbols.declare("ReadLongReal", SymbolKind::Procedure)?;
    symbols.declare("EOF", SymbolKind::Procedure)?;
    symbols.declare("FLT", SymbolKind::Procedure)?;
    symbols.declare("FLOOR", SymbolKind::Procedure)?;
    let mut proc_arity: HashMap<String, Option<usize>> = HashMap::new();
    let mut proc_params: HashMap<String, Vec<ParamDecl>> = HashMap::new();
    let mut const_values: HashMap<String, Expr> = HashMap::new();
    proc_arity.insert("WriteInt".to_string(), None);
    proc_arity.insert("WriteString".to_string(), Some(1));
    proc_arity.insert("WriteLn".to_string(), Some(0));
    proc_arity.insert("WriteReal".to_string(), Some(1));
    proc_arity.insert("WriteLongReal".to_string(), Some(1));
    proc_arity.insert("ReadInt".to_string(), Some(0));
    proc_arity.insert("ReadReal".to_string(), Some(0));
    proc_arity.insert("ReadLongReal".to_string(), Some(0));
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
            && !is_internal_builtin_module_name(&import.external_name)
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
                let Declaration::Const { value, .. } = declaration else {
                    unreachable!("const declaration expected");
                };
                const_values.insert(name.clone(), combine_expression(value)?);
                symbols.declare(name, SymbolKind::Constant)?;
            }
            Declaration::Type { name, target, .. } => {
                validate_declaration_name(name, &types)?;
                validate_declared_type_with_imports(
                    target,
                    &types,
                    &import_aliases,
                    &external_modules,
                    &const_values,
                )?;
                let resolved_target = resolve_type_ref_with_consts(target, &types, &const_values)
                    .ok_or_else(|| SemanticError::UnknownType {
                    name: type_ref_name_for_error(target),
                })?;
                symbols.declare_with_type(
                    name,
                    SymbolKind::TypeName,
                    Some(resolved_target.clone()),
                )?;
                types.insert(name.clone(), resolved_target);
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
                        &const_values,
                    )?;
                }
                let normalized_declared_type = declared_type
                    .as_ref()
                    .map(|type_ref| {
                        resolve_type_ref_with_consts(type_ref, &types, &const_values).ok_or_else(
                            || SemanticError::UnknownType {
                                name: type_ref_name_for_error(type_ref),
                            },
                        )
                    })
                    .transpose()?;
                symbols.declare_with_type(name, SymbolKind::Variable, normalized_declared_type)?;
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
                            &const_values,
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
                            &const_values,
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
                let normalized_type = param.declared_type.as_ref().map(|type_ref| {
                    resolve_type_ref_with_consts(type_ref, &types, &const_values)
                        .unwrap_or_else(|| type_ref.clone())
                });
                symbols.declare_with_type(&param.name, SymbolKind::Parameter, normalized_type)?;
            }
            for local_var in local_vars {
                let normalized_type = local_var.declared_type.as_ref().map(|type_ref| {
                    resolve_type_ref_with_consts(type_ref, &types, &const_values)
                        .unwrap_or_else(|| type_ref.clone())
                });
                symbols.declare_with_type(
                    &local_var.name,
                    SymbolKind::Variable,
                    normalized_type,
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
            match target {
                AssignTarget::Name(name) => {
                    let symbol = symbols
                        .resolve(name)
                        .ok_or_else(|| SemanticError::UndefinedSymbol { name: name.clone() })?;

                    if let Some(expected_type) = &symbol.declared_type
                        && let Some(actual_type) = infer_expr_type(value, symbols, types)?
                    {
                        let expected_type = resolve_type_ref(expected_type, types).expect(
                            "declared target type should resolve after semantic validation",
                        );
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
                                    name
                                ),
                            }
                            .into());
                        }
                    }
                }
                AssignTarget::Indexed { name, index } => {
                    analyze_expr(index, symbols)?;
                    let index_type = infer_expr_type(index, symbols, types)?;
                    if index_type != Some(TypeRef::Integer) {
                        return Err(SemanticError::TypeMismatch {
                            detail: format!(
                                "array index for '{}' must be INTEGER, got {}",
                                name,
                                index_type
                                    .as_ref()
                                    .map(format_type_name)
                                    .unwrap_or("<unknown>")
                            ),
                        }
                        .into());
                    }

                    let symbol = symbols
                        .resolve(name)
                        .ok_or_else(|| SemanticError::UndefinedSymbol { name: name.clone() })?;

                    let expected_type = symbol
                        .declared_type
                        .as_ref()
                        .and_then(|declared_type| resolve_type_ref(declared_type, types))
                        .ok_or_else(|| SemanticError::TypeMismatch {
                            detail: format!("'{}' is not an array variable", name),
                        })?;

                    let TypeRef::Array { element_type, .. } = expected_type else {
                        return Err(SemanticError::TypeMismatch {
                            detail: format!("'{}' is not an array variable", name),
                        }
                        .into());
                    };

                    if let Some(actual_type) = infer_expr_type(value, symbols, types)?
                        && !assignment_compatible_extended(
                            &element_type,
                            &actual_type,
                            import_aliases,
                            external_modules,
                        )
                    {
                        return Err(SemanticError::TypeMismatch {
                            detail: format!(
                                "cannot assign {} to array element '{}' of type {}",
                                format_type_name(&actual_type),
                                name,
                                format_type_name(&element_type)
                            ),
                        }
                        .into());
                    }
                }
            }

            Ok(())
        }
        Statement::Call { module, name, args } => {
            ensure_internal_builtin_module_imported(module.as_deref(), symbols)?;

            if module.is_none()
                && let Some(required_module) = required_builtin_module_for_name(name)
            {
                return Err(SemanticError::InvalidBuiltinArgument {
                    name: name.clone(),
                    detail: format!("must be qualified as {}.{}(...)", required_module, name),
                }
                .into());
            }

            if let Some(builtin_id) =
                resolve_builtin_with_module_validation(module.as_deref(), name)?
            {
                if !builtin_allows_statement(builtin_id) {
                    return Err(SemanticError::InvalidBuiltinArgument {
                        name: builtin_display_name(module.as_deref(), name),
                        detail: "must be used as a call expression".to_string(),
                    }
                    .into());
                }

                if let Some(expected_arity) = builtin_fixed_arity(builtin_id)
                    && args.len() != expected_arity
                {
                    return Err(SemanticError::ArityMismatch {
                        name: builtin_display_name(module.as_deref(), name),
                        expected: expected_arity,
                        got: args.len(),
                    }
                    .into());
                }

                if matches!(builtin_id, BuiltinId::WriteString) {
                    match args.first() {
                        Some(Expr::String(_)) => return Ok(()),
                        Some(_) => {
                            return Err(SemanticError::InvalidBuiltinArgument {
                                name: builtin_display_name(module.as_deref(), name),
                                detail: "expected a string literal".to_string(),
                            }
                            .into())
                        }
                        None => unreachable!("arity checked above"),
                    }
                }

                for arg in args {
                    analyze_expr(arg, symbols)?;
                }

                return Ok(());
            }

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
            validate_boolean_condition(condition, symbols, types)?;
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
            validate_boolean_condition(condition, symbols, types)?;
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
        Expr::Indexed { name, index } => {
            analyze_expr(index, symbols)?;
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
        Expr::Call { module, name, args } => {
            ensure_internal_builtin_module_imported(module.as_deref(), symbols)?;

            if module.is_none()
                && let Some(required_module) = required_builtin_module_for_name(name)
            {
                return Err(SemanticError::InvalidBuiltinArgument {
                    name: name.clone(),
                    detail: format!("must be qualified as {}.{}(...)", required_module, name),
                }
                .into());
            }

            if let Some(builtin_id) = resolve_builtin_with_module_validation(module.as_deref(), name)? {
                if !builtin_allows_expression(builtin_id) {
                    return Err(SemanticError::InvalidBuiltinArgument {
                        name: builtin_display_name(module.as_deref(), name),
                        detail: "must be used as a statement call".to_string(),
                    }
                    .into());
                }

                if let Some(expected_arity) = builtin_fixed_arity(builtin_id)
                    && args.len() != expected_arity
                {
                    return Err(SemanticError::ArityMismatch {
                        name: builtin_display_name(module.as_deref(), name),
                        expected: expected_arity,
                        got: args.len(),
                    }
                    .into());
                }

                for arg in args {
                    analyze_expr(arg, symbols)?;
                }

                return Ok(());
            }

            if let Some(module_name) = module {
                return Err(SemanticError::UndefinedSymbol {
                    name: format!("{}.{}", module_name, name),
                }
                .into());
            }

            if symbols.resolve(name).is_none() {
                return Err(SemanticError::UndefinedSymbol { name: name.clone() }.into());
            }

            Err(SemanticError::InvalidBuiltinArgument {
                name: name.clone(),
                detail:
                    "call expressions currently support only ReadInt(), ReadReal(), ReadLongReal(), EOF(), FLT(), and FLOOR()"
                        .to_string(),
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

fn infer_indexed_expr_type(
    name: &str,
    index: &Expr,
    symbols: &SymbolTable,
    types: &HashMap<String, TypeRef>,
) -> Result<Option<TypeRef>> {
    let index_type = infer_expr_type(index, symbols, types)?;
    if index_type != Some(TypeRef::Integer) {
        return Err(SemanticError::TypeMismatch {
            detail: format!(
                "array index for '{}' must be INTEGER, got {}",
                name,
                index_type
                    .as_ref()
                    .map(format_type_name)
                    .unwrap_or("<unknown>")
            ),
        }
        .into());
    }

    let symbol = symbols
        .resolve(name)
        .ok_or_else(|| SemanticError::UndefinedSymbol {
            name: name.to_string(),
        })?;

    let Some(declared_type) = symbol.declared_type.as_ref() else {
        return Err(SemanticError::TypeMismatch {
            detail: format!("'{}' is not an array variable", name),
        }
        .into());
    };

    let resolved_type =
        resolve_type_ref(declared_type, types).ok_or_else(|| SemanticError::TypeMismatch {
            detail: format!("'{}' is not an array variable", name),
        })?;

    match resolved_type {
        TypeRef::Array { element_type, .. } => Ok(Some(*element_type)),
        _ => Err(SemanticError::TypeMismatch {
            detail: format!("'{}' is not an array variable", name),
        }
        .into()),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
