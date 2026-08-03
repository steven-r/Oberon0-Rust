# Changelog

<!-- markdownlint-configure-file {"MD024": {"siblings_only": true}} -->

All notable changes to this project will be documented in this file.

## Unreleased

### Features

- add array type declarations in `TYPE` blocks, including constant-expression lengths that fold during semantic analysis (#7)
- add indexed designators for expressions and assignment targets, including lowering and Rust code generation support (`a[i]`, `a[i] := expr`) (#8)

### Fixes

- propagate `VAR` array parameter mutations back to caller/module state in generated runtime code paths (#8)
- fix the quicksort golden sample loop bounds/initialization so the algorithm executes and produces sorted output

### Documentation

- update README subset notes to describe array type and indexed-designator support (#7, #8)

### Tests

- add parser and semantic valid corpus coverage for array type declarations, constant-expression array lengths, and indexed designators (#7, #8)
- add codegen regression coverage for indexed read/write emission and `VAR` array roundtrip behavior (#8)

## v0.9.1 - 2026-08-02

### Features

- add a shared numeric type model for `INTEGER`, `REAL`, and `LONGREAL` with stricter compatibility rules for assignments and numeric conversions
- add semantic, lowering, and code generation support for built-in real/longreal I/O procedures such as `ReadReal`, `ReadLongReal`, `WriteReal`, and `WriteLongReal`
- extend the parser and generated runtime to handle `LONGREAL` literals and numeric operations with the new runtime value model

### Fixes

- reject invalid numeric assignments such as `REAL` to `INTEGER` while preserving valid mixed numeric compatibility
- keep generated Rust code aligned with the new numeric semantics and built-in I/O behavior

### Documentation

- document the compiler pipeline, the role of the HIR, and why code generation relies on internal analysis helpers
- clarify the import/module-translation workflow and link the new pipeline notes

### CI

- upload generated coverage reports to Codecov from the CI workflow

### Tests

- extend code generation coverage for state-map decision paths and nested assignment tracking in generated code paths
- add golden regression coverage for branching, procedure-local behavior, and real/longreal built-in execution paths

## v0.9.0 - 2026-08-02

### Features

- allow `CONST` declarations to use full expressions (not only raw integer literals), with compile-time folding into literal values
- carry constant expressions as HIR expressions and emit folded constant values in generated Rust
- add semantic/lowering support for built-in conversion functions `FLT()` and `FLOOR()` for integer/real conversions

### Fixes

- add semantic validation for constant initializers and report `E016` when a `CONST` does not fold to a literal expression
- validate types for procedure parameters (types where optional before)
- extend numeric literal parsing so decimal literals with optional `E`/`D` scale factors are accepted as `REAL` or `LONGREAL`

### Documentation

- clarify README language-subset notes for constant expression initializers and literal expression coverage

## v0.8.0 - 2026-07-26

### Features

- allow declaration-only modules to omit the top-level `BEGIN` block while preserving existing module-body parsing (#25)
- add export markers (`*`) on type and procedure declarations to prepare visibility controls for cross-module access (#26)
- parse qualified identifiers (`Module.Name`) in expressions, calls, and type references to prepare for cross-module reference resolution (#26)
- Allow constants to be negative (e.g. `CONST x = -1;`)

### Fixes

- parse zero-argument call expressions like `ReadInt()` and `EOF()` as call nodes (not variable references), restoring expected runtime IO behavior and golden-case outcomes (#26)
- reject qualified variable expressions (for example `B.value`) during semantic analysis with explicit `E015`, avoiding late lowering failures (#26)

### Documentation

- document optional top-level module `BEGIN` blocks in the README and subset/alignment docs (#25)
- document Issue #26 expected behavior for qualified names and export markers in dedicated specification (#26)
- add dedicated module-translation workflow documentation for import/manifest-based projects, including current limits (#26)

### Tests

- add parser and semantic corpus coverage for declaration-only and minimal modules without `BEGIN` (#25)
- add parser unit tests for export markers and qualified expressions with AST verification (#26)
- add parser/semantic corpus coverage for qualified-name syntax validation and current-state error diagnostics (#26)
- add semantic unit tests documenting future Issue #26 expected behavior via ignored target tests (#26)
- add parser regression coverage for zero-argument call-expression parsing (`ReadInt()`, `EOF()`) to prevent lowering/codegen regressions (#26)
- add parser and semantic invalid corpus edge cases for malformed import syntax and qualified-name visibility/alias resolution checks (#26)
- add semantic invalid corpus coverage for unsupported qualified variable expressions with single-fault repair mapping (#26)

### CI

- enforce CI coverage gates with `cargo llvm-cov` total line coverage >90% and Rust changed-line coverage >95% against `main`

## v0.7.0 - 2026-07-12

### Features

- add `TYPE` declarations and typed `VAR` declarations for `INTEGER` and simple named aliases (#5)
- preserve declared type information through semantic symbols and HIR for the first typed-declaration slice (#6)
- add built-in scalar declaration support for `BOOLEAN`, `REAL`, and `LONGREAL` alongside `INTEGER` (#17)
- keep built-in scalar names reserved while allowing procedure parameters to shadow user-defined module-scope type names, except in declarations like `Count: Count` (no dedicated issue)
- add optional procedure-local `VAR` sections before `BEGIN` and carry those local bindings through semantic analysis and lowering (no dedicated issue)
- add Oberon operator support for `DIV`, `MOD`, `OR`, `&`, unary `~`, and unary signs in scanner, parser, semantic analysis, lowering, and code generation (#18)
- add relational operators (`=`, `#`, `<`, `<=`, `>`, `>=`) across grammar, parser, semantic analysis, and code generation (#19)

### Documentation

- align contributor guidance across repository docs with the project decision log (no dedicated issue)
- document built-in scalar declaration support in the README, roadmap, and typed-declarations example (#17)
- document extended operator coverage in the root README and examples index, and add a focused `operators-extended` example (#18)

### Tests

- add semantic corpus and lowering coverage for typed declarations and preserved type information in HIR (#5, #6)
- add semantic coverage for `BOOLEAN`, `REAL`, and `LONGREAL` declaration support (#17)
- add semantic coverage for user-defined type-name shadowing by parameters while rejecting built-in shadowing and `Count: Count` self-shadowing declarations (no dedicated issue)
- add semantic and lowering coverage for procedure-local `VAR` declarations, including local shadowing constraints for built-in and user-defined type names (no dedicated issue)
- add parser and semantic valid/invalid corpus coverage for extended operators, including dedicated single-fault repairs for new invalid cases (#18)
- add parser/semantic/codegen coverage for relational operators, including invalid numeric-operand diagnostics and parser invalid repair cases (#19)

### Build

- update the `toml` crate to v1 for the toolchain and manifest stack (no dedicated issue)

### CI

- migrate Renovate configuration into `.github/renovate.json` and extend scanning to `oberon.toml` files under examples and tests (no dedicated issue)
- switch release automation to a PR-based flow and harden changelog promotion, git identity handling, and release note extraction (no dedicated issue)

## v0.6.0 - 2026-07-12

### Features

- add Pascal-style string literals and `WriteString` builtin support across scanner, parser, semantic analysis, lowering, and code generation
- add explicit state-output controls via `compiler.emit_state` in `oberon.toml` and one-shot CLI overrides (`--emit-state`, `--no-emit-state`)
- extend generated runtime state output to include procedure-scope shadowing bindings under qualified keys (for example `Proc.x`)
- enforce declarative assignment-target resolution in semantic analysis and keep the same invariant in lowering

### Fixes

- preserve module constant values during expression code generation
- generate mutable Rust parameter bindings so reassigned Oberon0 procedure parameters compile correctly

### Documentation

- document explicit state-output controls and current subset limits for procedure-local `VAR` declarations
- add focused examples for manifest-backed imports and procedure-scope shadowing flows
- expand language-planning documentation for declarative semantics and scope behavior

### Tests

- add parser and semantic corpus coverage for valid and invalid string literal cases
- add codegen/runtime regressions for explicit state output control, shadowed bindings, and mutable reassigned procedure parameters
- add end-to-end example coverage for string handling and new procedure-scope shadowing scenarios
- strengthen declarative-scope regressions for undeclared assignment targets and stable `E005` diagnostics

### Chores

- introduce shared scoped map helper reused by semantic symbol table and lowering resolver

## v0.2.0 - 2026-07-11

### Features

- add feature-focused example suite with readmes
- emit procedure bodies from HIR
- track stable local refs across nested flow
- introduce lowering stage and HIR-based codegen
- add declarations, params, and arity checks
- add IF/WHILE parsing, semantics, and codegen
- add CONST/VAR declarations with semantic checks
- add project compile wrapper scripts
- add symbol table foundation and error codes
- add Milestone A spec and parser/semantic corpora

### Fixes

- avoid unnecessary parentheses in generated expressions

### Documentation

- refresh language subset and unreleased notes
- mark A6 as done
- mark A3 as done
- mark A2 as done

### Tests

- add scan coverage and '=' regression
- add unit tests for codegen, cli, manifest, symbols, semantic

### Chores

- configure markdownlint duplicate-heading handling
- enforce markdownlint and format changelog output

## v0.1.1 - 2026-07-11

### Chores

- update logos and toml
- add changelog and semver release tooling
