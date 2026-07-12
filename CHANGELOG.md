# Changelog

<!-- markdownlint-configure-file {"MD024": {"siblings_only": true}} -->

All notable changes to this project will be documented in this file.

## Unreleased

### Features

- enforce declarative symbol resolution for assignment targets in semantic analysis
- enforce lowering invariant: assignment targets must already be resolved symbols

### Documentation

- add declarative semantics roadmap (D1-D6)
- define declaration/scope/shadowing rules in language subset specification

### Tests

- add semantic regression for undeclared assignment target
- add semantic invalid corpus case for undeclared assignment target
- adapt semantic valid corpus to explicit declaration model
- add lowering regression for unresolved assignment target invariant
- assert stable E005 diagnostics for undeclared assignment target
- align scanner keyword tokens with parser grammar for declaration/control-flow keywords

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
