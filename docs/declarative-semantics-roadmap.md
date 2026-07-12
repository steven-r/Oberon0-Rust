# Declarative Semantics Roadmap

This roadmap defines the next implementation steps to align the compiler with the declarative Oberon model (no implicit declarations).

## D1. Semantic rule freeze for declarations

Status: Done.

Tasks:

1. Update the language spec to explicitly state that assignment targets must be declared before use.
2. Define how declarations are validated per scope (module scope, procedure scope, parameter scope).
3. Clarify allowed shadowing behavior and redeclaration errors.

Definition of done:

1. The spec explicitly disallows implicit variable declarations.
2. Scope and shadowing behavior are documented with examples.

## D2. Remove implicit declarations from semantic analysis

Status: Done.

Tasks:

1. Replace auto-declare behavior in assignment analysis with strict symbol resolution.
2. Emit `UndefinedSymbol` when assignment target is not declared.
3. Keep existing duplicate declaration checks unchanged.

Definition of done:

1. Any assignment to an undeclared target fails semantic analysis.
2. Existing valid corpus files with explicit declarations still pass.

## D3. Keep lowering behavior in sync with semantics

Status: Done.

Tasks:

1. Remove fallback variable declaration from lowering.
2. Require all assignment targets to be pre-resolved symbols.
3. Keep lowering errors as internal consistency checks, not user-facing language decisions.

Definition of done:

1. Lowering does not create new variables implicitly.
2. Lowering succeeds for all semantically valid inputs and fails for broken invariants only.

## D4. Corpus and diagnostics update

Status: Done.

Tasks:

1. Add semantic invalid cases for assignment to undeclared identifiers.
2. Review and adapt existing semantic corpus cases that relied on implicit declarations.
3. Ensure diagnostics remain stable and explicit for declaration-related failures.

Definition of done:

1. Semantic invalid corpus includes undeclared assignment-target coverage.
2. `cargo test` remains green with updated expectations.

## D5. Frontend consistency hardening

Status: Done.

Tasks:

1. Decide whether scanner output remains part of the compilation pipeline or is parser-only test infrastructure.
2. Ensure token-level behavior does not diverge from parser grammar for declaration-related constructs.
3. Add regression tests for scanner/parser consistency.

Definition of done:

1. No duplicate frontend logic can produce conflicting acceptance behavior.
2. Regression tests guard against scanner/parser drift.

## D6. Unified symbol-resolution architecture

Status: Done.

Tasks:

1. Introduce a shared symbol-resolution component or semantic result contract reused by lowering.
2. Remove duplicated declaration/resolve logic where possible.
3. Keep semantic errors user-facing and lowering errors internal.

Definition of done:

1. Name resolution rules are implemented once and reused across phases.
2. Future language extensions require changes in one resolution path only.

## Priority order

1. D1
2. D2
3. D3
4. D4
5. D5
6. D6
