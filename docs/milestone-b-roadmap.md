# Milestone B Roadmap

This roadmap defines the next implementation phase after the completed Milestone A subset.

Reference alignment snapshot:

1. docs/wirth-page63-alignment-matrix.md

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

Status: Completed.

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

Status: Completed.

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

Status: Completed.

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

Status: Completed.

Tasks:

1. Add `TYPE` declarations to the grammar.
2. Extend `VAR` declarations to carry a declared type.
3. Introduce semantic representations for built-in scalar types and named user types.
4. Add type information to symbols and HIR.

Definition of done:

1. Typed variable declarations parse and analyze successfully.
2. The compiler distinguishes symbol kind from symbol type.
3. HIR preserves declared type information for code generation.

Current status:

1. `TYPE` declarations and typed `VAR` declarations are being added first for `INTEGER` and simple named aliases.
2. Symbol entries now carry declared type information for typed declarations.
3. HIR now preserves `TYPE` declarations and typed `VAR` declarations explicitly, without changing current code generation behavior.
4. Built-in scalar declaration support now covers `INTEGER`, `BOOLEAN`, `REAL`, and `LONGREAL`; expression-level typing and runtime representation remain follow-up work.
5. The typed-declaration slice is complete enough to close the foundational issues for this phase (#5, #6, #17).

## B5. Array types and indexing

Status: Completed.

Tasks:

1. Add array type syntax.
2. Add indexed designators in expressions and assignments.
3. Introduce array type checking and index validation rules.
4. Extend code generation to emit Rust array or vector-backed storage.

Definition of done:

1. Arrays can be declared, assigned, and indexed.
2. Invalid indexing produces semantic diagnostics.
3. HIR models indexed l-values explicitly.

Current status:

1. Array type declarations are implemented, including constant-expression lengths folded during semantic analysis.
2. Indexed designators are implemented for both expressions and assignment targets.
3. Lowering and HIR model indexed l-values explicitly.
4. Rust code generation supports indexed read/write execution and state serialization for array values.

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

Current status:

1. The shared type-model foundation is established and tracked by the type-model issue slice (#61).
2. Full consolidation still depends on array and record designators landing on the same compatibility path.

## Suggested implementation order

1. B1
2. B2
3. B3
4. B4
5. B5
6. B6
7. B7

## Prioritized issue order (language-complete target)

This priority list aligns implementation risk, dependency flow, and page-63 language coverage.

Priority 0 (already done or in-progress baseline):

1. #12 (`WriteLn`) and #13 (`ReadInt`/`EOF`) completed the minimum IO baseline.

Priority 1 (type system foundation):

1. #5 (`TYPE` + typed `VAR` declarations)
2. #6 (type information in symbols/HIR)
3. #17 (builtin scalar types: `BOOLEAN`, `REAL`, `LONGREAL`)
4. #61 (shared type model foundation)
5. #22 (typed formal parameters with optional `VAR` mode)

Priority 2 (expression and control-flow completeness):

1. #18 (operators `DIV`, `MOD`, `&`, `OR`, unary `~`, unary sign)
2. #19 (relational operators)
3. #20 (`ELSIF`)
4. #21 (`REPEAT ... UNTIL`)

Priority 3 (structured data model):

1. #9 (record type declarations)
2. #10 (field-selection designators)
3. #11 (type-checking consolidation)

Priority 4 (IO and grammar parity refinements):

1. #23 (`WriteChar`, `OpenInput`)
2. #24 (`WriteInt(x, n)` width parameter)
3. #25 (optional module `BEGIN`)
4. #26 (`ProcedureCall ... "*"` form decision/implementation)
5. Introduce internal builtin modules (`IO`, `MATH`) for qualified builtin calls.

Concept document:

1. docs/internal-builtin-modules-concept.md
