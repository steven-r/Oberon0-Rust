use std::collections::HashMap;

use anyhow::{Result, bail};

use crate::ast::{Declaration, Expr, Module, Statement};
use crate::hir::{HDeclaration, HExpr, HImportDecl, HModule, HParam, HResolvedIdent, HStatement};
use crate::symbols::SymbolKind;

#[derive(Debug, Default)]
struct Resolver {
    scopes: Vec<HashMap<String, HResolvedIdent>>,
    next_id: usize,
}

impl Resolver {
    fn new() -> Self {
        let mut resolver = Self::default();
        resolver.enter_scope();
        resolver
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str, kind: SymbolKind) -> Result<HResolvedIdent> {
        let scope = self
            .scopes
            .last_mut()
            .expect("resolver must always have an active scope");

        if scope.contains_key(name) {
            bail!("Lowering failed: duplicate symbol declaration '{}'.", name);
        }

        let resolved = HResolvedIdent {
            id: self.next_id,
            name: name.to_string(),
            kind,
        };
        self.next_id += 1;

        scope.insert(name.to_string(), resolved.clone());
        Ok(resolved)
    }

    fn resolve(&self, name: &str) -> Option<HResolvedIdent> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .cloned()
    }

    fn resolve_or_declare_var(&mut self, name: &str) -> Result<HResolvedIdent> {
        match self.resolve(name) {
            Some(resolved) => Ok(resolved),
            None => self.declare(name, SymbolKind::Variable),
        }
    }
}

pub fn lower_module(module: &Module) -> Result<HModule> {
    let mut resolver = Resolver::new();
    resolver.declare("WriteInt", SymbolKind::Procedure)?;

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
            Declaration::Var { name } => {
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

fn lower_declaration(declaration: &Declaration, resolver: &mut Resolver) -> Result<HDeclaration> {
    match declaration {
        Declaration::Const { name, value } => {
            let resolved = resolver
                .resolve(name)
                .ok_or_else(|| anyhow::anyhow!("Lowering failed: unknown constant '{}'.", name))?;
            Ok(HDeclaration::Const {
                id: resolved.id,
                name: name.clone(),
                value: *value,
            })
        }
        Declaration::Var { name } => {
            let resolved = resolver
                .resolve(name)
                .ok_or_else(|| anyhow::anyhow!("Lowering failed: unknown variable '{}'.", name))?;
            Ok(HDeclaration::Var {
                id: resolved.id,
                name: name.clone(),
            })
        }
        Declaration::Procedure {
            name,
            params,
            body,
            end_name,
        } => {
            let resolved_proc = resolver
                .resolve(name)
                .ok_or_else(|| anyhow::anyhow!("Lowering failed: unknown procedure '{}'.", name))?;

            resolver.enter_scope();
            let mut lowered_params = Vec::new();
            for param in params {
                let resolved = resolver.declare(param, SymbolKind::Parameter)?;
                lowered_params.push(HParam {
                    id: resolved.id,
                    name: param.clone(),
                });
            }

            let lowered_body = body
                .iter()
                .map(|statement| lower_statement(statement, resolver))
                .collect::<Result<Vec<_>>>()?;
            resolver.exit_scope();

            Ok(HDeclaration::Procedure {
                id: resolved_proc.id,
                name: name.clone(),
                params: lowered_params,
                body: lowered_body,
                end_name: end_name.clone(),
            })
        }
    }
}

fn lower_statement(statement: &Statement, resolver: &mut Resolver) -> Result<HStatement> {
    match statement {
        Statement::Assign { target, value } => Ok(HStatement::Assign {
            target: resolver.resolve_or_declare_var(target)?,
            value: lower_expr(value, resolver)?,
        }),
        Statement::Call { name, args } => {
            let resolved = resolver
                .resolve(name)
                .ok_or_else(|| anyhow::anyhow!("Lowering failed: unknown call target '{}'.", name))?;
            let lowered_args = args
                .iter()
                .map(|arg| lower_expr(arg, resolver))
                .collect::<Result<Vec<_>>>()?;
            Ok(HStatement::Call {
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

fn lower_expr(expr: &Expr, resolver: &Resolver) -> Result<HExpr> {
    match expr {
        Expr::Integer(value) => Ok(HExpr::Integer(*value)),
        Expr::Variable(name) => {
            let resolved = resolver
                .resolve(name)
                .ok_or_else(|| anyhow::anyhow!("Lowering failed: unknown identifier '{}'.", name))?;
            Ok(HExpr::Name(resolved))
        }
        Expr::Binary { op, left, right } => Ok(HExpr::Binary {
            op: *op,
            left: Box::new(lower_expr(left, resolver)?),
            right: Box::new(lower_expr(right, resolver)?),
        }),
    }
}
