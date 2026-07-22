//! Parser from Oberon0 source text to the syntax tree in `ast`.

use anyhow::{Context, Result, bail};
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

use crate::ast::{
    BinaryOp, Declaration, Expr, ImportDecl, LocalVarDecl, Module, ParamDecl, Statement, TypeRef,
    UnaryOp,
};

#[derive(Parser)]
#[grammar = "oberon0.pest"]
struct Oberon0Parser;

/// Parses a complete Oberon0 module into the compiler AST.
pub fn parse_module(source: &str) -> Result<Module> {
    let mut pairs = Oberon0Parser::parse(Rule::module, source).context("Invalid Oberon0 syntax")?;
    let module_pair = pairs.next().context("No module found")?;
    build_module(module_pair)
}

/// Builds a module AST node from the grammar's top-level parse pair.
fn build_module(module_pair: Pair<Rule>) -> Result<Module> {
    let mut inner = module_pair.into_inner();

    let name = take_ident(inner.next(), "module name")?;

    let mut next = inner.next().context("Unexpected end after module name")?;

    let imports = if next.as_rule() == Rule::import_section {
        let imports = parse_import_section(next)?;
        next = inner
            .next()
            .context("Unexpected end before declarations, module body, or END name")?;
        imports
    } else {
        Vec::new()
    };

    let mut declarations = Vec::new();
    while next.as_rule() == Rule::declaration_section {
        declarations.extend(parse_declaration_section(next)?);
        next = inner
            .next()
            .context("Unexpected end before module body or END name")?;
    }

    let begin_pair = next;

    let (statements, end_name_pair) = if begin_pair.as_rule() == Rule::stmt_list {
        let stmts = parse_stmt_list(begin_pair)?;
        let end_name_pair = inner
            .next()
            .context("Unexpected end after statements")?;
        (stmts, end_name_pair)
    } else {
        (Vec::new(), begin_pair)
    };

    let end_name = take_ident(Some(end_name_pair), "END module name")?;

    Ok(Module {
        name,
        end_name,
        imports,
        declarations,
        statements,
    })
}

/// Dispatches one declaration section to the matching AST builder.
fn parse_declaration_section(section: Pair<Rule>) -> Result<Vec<Declaration>> {
    let inner = section
        .into_inner()
        .next()
        .context("Empty declaration section")?;

    match inner.as_rule() {
        Rule::const_section => parse_const_section(inner),
        Rule::type_section => parse_type_section(inner),
        Rule::var_section => parse_var_section(inner),
        Rule::procedure_decl => Ok(vec![parse_procedure_decl(inner)?]),
        _ => bail!("Unknown declaration section: {:?}", inner.as_rule()),
    }
}

/// Parses a single procedure declaration, including its optional body.
fn parse_procedure_decl(decl: Pair<Rule>) -> Result<Declaration> {
    let mut parts = decl.into_inner();
    let name = take_ident(parts.next(), "procedure declaration name")?;

    // Check for export marker "*"
    let mut is_exported = false;
    let mut next = parts
        .next()
        .context("Missing data in procedure declaration")?;

    if next.as_rule() == Rule::export_marker {
        is_exported = true;
        next = parts
            .next()
            .context("Missing data after procedure export marker")?;
    }

    let mut params = Vec::new();
    let mut local_vars = Vec::new();

    if next.as_rule() == Rule::formal_params {
        params = parse_formal_params(next)?;
        next = parts
            .next()
            .context("Missing procedure body or END name")?;
    }

    while next.as_rule() == Rule::var_section {
        local_vars.extend(parse_local_var_section(next)?);
        next = parts
            .next()
            .context("Missing procedure body or END name")?;
    }

    let (body, end_name_pair) = if next.as_rule() == Rule::stmt_list {
        let body = parse_stmt_list(next)?;
        let end_name_pair = parts.next().context("Missing END procedure name")?;
        (body, end_name_pair)
    } else {
        (Vec::new(), next)
    };

    let end_name = take_ident(Some(end_name_pair), "END procedure name")?;

    Ok(Declaration::Procedure {
        name,
        params,
        local_vars,
        body,
        end_name,
        is_exported,
    })
}

