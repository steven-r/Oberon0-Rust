use anyhow::{Context, Result, bail};
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

use crate::ast::{BinaryOp, Expr, ImportDecl, Module, Statement};

#[derive(Parser)]
#[grammar = "oberon0.pest"]
struct Oberon0Parser;

pub fn parse_module(source: &str) -> Result<Module> {
    let mut pairs = Oberon0Parser::parse(Rule::module, source).context("Invalid Oberon0 syntax")?;
    let module_pair = pairs.next().context("No module found")?;
    build_module(module_pair)
}

fn build_module(module_pair: Pair<Rule>) -> Result<Module> {
    let mut inner = module_pair.into_inner();

    let name = take_ident(inner.next(), "module name")?;

    let maybe_next = inner.next().context("Unexpected end after module name")?;

    let (imports, begin_pair) = if maybe_next.as_rule() == Rule::import_section {
        let imports = parse_import_section(maybe_next)?;
        let begin_pair = inner
            .next()
            .context("Unexpected end before BEGIN block")?;
        (imports, begin_pair)
    } else {
        (Vec::new(), maybe_next)
    };

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
        declarations: Vec::new(),
        statements,
    })
}

fn parse_import_section(section: Pair<Rule>) -> Result<Vec<ImportDecl>> {
    section
        .into_inner()
        .map(parse_import_item)
        .collect::<Result<Vec<_>>>()
}

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

fn parse_stmt_list(list: Pair<Rule>) -> Result<Vec<Statement>> {
    list.into_inner()
        .map(parse_statement)
        .collect::<Result<Vec<_>>>()
}

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
            let name = take_ident(parts.next(), "procedure name")?;
            let args = match parts.next() {
                Some(arg_list) => parse_arg_list(arg_list)?,
                None => Vec::new(),
            };
            Ok(Statement::Call { name, args })
        }
        Rule::statement => {
            let inner = stmt.into_inner().next().context("Empty statement")?;
            parse_statement(inner)
        }
        _ => bail!("Unknown statement: {:?}", stmt.as_rule()),
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
    let mut left = parse_term(inner.next().context("Empty expression")?)?;

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
        Rule::integer => {
            let value = inner
                .as_str()
                .parse::<i64>()
                .with_context(|| format!("Invalid integer: {}", inner.as_str()))?;
            Ok(Expr::Integer(value))
        }
        Rule::ident => Ok(Expr::Variable(inner.as_str().to_string())),
        Rule::expr => parse_expr(inner),
        _ => bail!("Unknown factor: {:?}", inner.as_rule()),
    }
}

fn parse_add_op(op: Pair<Rule>) -> Result<BinaryOp> {
    match op.as_str() {
        "+" => Ok(BinaryOp::Add),
        "-" => Ok(BinaryOp::Sub),
        other => bail!("Unknown add operator: {}", other),
    }
}

fn parse_mul_op(op: Pair<Rule>) -> Result<BinaryOp> {
    match op.as_str() {
        "*" => Ok(BinaryOp::Mul),
        "/" => Ok(BinaryOp::Div),
        other => bail!("Unknown mul operator: {}", other),
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

    #[test]
    fn valid_corpus_parses() {
        for (name, source) in read_dir_sources("tests/parser_cases/valid") {
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
    fn semantic_valid_corpus_passes() {
        for (name, source) in read_dir_sources("tests/semantic_cases/valid") {
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
}
