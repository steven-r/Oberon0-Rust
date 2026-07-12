# Builtin IO Surface and Runtime Contract

This document freezes the initial builtin IO contract for Milestone B.

It resolves issue #4 and provides an implementation target for follow-up issues.

## Design decisions

1. Builtins are predefined symbols, not manifest-backed imports.
2. Builtin names are reserved in module scope.
3. Builtin arity is enforced in semantic analysis.
4. Code generation targets a stable Rust runtime mapping with no re-interpretation in later phases.

## Builtin surface

| Name | Kind | Arity | Arguments | Return | Status |
| --- | --- | --- | --- | --- | --- |
| `WriteInt` | Procedure | 1 | `INTEGER` | none | Implemented |
| `WriteString` | Procedure | 1 | string literal (subset constraint) | none | Implemented |
| `WriteLn` | Procedure | 0 | none | none | Implemented |
| `ReadInt` | Function-like builtin | 0 | none | `INTEGER` | Implemented |
| `EOF` | Function-like builtin | 0 | none | `INTEGER` (`1` = EOF, `0` = not EOF) | Implemented |

Notes:

1. `WriteString` currently accepts string literals only, matching the subset constraints.
2. `ReadInt` and `EOF` are represented as call expressions in source syntax (`ReadInt()`, `EOF()`).

## Semantic registration contract

Semantic analysis must register builtin symbols before imports and user declarations.

Required behavior:

1. Builtin names cannot be redeclared by `CONST`, `VAR`, or `PROCEDURE`.
2. Import aliases cannot reuse builtin names.
3. Arity must be validated against the table above.
4. Argument kind constraints must be validated for each builtin.

## Runtime contract for code generation

Code generation must map builtins to Rust behavior as follows:

1. `WriteInt(x)`
   - Emits integer text without a trailing newline (`print!("{}", x)`).
2. `WriteString(s)`
   - Emits string content without newline (`print!("{}", s)`).
3. `WriteLn()`
   - Emits a single newline (`println!()`).
4. `ReadInt()`
   - Reads one line from `stdin`.
   - Trims trailing whitespace.
   - Parses signed base-10 integer.
   - On parse or read failure: terminates with a clear runtime error message.
5. `EOF()`
   - Returns `1` when end-of-input is reached, otherwise `0`.
   - Must not consume extra user-visible tokens beyond what is needed to detect stream state.

## Follow-up execution split

Implementation tracking status:

1. #12: `WriteLn` behavior (implemented)
2. #13: `ReadInt` and `EOF` behavior (implemented)
3. #3: alignment of `WriteString` with the same builtin model (implemented)

## Out of scope for this decision

1. Typed string variables and general string operations.
2. Advanced stream abstractions beyond stdin/stdout.
3. Localization or formatting controls.