fn parse_local_var_section(section: Pair<Rule>) -> Result<Vec<LocalVarDecl>> {
    let mut out = Vec::new();

    for item in section.into_inner() {
        let mut parts = item.into_inner();
        let ident_list = parts.next().context("Missing procedure-local variable names")?;
        let declared_type = parts
            .next()
            .map(|pair| parse_type_ref_name(pair.as_str().to_string()));

        for ident in ident_list.into_inner() {
            if ident.as_rule() != Rule::ident {
                bail!("Procedure-local variable name is not an identifier");
            }
            out.push(LocalVarDecl {
                name: ident.as_str().to_string(),
                declared_type: declared_type.clone(),
            });
        }
    }

    Ok(out)
}

/// Parses the positional parameter list for a procedure declaration.
fn parse_formal_params(params: Pair<Rule>) -> Result<Vec<ParamDecl>> {
    let mut out = Vec::new();
    for section in params.into_inner() {
        let mut parts = section.into_inner();
        let first = parts.next().context("Procedure parameter section is missing")?;

        let (is_var, ident_list_pair) = if first.as_rule() == Rule::var_modifier {
            (
                true,
                parts
                    .next()
                    .context("Procedure VAR parameter section is missing identifiers")?,
            )
        } else {
            (false, first)
        };

        let declared_type = parts
            .next()
            .map(|pair| parse_type_ref_name(pair.as_str().to_string()));

        for ident in ident_list_pair.into_inner() {
            if ident.as_rule() != Rule::ident {
                bail!("Procedure parameter is not an identifier");
            }
            out.push(ParamDecl {
                name: ident.as_str().to_string(),
                declared_type: declared_type.clone(),
                is_var,
            });
        }
    }
    Ok(out)
}

/// Parses a `CONST` declaration section.
fn parse_const_section(section: Pair<Rule>) -> Result<Vec<Declaration>> {
    let mut out = Vec::new();

    for item in section.into_inner() {
        let mut parts = item.into_inner();
        let name = take_ident(parts.next(), "constant name")?;
        let value_pair = parts.next().context("Missing constant value")?;
        let value = value_pair
            .as_str()
            .parse::<i64>()
            .with_context(|| format!("Invalid integer: {}", value_pair.as_str()))?;
        out.push(Declaration::Const { name, value });
    }

    Ok(out)
}

/// Parses a `TYPE` declaration section with simple named aliases.
fn parse_type_section(section: Pair<Rule>) -> Result<Vec<Declaration>> {
    let mut out = Vec::new();

    for item in section.into_inner() {
        let mut parts = item.into_inner();
        let name = take_ident(parts.next(), "type name")?;

        // Check for export marker "*"
        let mut is_exported = false;
        let mut type_ref_pair = parts.next().context("Missing type reference")?;

        if type_ref_pair.as_rule() == Rule::export_marker {
            is_exported = true;
            type_ref_pair = parts.next().context("Missing type reference after export marker")?;
        }

        let target = parse_type_ref(type_ref_pair)?;
        out.push(Declaration::Type { name, target, is_exported });
    }

    Ok(out)
}

/// Parses a `VAR` declaration section.
fn parse_var_section(section: Pair<Rule>) -> Result<Vec<Declaration>> {
    let mut out = Vec::new();

    for item in section.into_inner() {
        let mut parts = item.into_inner();
        let ident_list = parts.next().context("Missing variable names")?;
        let declared_type = parts
            .next()
            .map(|pair| parse_type_ref_name(pair.as_str().to_string()));

        for ident in ident_list.into_inner() {
            if ident.as_rule() != Rule::ident {
                bail!("Variable name is not an identifier");
            }
            out.push(Declaration::Var {
                name: ident.as_str().to_string(),
                declared_type: declared_type.clone(),
            });
        }
    }

    Ok(out)
}

