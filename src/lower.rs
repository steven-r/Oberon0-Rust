//! Lowers the parsed AST into a name-resolved HIR for code generation.

use anyhow::{Result, bail};

use crate::ast::{Declaration, Expr, Module, Statement};
use crate::hir::{HDeclaration, HExpr, HImportDecl, HModule, HParam, HResolvedIdent, HStatement};
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

    /// Declares a resolved identifier and assigns it the next stable id.
    fn declare(&mut self, name: &str, kind: SymbolKind) -> Result<HResolvedIdent> {
        let resolved = HResolvedIdent {
            id: self.next_id,
            name: name.to_string(),
            kind,
        };
        self.next_id += 1;

        self.scopes.declare(name, resolved.clone(), |name| {
            anyhow::anyhow!("Lowering failed: duplicate symbol declaration '{}'.", name)
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
    resolver.declare("ReadInt", SymbolKind::Procedure)?;
    resolver.declare("EOF", SymbolKind::Procedure)?;

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
            Ok(HDeclaration::Const {
                id: resolved.id,
                name: name.clone(),
                value: *value,
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
        Statement::Assign { target, value } => {
            let resolved_target = resolver.resolve(target).ok_or_else(|| {
                anyhow::anyhow!(
                    "Lowering invariant violated: unresolved assignment target '{}'.",
                    target
                )
            })?;

            Ok(HStatement::Assign {
                target: resolved_target,
                value: lower_expr(value, resolver)?,
            })
        }
        Statement::Call { name, args, .. } => {
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

/// Lowers one expression into its resolved HIR form.
fn lower_expr(expr: &Expr, resolver: &Resolver) -> Result<HExpr> {
    match expr {
        Expr::Integer(value) => Ok(HExpr::Integer(*value)),
        Expr::String(value) => Ok(HExpr::String(value.clone())),
        Expr::QualifiedVariable { module: _, name: _ } => {
            bail!("Qualified variables are not yet supported in code generation")
        }
        Expr::Variable(name) => {
            let resolved = resolver
                .resolve(name)
                .ok_or_else(|| anyhow::anyhow!("Lowering failed: unknown identifier '{}'.", name))?;
            Ok(HExpr::Name(resolved))
        }
        Expr::Call { name, args, module: _ } => {
            let resolved = resolver
                .resolve(name)
                .ok_or_else(|| anyhow::anyhow!("Lowering failed: unknown call target '{}'.", name))?;
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

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::lower_module;
    use crate::ast::TypeRef;
    use crate::hir::{HDeclaration, HExpr, HStatement};
    use crate::parser::parse_module;
    use crate::semantic::analyze;

    fn lower_from_source(source: &str) -> Result<crate::hir::HModule> {
        let module = parse_module(source)?;
        analyze(&module, None)?;
        lower_module(&module)
    }

    fn collect_assign_target_ids(stmts: &[HStatement], out: &mut Vec<usize>) {
        for stmt in stmts {
            match stmt {
                HStatement::Assign { target, .. } => out.push(target.id),
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
VAR x;
PROCEDURE P(p);
BEGIN
  IF p THEN
    x := p;
    WHILE p DO
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
        assert_eq!(proc.1.len(), 0, "expected no implicit procedure local variable");

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
                HExpr::Name(ident) => assert_eq!(ident.id, proc.0[0].id),
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
  x := 1
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

        assert_eq!(local_vars.len(), 1, "procedure P should have one local variable");
        assert_eq!(local_vars[0].name, "x");

        let assigned_id = body
            .iter()
            .find_map(|stmt| match stmt {
                HStatement::Assign { target, .. } => Some(target.id),
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
        let err = lower_module(&module).expect_err("lowering should fail on unresolved assignment target");
        let msg = err.to_string();
        assert!(
            msg.contains("Lowering invariant violated: unresolved assignment target 'y'."),
            "unexpected lowering error message: {msg}"
        );
    }
}
