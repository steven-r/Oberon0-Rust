# Internal Builtin Modules Concept (IO and MATH)

> **Superseded architecture:** Issue #75 replaces the catalog, resolution, HIR, and
> dispatch design in this document with
> `internal-function-catalog-design.md`. The qualified `IO` and `MATH` language
> surface below remains the compatibility contract during that migration.

This document proposes a module-based model for internal builtins, starting with two predefined modules:

1. `IO` for input/output builtins (`Write*`, `Read*`, `EOF`).
2. `MATH` for numeric conversion/math builtins (`FLT`, `FLOOR`).

The goal is to remove global builtin names over time and make builtin lookup explicit and scalable.

## Motivation

Current behavior treats builtins as global predefined procedures/functions.

Pain points:

1. Builtin checks are scattered (`semantic.rs`, `lower.rs`, `codegen.rs`) with repeated string matching.
2. Arity/type rules are encoded ad hoc per call site.
3. There is no single registry that captures return type, argument types, and call context rules.
4. Future expansion (for example `WriteInt(x, n)` width overload) increases duplication risk.

## Target language surface

Target source usage:

1. `IO.WriteInt(x)`
2. `x := IO.ReadInt()`
3. `IF IO.EOF() THEN ... END`
4. `r := MATH.FLT(i)`
5. `i := MATH.FLOOR(r)`

Import requirement:

1. `IO` and `MATH` must be imported before use (`IMPORT IO, MATH;`).
2. Qualified builtin calls without import must fail like regular unresolved module usage.
3. Unqualified builtin usage is not supported.

No unqualified fallback:

1. `Write*`, `Read*`, `EOF`, `FLT`, and `FLOOR` must be called via `IO.*` or `MATH.*`.

## Core design

### 1. Builtin registry as source of truth

Introduce a dedicated builtin catalog shared by semantic analysis, lowering, and code generation.

Suggested model:

1. `BuiltinModule { name, members }`
2. `BuiltinEntry { name, call_context, overloads }`
3. `BuiltinOverload { params, return_type }`
4. `ParamConstraint` for type-compatible argument matching.

Example `ParamConstraint` variants:

1. `Exact(TypeRef)`
2. `OneOf(Vec<TypeRef>)`
3. `StringLiteralOnly`

Call context should be explicit:

1. `StatementOnly` (for pure procedures, if needed)
2. `ExpressionOnly` (for function-like builtins such as `Read*`, `EOF`, `FLT`, `FLOOR`)
3. `StatementOrExpression` (if future builtins require it)

### 2. Module-qualified builtin resolution

When resolving `Expr::Call` and statement calls:

1. If `module` is `Some("IO")` or `Some("MATH")`, resolve against builtin registry first.
2. If `module` is any other alias, keep external-import behavior.
3. If `module` is `None`, follow compatibility mode policy:
   1. strict mode: reject unqualified builtins
   2. compatibility mode: accept but mark deprecated

### 3. Type and arity checking from overload matching

Replace ad hoc `if name == ...` logic with one matcher:

1. Lookup module/member in builtin registry.
2. Filter overloads by arity.
3. For each candidate overload, validate argument constraints.
4. If exactly one overload matches, use its return type.
5. If no overload matches, emit a typed diagnostic with expected signatures.

This naturally supports different parameter counts and types.

### 4. HIR representation update

Today, statement calls keep `module`, but expression calls do not.

For consistency and future safety, add `module: Option<String>` to `HExpr::Call` as well.

Benefits:

1. `IO.ReadInt` and any future `OtherModule.ReadInt` stay distinguishable after lowering.
2. Codegen can map builtin modules without re-deriving context from semantic side effects.

### 5. Code generation mapping

Codegen should map fully qualified builtins to runtime helpers via a small dispatcher table.

Initial mapping:

1. `IO.ReadInt()` -> `read_int()`
2. `IO.ReadReal()` -> `read_real()`
3. `IO.ReadLongReal()` -> `read_longreal()`
4. `IO.EOF()` -> `eof()`
5. `MATH.FLT(x)` -> `value_as_real(...)`
6. `MATH.FLOOR(x)` -> `value_as_integer(...)`
7. `IO.Write*` remain statement calls mapped to existing write emitters.