/// Parses the optional module import section.
fn parse_import_section(section: Pair<Rule>) -> Result<Vec<ImportDecl>> {
    section
        .into_inner()
        .map(parse_import_item)
        .collect::<Result<Vec<_>>>()
}

/// Parses a single import item, including optional aliasing.
fn parse_import_item(item: Pair<Rule>) -> Result<ImportDecl> {
    let mut inner = item.into_inner();
    let first = take_ident(inner.next(), "import name")?;
    let second = inner.next().map(|p| p.as_str().to_string());

    let (local_name, external_name) = match second {
        Some(ext) => (first, ext),
        None => (first.clone(), first),
    };

    Ok(ImportDecl {
        local_name,
        external_name,
    })
}

/// Parses a sequence of statements from a grammar list node.
fn parse_stmt_list(list: Pair<Rule>) -> Result<Vec<Statement>> {
    list.into_inner()
        .map(parse_statement)
        .collect::<Result<Vec<_>>>()
}

/// Parses a single statement node.
fn parse_statement(stmt: Pair<Rule>) -> Result<Statement> {
    match stmt.as_rule() {
        Rule::assign_stmt => {
            let mut parts = stmt.into_inner();
            let target = take_ident(parts.next(), "assignment target")?;
            let value = parse_expr(parts.next().context("Missing expression")?)?;
            Ok(Statement::Assign { target, value })
        }
        Rule::call_stmt => {
            let mut parts = stmt.into_inner();
            let qualified_pair = parts.next().context("Missing procedure name in call")?;
            let (module, name) = parse_qualified_ident(qualified_pair)?;
            let args = match parts.next() {
                Some(arg_list) => parse_arg_list(arg_list)?,
                None => Vec::new(),
            };
            Ok(Statement::Call { module, name, args })
        }
        Rule::if_stmt => {
            let mut parts = stmt.into_inner();
            let condition = parse_expr(parts.next().context("Missing IF condition")?)?;

            let mut then_branch = Vec::new();
            let mut else_branch = None;

            if let Some(next) = parts.next() {
                match next.as_rule() {
                    Rule::stmt_list => {
                        then_branch = parse_stmt_list(next)?;
                        if let Some(else_section) = parts.next() {
                            else_branch = Some(parse_else_section(else_section)?);
                        }
                    }
                    Rule::else_section => {
                        else_branch = Some(parse_else_section(next)?);
                    }
                    _ => bail!("Unknown IF branch: {:?}", next.as_rule()),
                }
            }

            Ok(Statement::If {
                condition,
                then_branch,
                else_branch,
            })
        }
        Rule::while_stmt => {
            let mut parts = stmt.into_inner();
            let condition = parse_expr(parts.next().context("Missing WHILE condition")?)?;
            let body = match parts.next() {
                Some(stmt_list) => parse_stmt_list(stmt_list)?,
                None => Vec::new(),
            };

            Ok(Statement::While { condition, body })
        }
        Rule::statement => {
            let inner = stmt.into_inner().next().context("Empty statement")?;
            parse_statement(inner)
        }
        _ => bail!("Unknown statement: {:?}", stmt.as_rule()),
    }
}

fn parse_else_section(section: Pair<Rule>) -> Result<Vec<Statement>> {
    match section.into_inner().next() {
        Some(stmt_list) => parse_stmt_list(stmt_list),
        None => Ok(Vec::new()),
    }
}

fn parse_arg_list(arg_list: Pair<Rule>) -> Result<Vec<Expr>> {
    arg_list
        .into_inner()
        .map(parse_expr)
        .collect::<Result<Vec<_>>>()
}

fn parse_expr(expr: Pair<Rule>) -> Result<Expr> {
    let mut inner = expr.into_inner();
    let left = parse_simple_expr(inner.next().context("Empty expression")?)?;

    if let Some(op) = inner.next() {
        let right = parse_simple_expr(inner.next().context("Missing right relational operand")?)?;
        Ok(Expr::Binary {
            op: parse_rel_op(op)?,
            left: Box::new(left),
            right: Box::new(right),
        })
    } else {
        Ok(left)
    }
}

