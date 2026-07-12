# Changelog

<!-- markdownlint-configure-file {"MD024": {"siblings_only": true}} -->

All notable changes to this project will be documented in this file.

## Unreleased

## v0.7.0 - 2026-07-12

### Features

- add `TYPE` declarations and typed `VAR` declarations for `INTEGER` and simple named aliases (#5)
- preserve declared type information through semantic symbols and HIR for the first typed-declaration slice (#6)
- add built-in scalar declaration support for `BOOLEAN`, `REAL`, and `LONGREAL` alongside `INTEGER` (#17)
- keep built-in scalar names reserved while allowing procedure parameters to shadow user-defined module-scope type names, except in declarations like `Count: Count` (no dedicated issue)
- add optional procedure-local `VAR` sections before `BEGIN` and carry those local bindings through semantic analysis and lowering (no dedicated issue)

### Documentation

- align contributor guidance across repository docs with the project decision log (no dedicated issue)
- document built-in scalar declaration support in the README, roadmap, and typed-declarations example (#17)

### Tests

- add semantic corpus and lowering coverage for typed declarations and preserved type information in HIR (#5, #6)
- add semantic coverage for `BOOLEAN`, `REAL`, and `LONGREAL` declaration support (#17)
- add semantic coverage for user-defined type-name shadowing by parameters while rejecting built-in shadowing and `Count: Count` self-shadowing declarations (no dedicated issue)
- add semantic and lowering coverage for procedure-local `VAR` declarations, including local shadowing constraints for built-in and user-defined type names (no dedicated issue)

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