## Initial builtin signatures

`IO` module:

1. `WriteInt(INTEGER)` -> no return
2. `WriteString(string-literal)` -> no return
3. `WriteLn()` -> no return
4. `WriteReal(REAL)` -> no return
5. `WriteLongReal(LONGREAL)` -> no return
6. `ReadInt()` -> `INTEGER`
7. `ReadReal()` -> `REAL`
8. `ReadLongReal()` -> `LONGREAL`
9. `EOF()` -> `INTEGER` (`1` for EOF, `0` otherwise)

`MATH` module:

1. `FLT(INTEGER)` -> `REAL`
2. `FLOOR(REAL|LONGREAL)` -> `INTEGER`

## Diagnostics

Add focused diagnostics for module-builtins:

1. Unknown builtin module (for example `I0.ReadInt`).
2. Unknown builtin member (for example `IO.ReadINT`).
3. Wrong call context (using an expression-only builtin as statement call).
4. Arity mismatch with signature text.
5. Type mismatch with signature text.
6. Deprecated unqualified builtin usage (compatibility mode only).

## Test plan

### Parser tests

1. Parse `IO.ReadInt()` and `MATH.FLOOR(x)` as qualified call expressions.
2. Parse `IO.WriteInt(x)` and `IO.WriteLn()` as qualified statement calls.

### Semantic success cases

1. `x := IO.ReadInt()`
2. `IF IO.EOF() THEN ... END`
3. `r := MATH.FLT(i)` where `i: INTEGER`
4. `i := MATH.FLOOR(r)` where `r: REAL`
5. `i := MATH.FLOOR(lr)` where `lr: LONGREAL`

### Semantic error cases

1. `x := IO.ReadInt(1)` (arity mismatch)
2. `IO.ReadInt()` as statement call (wrong call context)
3. `x := MATH.FLT(r)` where `r: REAL` (type mismatch)
4. `x := MATH.FLOOR(i)` where `i: INTEGER` (type mismatch)
5. `x := IO.Unknown()` (unknown member)
6. `x := UNKNOWN.ReadInt()` (unknown module)

### Lowering/HIR tests

1. Verify expression-call module retention in HIR (`HExpr::Call.module`).
2. Verify statement-call module retention remains stable.

### Codegen unit tests

1. Ensure qualified `IO.ReadInt` emits `read_int()`.
2. Ensure qualified `IO.EOF` emits `eof()` in conditions.
3. Ensure qualified `MATH.FLT` and `MATH.FLOOR` emit conversion helpers.

### Golden integration tests

1. `io_qualified_readint_eof`
2. `io_qualified_real_longreal`
3. `math_qualified_floor_flt`
4. Compatibility test (if enabled): unqualified builtins still run and emit deprecation warnings.

## Example plan

Add three focused examples:

1. `examples/io-qualified-basic`
   1. Demonstrates `IO.ReadInt`, `IO.WriteInt`, `IO.WriteLn`.
2. `examples/io-qualified-eof`
   1. Demonstrates `IO.EOF` branching.
3. `examples/math-qualified-floor-flt`
   1. Demonstrates `MATH.FLT` and `MATH.FLOOR` with typed variables.

Each example should include:

1. `src/Main.ob0`
2. `README.md` with run command and expected output
3. optional stdin sample when input is required

## Migration plan

Phase 1:

1. Add builtin registry and qualified resolution in semantic analysis.
2. Keep unqualified compatibility mode on by default.
3. Add deprecation diagnostics for unqualified usage.

Phase 2:

1. Extend lowering/HIR with expression-call module retention.
2. Switch codegen dispatch to module + member lookup.

Phase 3:

1. Update examples and docs to qualified form.
2. Flip default to strict qualified mode.
3. Remove unqualified fallback in a later major/minor boundary (per release policy).

## Open decisions

1. Should `IO` and `MATH` names be reserved against user `IMPORT` aliases?
2. Should compatibility mode be controlled by CLI flag, manifest setting, or both?
3. Should `EOF()` become `BOOLEAN` in a later compatibility-breaking release?