fn parse_simple_expr(expr: Pair<Rule>) -> Result<Expr> {
    let mut inner = expr.into_inner();
    let mut unary_sign = None;

    let mut next = inner.next().context("Empty simple expression")?;
    if next.as_rule() == Rule::unary_sign {
        unary_sign = Some(parse_unary_sign(next)?);
        next = inner
            .next()
            .context("Missing term after unary sign")?;
    }

    let mut left = parse_term(next)?;
    if let Some(op) = unary_sign {
        left = Expr::Unary {
            op,
            value: Box::new(left),
        };
    }

    while let Some(op) = inner.next() {
        let right_term = inner.next().context("Missing right term")?;
        let right = parse_term(right_term)?;
        left = Expr::Binary {
            op: parse_add_op(op)?,
            left: Box::new(left),
            right: Box::new(right),
        };
    }

    Ok(left)
}

fn parse_term(term: Pair<Rule>) -> Result<Expr> {
    let mut inner = term.into_inner();
    let mut left = parse_factor(inner.next().context("Empty term")?)?;

    while let Some(op) = inner.next() {
        let right_factor = inner.next().context("Missing right factor")?;
        let right = parse_factor(right_factor)?;
        left = Expr::Binary {
            op: parse_mul_op(op)?,
            left: Box::new(left),
            right: Box::new(right),
        };
    }

    Ok(left)
}

fn parse_factor(factor: Pair<Rule>) -> Result<Expr> {
    let inner = factor.into_inner().next().context("Empty factor")?;
    match inner.as_rule() {
        Rule::not_factor => {
            let value = inner
                .into_inner()
                .next()
                .context("Missing operand for unary '~'")?;
            Ok(Expr::Unary {
                op: UnaryOp::Not,
                value: Box::new(parse_factor(value)?),
            })
        }
        Rule::primary_factor => parse_primary_factor(inner),
        _ => bail!("Unknown factor: {:?}", inner.as_rule()),
    }
}

fn parse_primary_factor(primary: Pair<Rule>) -> Result<Expr> {
    let inner = primary
        .into_inner()
        .next()
        .context("Empty primary factor")?;

    match inner.as_rule() {
        Rule::integer => {
            let value = inner
                .as_str()
                .parse::<i64>()
                .with_context(|| format!("Invalid integer: {}", inner.as_str()))?;
            Ok(Expr::Integer(value))
        }
        Rule::string => Ok(Expr::String(parse_pascal_string(inner.as_str())?)),
        Rule::call_or_var => {
            // `arg_list` is optional and does not produce a pair for empty calls like `ReadInt()`,
            // so we must detect call syntax from the raw text to preserve zero-arg calls.
            let is_call_syntax = inner.as_str().contains('(');
            let mut parts = inner.into_inner();
            let qualified_pair = parts.next().context("Missing qualified identifier")?;
            let (module, name) = parse_qualified_ident(qualified_pair)?;

            if is_call_syntax {
                let args = match parts.next() {
                    Some(arg_list) => parse_arg_list(arg_list)?,
                    None => Vec::new(),
                };
                Ok(Expr::Call { module, name, args })
            } else {
                match module {
                    Some(mod_name) => Ok(Expr::QualifiedVariable { module: mod_name, name }),
                    None => Ok(Expr::Variable(name)),
                }
            }
        }
        Rule::ident => Ok(Expr::Variable(inner.as_str().to_string())),
        Rule::expr => parse_expr(inner),
        _ => bail!("Unknown primary factor: {:?}", inner.as_rule()),
    }
}

fn parse_pascal_string(raw: &str) -> Result<String> {
    if raw.len() < 2 || !raw.starts_with('"') || !raw.ends_with('"') {
        bail!("String literal is malformed: {}", raw);
    }

    Ok(raw[1..raw.len() - 1].replace("\"\"", "\""))
}

