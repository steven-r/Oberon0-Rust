# Milestone A Backlog

This backlog translates the compiler roadmap into executable tasks.

## A1. Language freeze and corpus

Status: Done in this change set.

Tasks:

1. Maintain and evolve [oberon0-v1-subset.md](oberon0-v1-subset.md).
2. Keep parser corpus files under `tests/parser_cases/`.
3. Add a parser test harness that runs all corpus files.

## A2. Semantic model foundation

Status: Done.

Tasks:

1. Add symbol table types with explicit scopes.
2. Introduce symbol kinds: variable, constant, procedure, parameter.
3. Add declaration placeholders in AST for upcoming grammar extension.
4. Add error variants with stable codes (E001, E002, ...).

Definition of done:

1. Duplicate declarations in scope are rejected by semantic pass.
2. Undefined symbol usage is rejected by semantic pass.

## A3. Grammar extension (declarations)

Status: Done.

Tasks:

1. Extend grammar to support `CONST` and `VAR` sections.
2. Parse declaration sections into AST nodes.
3. Preserve backward compatibility with current module-only programs.

Definition of done:

1. Corpus includes positive and negative declaration cases.
2. Parser and semantic passes remain green.

## A4. Control-flow extension

Status: Done.

Tasks:

1. Add `IF ... THEN ... [ELSE ...] END` syntax and AST nodes.
2. Add `WHILE ... DO ... END` syntax and AST nodes.
3. Extend semantic checks for condition expression type expectations.

Definition of done:

1. Parser corpus covers nested control-flow cases.
2. Errors are emitted for malformed and unsupported condition forms.

## A5. Procedure declarations and calls

Status: Done.

Tasks:

1. Add procedure declarations without return values.
2. Add parameter list parsing.
3. Validate arity and symbol resolution for procedure calls.

Definition of done:

1. Valid procedure corpus parses and checks.
2. Arity mismatch yields semantic diagnostics.

## A6. HIR preparation

Status: Done.

Tasks:

1. Introduce a lowered representation layer (HIR).
2. Add explicit resolved identifiers in HIR.
3. Move codegen input from AST to HIR after parity verification.

Definition of done:

1. Existing example output remains unchanged.
2. HIR can fully represent current Milestone A subset.

## Priority order

1. A2
2. A3
3. A4
4. A5
5. A6
