//! Lowers the parsed AST into a name-resolved HIR for code generation.

use anyhow::{Result, bail};

use crate::ast::{AssignTarget, Declaration, Expr, Module, Statement};
use crate::expression_constant_handler::combine_expression;
use crate::hir::{
    HDeclaration, HExpr, HImportDecl, HModule, HParam, HResolvedIdent, HStatement, HTarget,
};
use crate::scope::ScopedMap;
use crate::symbols::SymbolKind;

#[derive(Debug)]
/// Tracks lexical scopes while assigning stable ids to resolved identifiers.
struct Resolver {
    scopes: ScopedMap<HResolvedIdent>,
    next_id: usize,
}

impl Resolver {
    /// Creates a resolver with a root scope and id counter starting at zero.
    fn new() -> Self {
        Self {
            scopes: ScopedMap::new(),
            next_id: 0,
        }
    }

    /// Enters a nested lexical scope.
    fn enter_scope(&mut self) {
        self.scopes.enter_scope();
    }

    /// Exits the current lexical scope.
    fn exit_scope(&mut self) {
        self.scopes.exit_scope();
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn declare_on_duplicate(name: &str) -> anyhow::Error {
        anyhow::anyhow!("Lowering failed: duplicate symbol declaration '{}'.", name)
    }

    /// Declares a resolved identifier and assigns it the next stable id.
    fn declare(&mut self, name: &str, kind: SymbolKind) -> Result<HResolvedIdent> {
        let resolved = HResolvedIdent {
            id: self.next_id,
            name: name.to_string(),
            kind,
        };
        self.next_id += 1;

        self.scopes.declare(name, resolved.clone(), |name| {
            Self::declare_on_duplicate(name)
        })?;

        Ok(resolved)
    }

    /// Resolves a name using lexical scoping rules.
    fn resolve(&self, name: &str) -> Option<HResolvedIdent> {
        self.scopes.resolve(name).cloned()
    }

    /// Returns the symbols declared directly in the active scope.
    fn current_scope_symbols(&self) -> Vec<HResolvedIdent> {
        self.scopes.current_scope_values()
    }
}

/// Converts the parsed AST into HIR with resolved identifiers.
pub fn lower_module(module: &Module) -> Result<HModule> {
    let mut resolver = Resolver::new();
    resolver.declare("WriteInt", SymbolKind::Procedure)?;
    resolver.declare("WriteString", SymbolKind::Procedure)?;
    resolver.declare("WriteLn", SymbolKind::Procedure)?;
    resolver.declare("WriteReal", SymbolKind::Procedure)?;
    resolver.declare("WriteLongReal", SymbolKind::Procedure)?;
    resolver.declare("ReadInt", SymbolKind::Procedure)?;
    resolver.declare("ReadReal", SymbolKind::Procedure)?;
    resolver.declare("ReadLongReal", SymbolKind::Procedure)?;
    resolver.declare("EOF", SymbolKind::Procedure)?;
    resolver.declare("FLT", SymbolKind::Procedure)?;
    resolver.declare("FLOOR", SymbolKind::Procedure)?;

    let imports = module
        .imports
        .iter()
        .map(|import| {
            resolver.declare(&import.local_name, SymbolKind::Procedure)?;
            Ok(HImportDecl {
                local_name: import.local_name.clone(),
                external_name: import.external_name.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    for declaration in &module.declarations {
        match declaration {
            Declaration::Const { name, .. } => {
                resolver.declare(name, SymbolKind::Constant)?;
            }
            Declaration::Type { name, .. } => {
                resolver.declare(name, SymbolKind::TypeName)?;
            }
            Declaration::Var { name, .. } => {
                resolver.declare(name, SymbolKind::Variable)?;
            }
            Declaration::Procedure { name, .. } => {
                resolver.declare(name, SymbolKind::Procedure)?;
            }
        }
    }

    let declarations = module
        .declarations
        .iter()
        .map(|declaration| lower_declaration(declaration, &mut resolver))
        .collect::<Result<Vec<_>>>()?;

    let statements = module
        .statements
        .iter()
        .map(|statement| lower_statement(statement, &mut resolver))
        .collect::<Result<Vec<_>>>()?;

    Ok(HModule {
        name: module.name.clone(),
        end_name: module.end_name.clone(),
        imports,
        declarations,
        statements,
    })
}

/// Lowers one top-level declaration into its resolved HIR form.
fn lower_declaration(declaration: &Declaration, resolver: &mut Resolver) -> Result<HDeclaration> {
    match declaration {
        Declaration::Const { name, value } => {
            let resolved = resolver
                .resolve(name)
                .ok_or_else(|| anyhow::anyhow!("Lowering failed: unknown constant '{}'.", name))?;
            let compiled = combine_expression(value)?;
            let value = lower_expr(&compiled, resolver)?;
            Ok(HDeclaration::Const {
                id: resolved.id,
                name: name.clone(),
                value: value,
            })
        }
        Declaration::Type { name, target, .. } => {
            let resolved = resolver
                .resolve(name)
                .ok_or_else(|| anyhow::anyhow!("Lowering failed: unknown type '{}'.", name))?;
            Ok(HDeclaration::Type {
                id: resolved.id,
                name: name.clone(),
                target: target.clone(),
            })
        }
        Declaration::Var {
            name,
            declared_type,
        } => {
            let resolved = resolver
                .resolve(name)
                .ok_or_else(|| anyhow::anyhow!("Lowering failed: unknown variable '{}'.", name))?;
            Ok(HDeclaration::Var {
                id: resolved.id,
                name: name.clone(),
                declared_type: declared_type.clone(),
            })
        }
        Declaration::Procedure {
            name,
            params,
            local_vars,
            body,
            end_name,
            ..
        } => {
            let resolved_proc = resolver
                .resolve(name)
                .ok_or_else(|| anyhow::anyhow!("Lowering failed: unknown procedure '{}'.", name))?;

            resolver.enter_scope();
            let mut lowered_params = Vec::new();
            for param in params {
                let resolved = resolver.declare(&param.name, SymbolKind::Parameter)?;
                lowered_params.push(HParam {
                    id: resolved.id,
                    name: param.name.clone(),
                    declared_type: param.declared_type.clone(),
                    is_var: param.is_var,
                });
            }

            for local_var in local_vars {
                resolver.declare(&local_var.name, SymbolKind::Variable)?;
            }

            let lowered_body = body
                .iter()
                .map(|statement| lower_statement(statement, resolver))
                .collect::<Result<Vec<_>>>()?;

            let mut local_vars = resolver
                .current_scope_symbols()
                .iter()
                .filter(|symbol| symbol.kind == SymbolKind::Variable)
                .cloned()
                .collect::<Vec<_>>();
            local_vars.sort_by_key(|v| v.id);

            resolver.exit_scope();

            Ok(HDeclaration::Procedure {
                id: resolved_proc.id,
                name: name.clone(),
                params: lowered_params,
                local_vars,
                body: lowered_body,
                end_name: end_name.clone(),
            })
        }
    }
}

/// Lowers one statement while preserving the current lexical scope.
fn lower_statement(statement: &Statement, resolver: &mut Resolver) -> Result<HStatement> {
    match statement {
        Statement::Assign { target, value } => Ok(HStatement::Assign {
            target: lower_assign_target(target, resolver)?,
            value: lower_expr(value, resolver)?,
        }),
        Statement::Call {
            module, name, args, ..
        } => {
            let resolved = resolver.resolve(name).ok_or_else(|| {
                anyhow::anyhow!("Lowering failed: unknown call target '{}'.", name)
            })?;
            let lowered_args = args
                .iter()
                .map(|arg| lower_expr(arg, resolver))
                .collect::<Result<Vec<_>>>()?;
            Ok(HStatement::Call {
                module: module.clone(),
                name: resolved,
                args: lowered_args,
            })
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let then_branch = then_branch
                .iter()
                .map(|stmt| lower_statement(stmt, resolver))
                .collect::<Result<Vec<_>>>()?;

            let else_branch = else_branch
                .as_ref()
                .map(|branch| {
                    branch
                        .iter()
                        .map(|stmt| lower_statement(stmt, resolver))
                        .collect::<Result<Vec<_>>>()
                })
                .transpose()?;

            Ok(HStatement::If {
                condition: lower_expr(condition, resolver)?,
                then_branch,
                else_branch,
            })
        }
        Statement::While { condition, body } => Ok(HStatement::While {
            condition: lower_expr(condition, resolver)?,
            body: body
                .iter()
                .map(|stmt| lower_statement(stmt, resolver))
                .collect::<Result<Vec<_>>>()?,
        }),
    }
}

/// Lowers one expression into its resolved HIR form.
fn lower_expr(expr: &Expr, resolver: &Resolver) -> Result<HExpr> {
    match expr {
        Expr::Integer(value) => Ok(HExpr::Integer(*value)),
        Expr::Real(value) => Ok(HExpr::Real(*value)),
        Expr::LongReal(value) => Ok(HExpr::LongReal(*value)),
        Expr::Boolean(value) => Ok(HExpr::Boolean(*value)),
        Expr::String(value) => Ok(HExpr::String(value.clone())),
        Expr::Indexed { name, index } => {
            let resolved = resolver.resolve(name).ok_or_else(|| {
                anyhow::anyhow!("Lowering failed: unknown identifier '{}'.", name)
            })?;
            Ok(HExpr::Indexed {
                name: resolved,
                index: Box::new(lower_expr(index, resolver)?),
            })
        }
        Expr::QualifiedVariable { module: _, name: _ } => {
            bail!("Qualified variables are not yet supported in code generation")
        }
        Expr::Variable(name) => {
            let resolved = resolver.resolve(name).ok_or_else(|| {
                anyhow::anyhow!("Lowering failed: unknown identifier '{}'.", name)
            })?;
            Ok(HExpr::Name(resolved))
        }
        Expr::Call {
            name,
            args,
            module: _,
        } => {
            let resolved = resolver.resolve(name).ok_or_else(|| {
                anyhow::anyhow!("Lowering failed: unknown call target '{}'.", name)
            })?;
            let lowered_args = args
                .iter()
                .map(|arg| lower_expr(arg, resolver))
                .collect::<Result<Vec<_>>>()?;
            Ok(HExpr::Call {
                name: resolved,
                args: lowered_args,
            })
        }
        Expr::Unary { op, value } => Ok(HExpr::Unary {
            op: *op,
            value: Box::new(lower_expr(value, resolver)?),
        }),
        Expr::Binary { op, left, right } => Ok(HExpr::Binary {
            op: *op,
            left: Box::new(lower_expr(left, resolver)?),
            right: Box::new(lower_expr(right, resolver)?),
        }),
    }
}

fn lower_assign_target(target: &AssignTarget, resolver: &Resolver) -> Result<HTarget> {
    match target {
        AssignTarget::Name(name) => {
            let resolved_target = resolver.resolve(name).ok_or_else(|| {
                anyhow::anyhow!(
                    "Lowering invariant violated: unresolved assignment target '{}'.",
                    name
                )
            })?;

            Ok(HTarget::Name(resolved_target))
        }
        AssignTarget::Indexed { name, index } => {
            let resolved_target = resolver.resolve(name).ok_or_else(|| {
                anyhow::anyhow!(
                    "Lowering invariant violated: unresolved assignment target '{}'.",
                    name
                )
            })?;

            Ok(HTarget::Indexed {
                name: resolved_target,
                index: lower_expr(index, resolver)?,
            })
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
