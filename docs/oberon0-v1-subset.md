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

## Declaration and scope rules

The language model is declarative (Wirth-style): symbols must be declared before they are used.
Implicit declarations are not allowed.

Scope validation in Milestone A:

1. Module scope contains imported aliases, `CONST`, `VAR`, and `PROCEDURE` names.
2. Procedure scope contains procedure parameters and can reference module-scope symbols.
3. Assignment targets and expression identifiers must resolve in the current scope chain.
4. Assigning to an undeclared identifier is a semantic error.

Shadowing and redeclaration:

1. Redeclaration in the same scope is rejected.
2. Shadowing across nested scopes is allowed (for example, a procedure parameter may shadow a module variable).
3. Duplicate parameter names in a procedure declaration are rejected.

Examples:

Valid shadowing:

```oberon
MODULE Main;
VAR x;
PROCEDURE P(x);
BEGIN
  x := x + 1
END P;
BEGIN
  x := 0;
  P(41)
END Main.
```

Invalid undeclared assignment target:

```oberon
MODULE Main;
BEGIN
  y := 1
END Main.
```

Invalid redeclaration in same scope:

```oberon
MODULE Main;
VAR x, x;
BEGIN
END Main.
```

## Statements

Supported in Milestone A:

1. Assignment: `x := expr`
2. Procedure-style call: `Proc` and `Proc(arg1, arg2)`
3. Conditional: `IF expr THEN [statement-list] [ELSE [statement-list]] END`
4. Loop: `WHILE expr DO [statement-list] END`
5. Procedure declaration: `PROCEDURE Name([param1, ...]); BEGIN [statement-list] END Name;`

Not yet in Milestone A:

1. Type declarations (`TYPE`)
2. Return values from procedures and functions

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
6. Procedure END name mismatch
7. Procedure call arity mismatch
8. Assignment to undeclared identifiers
9. Duplicate declarations within the same scope

## Test acceptance criteria

Milestone A is complete when all criteria below are true:

1. Valid corpus files parse successfully.
2. Invalid corpus files fail parsing.
3. Semantic invalid corpus files fail semantic analysis.
4. End-to-end example still generates Rust and builds.

Corpus layout:

1. `tests/parser_cases/valid` and `tests/parser_cases/invalid` for grammar-only checks.
2. `tests/semantic_cases/valid` and `tests/semantic_cases/invalid` for analysis checks after parsing.
