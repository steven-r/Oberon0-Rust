# Milestone B Issue Backlog

This backlog translates the Milestone B roadmap into issue-sized work items.

These entries are written so they can be copied into GitHub issues with minimal editing.

## Issue 1. Establish Milestone B planning baseline

Suggested title:

`docs: define Milestone B roadmap for IO and types`

Scope:

1. Add the Milestone B roadmap document.
2. Update README and subset docs to reference the next implementation phase.
3. Preserve Milestone A docs as completed historical records.

Acceptance criteria:

1. Repository docs point to the current roadmap.
2. Milestone B scope is explicit about strings, IO, and typed architecture.

## Issue 2. Add string literal scanning and parsing

Suggested title:

`feat(parser): support double-quoted string literals`

Scope:

1. Extend the scanner token set with string literals.
2. Extend the Pest grammar to parse string literal expressions.
3. Add parser corpus coverage for valid and invalid string forms.

Acceptance criteria:

1. Double-quoted string literals parse successfully.
2. Invalid escape forms fail with deterministic diagnostics.

Dependencies:

1. Issue 1.

## Issue 3. Add `WriteString` builtin without a general string type

Suggested title:

`feat(semantic): add WriteString builtin for string literals`

Scope:

1. Extend AST and HIR with string literal support.
2. Register `WriteString` as a builtin.
3. Allow string literals only where the builtin contract permits them.
4. Extend Rust codegen to emit escaped Rust string literals.

Acceptance criteria:

1. `WriteString("Hello")` works end to end.
2. String literals are not yet allowed as general variable or parameter types.
3. Tests cover Pascal-style embedded quote escaping.

Dependencies:

1. Issue 2.

## Issue 4. Define and implement the first builtin IO surface

Suggested title:

`feat(io): add initial builtin read/write operations`

Scope:

1. Decide the builtin surface for `WriteLn`, `ReadInt`, and `EOF`.
2. Extend semantic handling for builtin procedure and function arity.
3. Extend code generation to map each builtin to stable Rust behavior.
4. Add end-to-end example coverage.

Acceptance criteria:

1. Output and input examples run through the compiler pipeline.
2. Unsupported builtin usage produces clear semantic errors.

Dependencies:

1. Issue 1.
2. Issue 3 if `WriteString` is included in the same builtin group.

## Issue 5. Add `TYPE` declarations and typed variable declarations

Suggested title:

`feat(types): parse and analyze TYPE declarations`

Scope:

1. Extend grammar and AST for `TYPE` declarations.
2. Extend `VAR` declarations with type annotations.
3. Introduce semantic representations for scalar and named types.
4. Add corpus coverage for valid and invalid typed declarations.

Acceptance criteria:

1. Typed declarations parse and pass semantic analysis.
2. Duplicate type names and unknown type references are rejected.

Dependencies:

1. Issue 1.

## Issue 6. Carry type information through symbols and HIR

Suggested title:

`refactor(hir): introduce explicit type information`

Scope:

1. Extend symbol entries with resolved type data.
2. Extend HIR nodes with declared or inferred type information where needed.
3. Keep codegen driven from HIR rather than AST re-analysis.

Acceptance criteria:

1. Typed declarations survive lowering unchanged in meaning.
2. Future array and record work can build on the same type model.

Dependencies:

1. Issue 5.

## Issue 7. Add array type declarations

Suggested title:

`feat(types): support array type declarations`

Scope:

1. Add array type syntax.
2. Extend semantic analysis for array element type and bounds representation.
3. Add valid and invalid corpus coverage.

Acceptance criteria:

1. Array types can be declared and referenced by variable declarations.
2. Malformed array types fail semantic analysis.

Dependencies:

1. Issue 6.

## Issue 8. Add indexed designators for expressions and assignments

Suggested title:

`feat(expr): support array indexing designators`

Scope:

1. Extend grammar for indexed designators.
2. Distinguish assignable designators from simple identifiers.
3. Extend semantic validation for indexed access.
4. Extend HIR and codegen accordingly.

Acceptance criteria:

1. Array reads and writes work end to end.
2. Non-array indexing and invalid index expressions are rejected.

Dependencies:

1. Issue 7.

## Issue 9. Add record type declarations

Suggested title:

`feat(types): support record type declarations`

Scope:

1. Add record syntax and field declarations.
2. Validate duplicate fields and unknown referenced field types.
3. Extend type representations accordingly.

Acceptance criteria:

1. Record declarations parse and analyze successfully.
2. Duplicate fields are rejected deterministically.

Dependencies:

1. Issue 6.

## Issue 10. Add field-selection designators

Suggested title:

`feat(expr): support record field access`

Scope:

1. Extend grammar for field selection.
2. Resolve fields against declared record types.
3. Extend HIR and codegen to preserve field access explicitly.

Acceptance criteria:

1. Record field reads and writes work end to end.
2. Unknown fields and non-record field access fail semantic analysis.

Dependencies:

1. Issue 9.

## Issue 11. Consolidate type checking and assignment rules

Suggested title:

`refactor(semantic): centralize type checking for assignments and designators`

Scope:

1. Centralize assignment compatibility checks.
2. Remove one-off type validation branches that would otherwise accumulate.
3. Stabilize type mismatch diagnostics.

Acceptance criteria:

1. Assignment and designator rules live in one coherent semantic path.
2. Arrays and records do not require duplicated validation logic.

Dependencies:

1. Issue 6.
2. Issue 8.
3. Issue 10.

## Suggested milestone slices

Slice 1:

1. Issue 1
2. Issue 2
3. Issue 3

Slice 2:

1. Issue 4
2. Issue 5
3. Issue 6

Slice 3:

1. Issue 7
2. Issue 8

Slice 4:

1. Issue 9
2. Issue 10
3. Issue 11
