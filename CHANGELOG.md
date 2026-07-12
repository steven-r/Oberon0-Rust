# Changelog

<!-- markdownlint-configure-file {"MD024": {"siblings_only": true}} -->



All notable changes to this project will be documented in this file.

## Unreleased

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
