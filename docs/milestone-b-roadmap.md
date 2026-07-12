# Milestone B Roadmap

This roadmap defines the next implementation phase after the completed Milestone A subset.

Milestone B has two goals:

1. Extend the language in ways that open a real path toward Oberon-style data modeling.
2. Keep the compiler architecture coherent while moving from untyped bindings toward typed symbols and typed HIR.

## Design constraints

The next phase should avoid ad hoc feature growth.

Required architectural rules:

1. Type-directed features must not be implemented purely as parser sugar.
2. Semantic analysis must evolve toward explicit type checking instead of name-resolution-only validation.
3. HIR must be able to represent typed declarations, typed expressions, and non-trivial l-values.
4. Rust code generation must remain a consumer of HIR, not re-derive language semantics from AST shapes.

## B1. Roadmap reset and target freeze

Status: Planned.

Tasks:

1. Keep Milestone A docs as historical baseline.
2. Define Milestone B acceptance boundaries for strings, IO, and types.
3. Decide naming and semantics for the initial builtin IO surface.
4. Document the typed architecture direction before implementation begins.

Definition of done:

1. Milestone B scope is documented in repository docs.
2. The implementation order is explicit.
3. GitHub issues can be opened directly from the documented backlog.

## B2. String literals and `WriteString`

Status: Planned.

Tasks:

1. Extend the scanner and grammar to support double-quoted string literals.
2. Use Pascal-style escaping rules for embedded quotes.
3. Extend AST and HIR to carry string literal nodes.
4. Add builtin handling for `WriteString` without introducing a general string type.
5. Extend Rust code generation to emit valid escaped Rust string literals.

Definition of done:

1. String literals parse and lower successfully.
2. `WriteString("...")` works end to end.
3. Corpus tests cover valid and invalid escape cases.
4. No general-purpose string variable type exists yet.

## B3. Minimal IO builtin surface

Status: Designed (implementation split pending).

Tasks:

1. Define the first builtin IO procedures and functions to support in the MVP.
2. Add semantic handling for `WriteLn`, `ReadInt`, and `EOF` or equivalent names.
3. Decide whether these are treated as predefined builtins or manifest-backed imports.
4. Extend code generation to map them to stable Rust runtime behavior.

Definition of done:

1. The builtin IO surface is documented and tested.
2. The semantic pass validates arity and builtin call legality.
3. End-to-end examples demonstrate text output and integer input.

Decision document:

1. docs/io-builtin-contract.md

## B4. Typed declaration model

Status: Planned.

Tasks:

1. Add `TYPE` declarations to the grammar.
2. Extend `VAR` declarations to carry a declared type.
3. Introduce semantic representations for built-in scalar types and named user types.
4. Add type information to symbols and HIR.

Definition of done:

1. Typed variable declarations parse and analyze successfully.
2. The compiler distinguishes symbol kind from symbol type.
3. HIR preserves declared type information for code generation.

## B5. Array types and indexing

Status: Planned.

Tasks:

1. Add array type syntax.
2. Add indexed designators in expressions and assignments.
3. Introduce array type checking and index validation rules.
4. Extend code generation to emit Rust array or vector-backed storage.

Definition of done:

1. Arrays can be declared, assigned, and indexed.
2. Invalid indexing produces semantic diagnostics.
3. HIR models indexed l-values explicitly.

## B6. Record types and field access

Status: Planned.

Tasks:

1. Add record type syntax and field declarations.
2. Add field-selection designators.
3. Extend semantic analysis for field lookup and duplicate-field validation.
4. Extend code generation to emit Rust structs and field access.

Definition of done:

1. Records can be declared and instantiated through variable storage.
2. Field access is validated semantically.
3. Generated Rust preserves field layout and names clearly.

## B7. Type checker consolidation

Status: Planned.

Tasks:

1. Introduce an explicit type model shared across semantic analysis, lowering, and code generation.
2. Define assignment compatibility rules.
3. Distinguish expression evaluation from assignable designators.
4. Ensure future feature additions change one type-checking path instead of several disconnected paths.

Definition of done:

1. Type checking is centralized.
2. Arrays and records do not rely on special-case validation outside the type model.
3. Diagnostics are stable for type mismatches and invalid designators.

## Suggested implementation order

1. B1
2. B2
3. B3
4. B4
5. B7
6. B5
7. B6
