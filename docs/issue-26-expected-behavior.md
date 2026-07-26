# Issue #26 Expected Behavior: Qualified Names and Exports

This document captures the intended behavior before full implementation.
It is written to drive parser/semantic tests.

## Scope

1. Qualified identifiers in expressions and calls.
2. Export markers on type/procedure declarations.
3. Visibility checks for cross-module references.

## Syntax Acceptance

The parser should accept the following forms:

```oberon
TYPE T* = INTEGER;
PROCEDURE HELLO*;
B.HELLO;
VAR x: B.T;
```

The parser should reject malformed qualified designators, for example:

```oberon
B.
```

## Semantic Rules (Target)

1. `IMPORT B := ModuleB;` introduces alias `B`.
2. `B.Name` resolves within the imported module namespace.
3. Only declarations marked with `*` are visible through qualified access.
4. Accessing `B.Name` when `Name` is not exported should fail.
5. Accessing `B.Name` when `B` is unknown should fail.
6. Qualified type references (`VAR x: B.T;`) must resolve and type-check.

## Diagnostic Intent

1. Missing member in qualified syntax remains a parser error.
2. Unknown import alias/member remains a semantic error.
3. Non-exported member access should produce a dedicated semantic error (to be introduced).

## Test Plan

### Added in this changeset

1. Parser unit tests in `src/parser.rs`:

   - export marker parsing
   - qualified call parsing
   - qualified variable-expression parsing
   - qualified type-ref parsing

1. Corpus tests:

   - `tests/parser_cases/valid/qualified_exports.ob0`
   - `tests/parser_cases/invalid/qualified_member_missing_name.ob0`
   - `tests/semantic_cases/invalid/qualified_call_member_unresolved.ob0`
   - `tests/semantic_cases/invalid/qualified_type_reference_unsupported.ob0`

1. Semantic unit tests in `src/semantic.rs`:

   - current failure behavior for qualified member/type references
   - ignored target tests for eventual Issue #26 completion

### Deferred until implementation

1. Semantic pass cases where `B.HELLO` succeeds for exported members.
1. Semantic pass cases where `B.Hidden` fails due to missing export marker.
1. Cross-module fixture tests that load/export symbols from multiple Oberon modules.
