//! Parser from Oberon0 source text to the syntax tree in `ast`.

use anyhow::{Context, Result, bail};
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

use crate::ast::{
    AssignTarget, BinaryOp, Declaration, Expr, ImportDecl, LocalVarDecl, Module, ParamDecl,
    Statement, TypeRef, UnaryOp,
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
        let end_name_pair = inner.next().context("Unexpected end after statements")?;
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
        next = parts.next().context("Missing procedure body or END name")?;
    }

    while next.as_rule() == Rule::var_section {
        local_vars.extend(parse_local_var_section(next)?);
        next = parts.next().context("Missing procedure body or END name")?;
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
        let ident_list = parts
            .next()
            .context("Missing procedure-local variable names")?;
        let declared_type = parts.next().map(parse_type_ref).transpose()?;

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
        let first = parts
            .next()
            .context("Procedure parameter section is missing")?;

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

        let declared_type = parts.next().map(parse_type_ref).transpose()?;

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
        let value = parse_expr(value_pair)?;
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
            type_ref_pair = parts
                .next()
                .context("Missing type reference after export marker")?;
        }

        let target = parse_type_ref(type_ref_pair)?;
        out.push(Declaration::Type {
            name,
            target,
            is_exported,
        });
    }

    Ok(out)
}

/// Parses a `VAR` declaration section.
fn parse_var_section(section: Pair<Rule>) -> Result<Vec<Declaration>> {
    let mut out = Vec::new();

    for item in section.into_inner() {
        let mut parts = item.into_inner();
        let ident_list = parts.next().context("Missing variable names")?;
        let declared_type = parts.next().map(parse_type_ref).transpose()?;

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
            let target =
                parse_assignment_target(parts.next().context("Missing assignment target")?)?;
            let value = parse_expr(parts.next().context("Missing expression")?)?;
            Ok(Statement::Assign { target, value })
        }
        Rule::call_stmt => {
            let raw = stmt.as_str().trim();
            let (designator_text, args_text) = if raw.contains('(') {
                split_call_text(raw)?
            } else {
                (raw, "")
            };
            let designator_pair = Oberon0Parser::parse(Rule::designator, designator_text)
                .context("Invalid procedure name in call")?
                .next()
                .context("Missing procedure name in call")?;
            let (module, name, indexes) = parse_designator(designator_pair)?;
            if !indexes.is_empty() {
                bail!("Indexed designators cannot be called")
            }
            let args = parse_call_args(args_text)?;
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

fn parse_call_args(args_text: &str) -> Result<Vec<Expr>> {
    let trimmed = args_text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let pair = Oberon0Parser::parse(Rule::arg_list, trimmed)
        .context("Invalid call arguments")?
        .next()
        .context("Missing argument list")?;
    parse_arg_list(pair)
}

fn split_call_text(raw: &str) -> Result<(&str, &str)> {
    let open = raw
        .find('(')
        .context("Missing opening parenthesis in call")?;
    let close = raw
        .rfind(')')
        .context("Missing closing parenthesis in call")?;

    if close < open {
        bail!("Malformed call syntax")
    }

    Ok((raw[..open].trim(), raw[open + 1..close].trim()))
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
        next = inner.next().context("Missing term after unary sign")?;
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
        Rule::number => parse_number(inner),
        Rule::boolean_literal => {
            let value = inner.as_str() == "TRUE";
            Ok(Expr::Boolean(value))
        }
        Rule::string => Ok(Expr::String(parse_pascal_string(inner.as_str())?)),
        Rule::call_or_var => {
            let raw = inner.as_str().trim();
            if raw.contains('(') {
                let (designator_text, args_text) = split_call_text(raw)?;
                let designator_pair = Oberon0Parser::parse(Rule::designator, designator_text)
                    .context("Invalid designator in call")?
                    .next()
                    .context("Missing designator")?;
                let (module, name, indexes) = parse_designator(designator_pair)?;
                if !indexes.is_empty() {
                    bail!("Indexed designators cannot be called")
                }

                let args = parse_call_args(args_text)?;
                Ok(Expr::Call { module, name, args })
            } else {
                let designator_pair = Oberon0Parser::parse(Rule::designator, raw)
                    .context("Invalid designator")?
                    .next()
                    .context("Missing designator")?;
                let (module, name, indexes) = parse_designator(designator_pair)?;

                if indexes.is_empty() {
                    match module {
                        Some(mod_name) => Ok(Expr::QualifiedVariable {
                            module: mod_name,
                            name,
                        }),
                        None => Ok(Expr::Variable(name)),
                    }
                } else if indexes.len() == 1 {
                    if module.is_some() {
                        bail!("Indexed qualified designators are not yet supported")
                    }
                    Ok(Expr::Indexed {
                        name,
                        index: Box::new(indexes.into_iter().next().unwrap()),
                    })
                } else {
                    bail!("Multiple index selectors are not yet supported")
                }
            }
        }
        Rule::expr => parse_expr(inner),
        _ => bail!("Unknown primary factor: {:?}", inner.as_rule()),
    }
}

fn parse_designator(pair: Pair<Rule>) -> Result<(Option<String>, String, Vec<Expr>)> {
    let mut inner = pair.into_inner();
    let mut base_pair = inner.next().context("Missing qualified identifier")?;
    while base_pair.as_rule() != Rule::qualified_ident && base_pair.as_rule() != Rule::ident {
        base_pair = base_pair
            .into_inner()
            .next()
            .context("Missing qualified identifier")?;
    }
    let (module, name) = parse_qualified_ident(base_pair)?;

    let mut indexes = Vec::new();
    for selector in inner {
        if selector.as_rule() != Rule::index_selector {
            continue;
        }
        let index_expr = selector
            .into_inner()
            .next()
            .context("Missing array index expression")?;
        indexes.push(parse_expr(index_expr)?);
    }

    Ok((module, name, indexes))
}

fn parse_assignment_target(pair: Pair<Rule>) -> Result<AssignTarget> {
    let (module, name, indexes) = parse_designator(pair)?;

    if module.is_some() {
        bail!("Qualified assignment targets are not yet supported")
    }

    match indexes.len() {
        0 => Ok(AssignTarget::Name(name)),
        1 => Ok(AssignTarget::Indexed {
            name,
            index: indexes.into_iter().next().unwrap(),
        }),
        _ => bail!("Multiple index selectors are not yet supported"),
    }
}

fn parse_number(number: Pair<Rule>) -> Result<Expr> {
    let inner = number
        .into_inner()
        .next()
        .context("Empty numeric literal")?;

    match inner.as_rule() {
        Rule::integer => {
            let value = inner
                .as_str()
                .parse::<i64>()
                .with_context(|| format!("Invalid integer: {}", inner.as_str()))?;
            Ok(Expr::Integer(value))
        }
        Rule::real => parse_real_literal(inner.as_str()),
        _ => bail!("Unknown numeric literal: {:?}", inner.as_rule()),
    }
}

fn parse_real_literal(raw: &str) -> Result<Expr> {
    let scale_pos = raw.find(['E', 'D']).unwrap_or(raw.len());
    let base_text = &raw[..scale_pos];
    let scale_text = if scale_pos < raw.len() {
        &raw[scale_pos + 1..]
    } else {
        ""
    };

    let dot_pos = base_text
        .find('.')
        .context("Real literal is missing decimal point")?;
    let whole_text = &base_text[..dot_pos];
    let fractional_text = &base_text[dot_pos + 1..];

    let mut value = whole_text
        .parse::<f64>()
        .with_context(|| format!("Invalid real literal mantissa: {}", whole_text))?;

    if !fractional_text.is_empty() {
        let fraction = fractional_text
            .parse::<f64>()
            .with_context(|| format!("Invalid real literal fraction: {}", fractional_text))?;
        value += fraction / 10f64.powi(fractional_text.len().try_into().unwrap_or(0));
    }

    let exponent = if scale_text.is_empty() {
        0
    } else {
        let mut chars = scale_text.chars();
        let sign = match chars.next() {
            Some('+') => 1,
            Some('-') => -1,
            Some(other) => {
                let mut digits = String::new();
                digits.push(other);
                digits.extend(chars);
                return parse_real_literal_with_exponent(
                    &digits,
                    value,
                    raw[scale_pos..].starts_with('D') || raw[scale_pos..].starts_with('d'),
                );
            }
            None => 1,
        };

        let digits = chars.collect::<String>();
        let parsed = digits
            .parse::<i32>()
            .with_context(|| format!("Invalid real literal exponent: {}", scale_text))?;
        parsed * sign
    };

    let scaled_value = if exponent == 0 {
        value
    } else {
        value * 10f64.powi(exponent)
    };

    let is_long_real = raw.contains('D') || raw.contains('d');

    if is_long_real {
        Ok(Expr::LongReal(scaled_value))
    } else {
        Ok(Expr::Real(scaled_value as f32))
    }
}

fn parse_real_literal_with_exponent(
    exponent_text: &str,
    base_value: f64,
    is_long_real: bool,
) -> Result<Expr> {
    let exponent = exponent_text
        .parse::<i32>()
        .with_context(|| format!("Invalid real literal exponent: {}", exponent_text))?;

    let scaled_value = base_value * 10f64.powi(exponent);

    if is_long_real {
        Ok(Expr::LongReal(scaled_value))
    } else {
        Ok(Expr::Real(scaled_value as f32))
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
    let raw = pair.as_str().trim();
    let mut parts = raw.splitn(2, '.');
    let first = parts
        .next()
        .context("first part of identifier is not an identifier")?;

    if first.is_empty() {
        bail!("first part of identifier is not an identifier")
    }

    match parts.next() {
        Some(second) if !second.is_empty() => {
            Ok((Some(first.to_string()), second.trim().to_string()))
        }
        Some(_) => bail!("second part of identifier is not an identifier"),
        None => Ok((None, first.to_string())),
    }
}

fn parse_type_ref(pair: Pair<Rule>) -> Result<TypeRef> {
    let qualified = pair
        .into_inner()
        .next()
        .context("Missing qualified_ident in type_ref")?;
    match qualified.as_rule() {
        Rule::array_type => parse_array_type(qualified),
        Rule::qualified_ident => {
            let (module, name) = parse_qualified_ident(qualified)?;

            match name.as_str() {
                "INTEGER" if module.is_none() => Ok(TypeRef::Integer),
                "BOOLEAN" if module.is_none() => Ok(TypeRef::Boolean),
                "REAL" if module.is_none() => Ok(TypeRef::Real),
                "LONGREAL" if module.is_none() => Ok(TypeRef::LongReal),
                _ => match module {
                    Some(mod_name) => Ok(TypeRef::Qualified {
                        module: mod_name,
                        name,
                    }),
                    None => Ok(TypeRef::Named(name)),
                },
            }
        }
        _ => bail!("Unknown type reference: {:?}", qualified.as_rule()),
    }
}

fn parse_array_type(pair: Pair<Rule>) -> Result<TypeRef> {
    let mut inner = pair.into_inner();
    let length_expr = inner.next().context("Missing ARRAY length")?;
    let length = parse_expr(length_expr)?;
    let element_type = inner.next().context("Missing ARRAY element type")?;

    Ok(TypeRef::Array {
        length,
        element_type: Box::new(parse_type_ref(element_type)?),
    })
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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
