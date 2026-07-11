# Oberon0 v1 Subset Specification

This document defines the implementation target for compiler Milestone A.

## Goals

1. Freeze a stable, testable language subset.
2. Prevent parser and semantic scope creep.
3. Establish acceptance criteria for each implemented feature.

## Module form

A source unit must follow this shape:

```oberon
MODULE <Ident>;
[IMPORT ... ;]
BEGIN
  [statement-list]
END <Ident>.
```

Rules:

1. The `END` identifier must match the `MODULE` identifier.
2. `IMPORT` is optional.
3. Empty statement list is allowed.

## Imports

Supported forms:

1. `IMPORT Math;`
2. `IMPORT IO, Math;`
3. `IMPORT Local := External;`

Semantics in Milestone A:

1. Duplicate local aliases are rejected.
2. If a manifest is provided, each imported external name must be mapped.

## Statements

Supported in Milestone A:

1. Assignment: `x := expr`
2. Procedure-style call: `Proc` and `Proc(arg1, arg2)`

Not yet in Milestone A:

1. `IF ... THEN ... [ELSE ...] END`
2. `WHILE ... DO ... END`
3. Declarations (`CONST`, `VAR`, `TYPE`)
4. User-defined procedure declarations

## Expressions

Grammar supports:

1. Integer literals
2. Identifiers
3. Parenthesized expressions
4. Binary operators: `+`, `-`, `*`, `/`

Operator precedence:

1. `*` and `/` bind stronger than `+` and `-`.
2. Parsing is left-associative for operators of equal precedence.

## Error behavior requirements

The compiler must produce user-facing errors for:

1. Invalid module structure
2. Unknown tokens
3. Module name mismatch (`MODULE` name vs `END` name)
4. Duplicate import aliases
5. Missing import-to-crate mapping when manifest validation is enabled

## Test acceptance criteria

Milestone A is complete when all criteria below are true:

1. Valid corpus files parse successfully.
2. Invalid corpus files fail parsing.
3. Semantic invalid corpus files fail semantic analysis.
4. End-to-end example still generates Rust and builds.

Corpus layout:

1. `tests/parser_cases/valid` and `tests/parser_cases/invalid` for grammar-only checks.
2. `tests/semantic_cases/valid` and `tests/semantic_cases/invalid` for analysis checks after parsing.
