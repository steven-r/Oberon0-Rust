# Operator and Type Compatibility Matrix

This document defines the current semantic compatibility rules between operand types and operators in the Oberon0 subset implemented in this repository.

## Type set in scope

The matrix uses the currently supported scalar types:

- `INTEGER`
- `REAL`
- `LONGREAL`
- `BOOLEAN`

Notes:

- String literals are not part of the general expression type system; they are only valid as `WriteString` arguments.
- The matrix reflects current compiler behavior (semantic analysis), not full Oberon language completeness.

## Legend

- `Allowed`: the operator usage is accepted by semantic analysis.
- `Not allowed`: semantic analysis reports a type mismatch.
- `Result`: inferred expression result type when usage is allowed.

## Unary operators

| Operator | Operand type | Allowed | Result |
| --- | --- | --- | --- |
| unary `+` | `INTEGER` | Allowed | `INTEGER` |
| unary `+` | `REAL` | Allowed | `REAL` |
| unary `+` | `LONGREAL` | Allowed | `LONGREAL` |
| unary `+` | `BOOLEAN` | Not allowed | n/a |
| unary `-` | `INTEGER` | Allowed | `INTEGER` |
| unary `-` | `REAL` | Allowed | `REAL` |
| unary `-` | `LONGREAL` | Allowed | `LONGREAL` |
| unary `-` | `BOOLEAN` | Not allowed | n/a |
| unary `~` | `BOOLEAN` | Allowed | `BOOLEAN` |
| unary `~` | `INTEGER` | Not allowed | n/a |
| unary `~` | `REAL` | Not allowed | n/a |
| unary `~` | `LONGREAL` | Not allowed | n/a |

## Binary arithmetic operators (`+`, `-`, `*`, `/`)

Rules:

- Allowed only for numeric operands (`INTEGER`, `REAL`, `LONGREAL`).
- Result type promotion:
  - if either operand is `LONGREAL`, result is `LONGREAL`
  - else if either operand is `REAL`, result is `REAL`
  - else result is `INTEGER`

| LHS \\ RHS | `INTEGER` | `REAL` | `LONGREAL` | `BOOLEAN` |
| --- | --- | --- | --- | --- |
| `INTEGER` | Allowed -> `INTEGER` | Allowed -> `REAL` | Allowed -> `LONGREAL` | Not allowed |
| `REAL` | Allowed -> `REAL` | Allowed -> `REAL` | Allowed -> `LONGREAL` | Not allowed |
| `LONGREAL` | Allowed -> `LONGREAL` | Allowed -> `LONGREAL` | Allowed -> `LONGREAL` | Not allowed |
| `BOOLEAN` | Not allowed | Not allowed | Not allowed | Not allowed |

## Integer-only arithmetic operators (`DIV`, `MOD`)

Rules:

- Allowed only for `INTEGER` on both sides.
- Result type is `INTEGER`.

| LHS \\ RHS | `INTEGER` | `REAL` | `LONGREAL` | `BOOLEAN` |
| --- | --- | --- | --- | --- |
| `INTEGER` | Allowed -> `INTEGER` | Not allowed | Not allowed | Not allowed |
| `REAL` | Not allowed | Not allowed | Not allowed | Not allowed |
| `LONGREAL` | Not allowed | Not allowed | Not allowed | Not allowed |
| `BOOLEAN` | Not allowed | Not allowed | Not allowed | Not allowed |

## Boolean binary operators (`OR`, `&`)

Rules:

- Allowed only for `BOOLEAN` on both sides.
- Result type is `BOOLEAN`.

| LHS \\ RHS | `INTEGER` | `REAL` | `LONGREAL` | `BOOLEAN` |
| --- | --- | --- | --- | --- |
| `INTEGER` | Not allowed | Not allowed | Not allowed | Not allowed |
| `REAL` | Not allowed | Not allowed | Not allowed | Not allowed |
| `LONGREAL` | Not allowed | Not allowed | Not allowed | Not allowed |
| `BOOLEAN` | Not allowed | Not allowed | Not allowed | Allowed -> `BOOLEAN` |

## Equality operators (`=`, `#`)

Rules:

- Allowed when both operands are numeric (mixing numeric types is allowed).
- Allowed when both operands are `BOOLEAN`.
- Result type is `BOOLEAN`.

| LHS \\ RHS | `INTEGER` | `REAL` | `LONGREAL` | `BOOLEAN` |
| --- | --- | --- | --- | --- |
| `INTEGER` | Allowed -> `BOOLEAN` | Allowed -> `BOOLEAN` | Allowed -> `BOOLEAN` | Not allowed |
| `REAL` | Allowed -> `BOOLEAN` | Allowed -> `BOOLEAN` | Allowed -> `BOOLEAN` | Not allowed |
| `LONGREAL` | Allowed -> `BOOLEAN` | Allowed -> `BOOLEAN` | Allowed -> `BOOLEAN` | Not allowed |
| `BOOLEAN` | Not allowed | Not allowed | Not allowed | Allowed -> `BOOLEAN` |

## Ordering relational operators (`<`, `<=`, `>`, `>=`)

Rules:

- Allowed only for numeric operands (`INTEGER`, `REAL`, `LONGREAL`).
- Result type is `BOOLEAN`.

| LHS \\ RHS | `INTEGER` | `REAL` | `LONGREAL` | `BOOLEAN` |
| --- | --- | --- | --- | --- |
| `INTEGER` | Allowed -> `BOOLEAN` | Allowed -> `BOOLEAN` | Allowed -> `BOOLEAN` | Not allowed |
| `REAL` | Allowed -> `BOOLEAN` | Allowed -> `BOOLEAN` | Allowed -> `BOOLEAN` | Not allowed |
| `LONGREAL` | Allowed -> `BOOLEAN` | Allowed -> `BOOLEAN` | Allowed -> `BOOLEAN` | Not allowed |
| `BOOLEAN` | Not allowed | Not allowed | Not allowed | Not allowed |

## Implementation references

- Grammar: `src/oberon0.pest`
- Expression parsing: `src/parser.rs`
- Type checks: `src/semantic.rs`
- Lowered expression model: `src/ast.rs`, `src/hir.rs`