/// Parses a qualified identifier (module.name or just name).
/// Returns (Optional<module>, name).
fn parse_qualified_ident(pair: Pair<Rule>) -> Result<(Option<String>, String)> {
    let mut inner = pair.into_inner();

    let first = take_ident(inner.next(), "first part of identifier")?;

    // If there's a second pair, it might be the "." operator (which we skip) or the second ident
    // When parsing "B.IntType", the structure is: ident("B") ~ ("." ~ ident("IntType"))?
    // This gives us: ident, ".", ident as inner pairs
    // So we need to skip the "." operator and get the next ident
    if let Some(next_pair) = inner.next() {
        // This could be a rule (like ".") or an ident
        if next_pair.as_rule() == Rule::ident {
            // Simple case: just "module" followed by another "ident" (shouldn't happen with current grammar)
            Ok((Some(first), next_pair.as_str().to_string()))
        } else {
            // Expected case: "." operator followed by ident
            // Try to get the next ident
            if let Some(second) = inner.next() {
                let name = take_ident(Some(second), "second part of identifier")?;
                Ok((Some(first), name))
            } else {
                // Just in case, return first as name
                Ok((None, first))
            }
        }
    } else {
        Ok((None, first))
    }
}

fn parse_type_ref(pair: Pair<Rule>) -> Result<TypeRef> {
    let qualified = pair.into_inner().next().context("Missing qualified_ident in type_ref")?;
    let (module, name) = parse_qualified_ident(qualified)?;

    match name.as_str() {
        "INTEGER" if module.is_none() => Ok(TypeRef::Integer),
        "BOOLEAN" if module.is_none() => Ok(TypeRef::Boolean),
        "REAL" if module.is_none() => Ok(TypeRef::Real),
        "LONGREAL" if module.is_none() => Ok(TypeRef::LongReal),
        _ => {
            match module {
                Some(mod_name) => {
                    Ok(TypeRef::Qualified { module: mod_name, name })
                },
                None => {
                    Ok(TypeRef::Named(name))
                },
            }
        }
    }
}

fn parse_type_ref_name(name: String) -> TypeRef {
    match name.as_str() {
        "INTEGER" => TypeRef::Integer,
        "BOOLEAN" => TypeRef::Boolean,
        "REAL" => TypeRef::Real,
        "LONGREAL" => TypeRef::LongReal,
        _ => {
            // Check if it's a qualified name (contains a dot)
            if let Some(dot_pos) = name.find('.') {
                let module = name[..dot_pos].to_string();
                let type_name = name[dot_pos + 1..].to_string();
                TypeRef::Qualified { module, name: type_name }
            } else {
                TypeRef::Named(name)
            }
        }
    }
}

fn parse_add_op(op: Pair<Rule>) -> Result<BinaryOp> {
    match op.as_str() {
        "+" => Ok(BinaryOp::Add),
        "-" => Ok(BinaryOp::Sub),
        "OR" => Ok(BinaryOp::Or),
        other => bail!("Unknown add operator: {}", other),
    }
}

fn parse_mul_op(op: Pair<Rule>) -> Result<BinaryOp> {
    match op.as_str() {
        "*" => Ok(BinaryOp::Mul),
        "/" => Ok(BinaryOp::Div),
        "DIV" => Ok(BinaryOp::IntDiv),
        "MOD" => Ok(BinaryOp::Mod),
        "&" => Ok(BinaryOp::And),
        other => bail!("Unknown mul operator: {}", other),
    }
}

fn parse_rel_op(op: Pair<Rule>) -> Result<BinaryOp> {
    match op.as_str() {
        "=" => Ok(BinaryOp::Eq),
        "#" => Ok(BinaryOp::Ne),
        "<" => Ok(BinaryOp::Lt),
        "<=" => Ok(BinaryOp::Le),
        ">" => Ok(BinaryOp::Gt),
        ">=" => Ok(BinaryOp::Ge),
        other => bail!("Unknown relational operator: {}", other),
    }
}

fn parse_unary_sign(op: Pair<Rule>) -> Result<UnaryOp> {
    match op.as_str() {
        "+" => Ok(UnaryOp::Plus),
        "-" => Ok(UnaryOp::Minus),
        other => bail!("Unknown unary sign operator: {}", other),
    }
}

fn take_ident(pair: Option<Pair<Rule>>, label: &str) -> Result<String> {
    let pair = pair.with_context(|| format!("{} is missing", label))?;
    if pair.as_rule() != Rule::ident {
        bail!("{} is not an identifier", label);
    }
    Ok(pair.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::parse_module;
    use crate::ast::{BinaryOp, Expr, Statement, UnaryOp};
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
            "bad_string_literal.ob0" => replace_required(source, "WriteString(\"Hello)", "WriteString(\"Hello\")"),
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
            "import_leading_dot_module.ob0" => replace_required(source, "IMPORT .Module;", "IMPORT ModuleB;"),
            "if_call_condition.ob0" => replace_required(source, "IF WriteInt(1 THEN", "IF 1 THEN"),
            "if_missing_end.ob0" => replace_required(source, "WriteInt(1)\nEND Main.", "WriteInt(1)\n  END\nEND Main."),
            "missing_module_dot.ob0" => format!("{}.", source.trim_end()),
            "operator_div_missing_rhs.ob0" => {
                replace_required(source, "x := 7 DIV", "x := 7 DIV 2")
            }
            "operator_not_missing_operand.ob0" => {
                replace_required(source, "flag := ~", "flag := ~flag")
            }
            "qualified_member_missing_name.ob0" => {
                replace_required(source, "B.", "B")
            }
            "relational_missing_rhs.ob0" => {
                replace_required(source, "b := 1 =", "b := 1 = 1")
            }
            "procedure_missing_semicolon.ob0" => {
                replace_required(source, "PROCEDURE P(x)\nBEGIN", "PROCEDURE P(x);\nBEGIN")
            }
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
            "duplicate_var_decl.ob0" => replace_required(source, "VAR x, x: INTEGER;", "VAR x, y: INTEGER;"),
            "end_name_mismatch.ob0" => replace_required(source, "END NotMain.", "END Main."),
            "eof_with_arg.ob0" => replace_required(source, "EOF(1)", "EOF()"),
            "if_undefined_condition.ob0" => replace_required(source, "IF unknown THEN", "IF 1 THEN"),
            "operator_div_requires_integer.ob0" => {
                replace_required(source, "x := src DIV 2", "x := 7 DIV 2")
            }
            "operator_not_requires_boolean.ob0" => replace_required(source, "b := ~1", "b := ~b"),
            "operator_or_requires_boolean.ob0" => replace_required(source, "b := 1 OR 0", "b := b OR b"),
            "operator_unary_sign_requires_numeric.ob0" => {
                replace_required(source, "x := +flag", "x := +1")
            }
            "relational_requires_numeric.ob0" => {
                replace_required(source, "b1 := b1 < b2", "b1 := 1 < 1")
            }
            "procedure_call_arity_mismatch.ob0" => replace_required(source, "AddAndPrint(2)", "AddAndPrint(2, 3)"),
            "procedure_end_name_mismatch.ob0" => replace_required(source, "END WrongName;", "END Echo;"),
            "procedure_local_var_self_shadows_type_alias.ob0" => {
                replace_required(source, "VAR Count: Count;", "VAR value: Count;")
            }
            "procedure_local_var_shadows_builtin_type.ob0" => {
                replace_required(source, "VAR INTEGER: INTEGER;", "VAR value: INTEGER;")
            }
            "qualified_call_member_unresolved.ob0" => {
                replace_required(source, "B.HELLO", "WriteLn()")
            }
            "qualified_call_non_exported.ob0" => {
                replace_required(source, "B.NonExportedProcedure()", "WriteLn()")
            }
            "qualified_type_reference_unsupported.ob0" => {
                replace_required(source, "VAR x: B.IntType;", "VAR x: INTEGER;")
            }
            "readint_statement_call.ob0" => r#"
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
            "typed_var_unknown_type.ob0" => replace_required(source, "VAR x: Missing;", "VAR x: INTEGER;"),
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
            "while_undefined_in_body.ob0" => replace_required(source, "x := y - 1", "x := x - 1"),
            "writeint_string_literal.ob0" => replace_required(source, "WriteInt(\"Hello\")", "WriteInt(1)"),
            "writeln_with_arg.ob0" => replace_required(source, "WriteLn(1)", "WriteLn()"),
            "writestring_missing_arg.ob0" => replace_required(source, "WriteString", "WriteString(\"Hello\")"),
            "writestring_non_string_arg.ob0" => replace_required(source, "WriteString(1)", "WriteString(\"1\")"),
            "writestring_too_many_args.ob0" => {
                replace_required(source, "WriteString(\"Hello\", \"World\")", "WriteString(\"Hello\")")
            }
            "qualified_call_unknown_alias.ob0" => replace_required(source, "C.HELLO()", "B.HELLO()"),
            "qualified_type_reference_non_exported.ob0" => replace_required(source, "B.HiddenType", "B.IntType"),
            other => panic!("missing semantic invalid repair mapping for {other}"),
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
                panic!(
                    "expected repaired parser case {name} to scan successfully, got: {err}"
                )
            });
            parse_module(&repaired).unwrap_or_else(|err| {
                panic!(
                    "expected repaired parser case {name} to parse successfully, got: {err}"
                )
            });
        }
    }

    #[test]
    fn semantic_valid_corpus_passes() {
        for (name, source) in read_dir_sources("tests/semantic_cases/valid") {
            scan(&source)
                .unwrap_or_else(|err| panic!("expected valid scan for semantic case {name}, got: {err}"));
            let module = parse_module(&source)
                .unwrap_or_else(|err| panic!("expected parse for semantic case {name}, got: {err}"));
            analyze(&module, None).unwrap_or_else(|err| {
                panic!("expected semantic success for {name}, got error: {err}")
            });
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
                panic!(
                    "expected repaired semantic case {name} to scan successfully, got: {err}"
                )
            });
            let module = parse_module(&repaired).unwrap_or_else(|err| {
                panic!(
                    "expected repaired semantic case {name} to parse successfully, got: {err}"
                )
            });
            analyze(&module, None).unwrap_or_else(|err| {
                panic!(
                    "expected repaired semantic case {name} to pass semantic analysis, got: {err}"
                )
            });
        }
    }

    #[test]
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
                    name,
                    is_exported,
                    ..
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

        let Statement::Call {
            module,
            name,
            args,
        } = &parsed.statements[0]
        else {
            panic!("expected first statement to be a call");
        };
        assert_eq!(module.as_deref(), Some("B"));
        assert_eq!(name, "HELLO");
        assert!(args.is_empty());

        let Statement::Assign { target, value } = &parsed.statements[1] else {
            panic!("expected second statement to be an assignment");
        };
        assert_eq!(target, "x");
        assert!(matches!(
            value,
            Expr::QualifiedVariable { module, name }
            if module == "B" && name == "value"
        ));
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
        assert!(matches!(left.as_ref(), Expr::Unary { op: UnaryOp::Minus, .. }));
        assert!(matches!(
            right.as_ref(),
            Expr::Binary { op: BinaryOp::Mod, .. }
        ));

        let Statement::Assign { value, .. } = &module.statements[1] else {
            panic!("expected second statement to be assignment");
        };
        let Expr::Binary { op, left, right } = value else {
            panic!("expected boolean binary expression");
        };
        assert!(matches!(op, BinaryOp::And));
        assert!(matches!(left.as_ref(), Expr::Unary { op: UnaryOp::Not, .. }));
        assert!(matches!(right.as_ref(), Expr::Binary { op: BinaryOp::Or, .. }));
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
}
