# Wirth Page 63 Alignment Matrix

Reference: N. Wirth, *Compiler Construction*, Appendix A (page 63), Oberon-0 syntax and predefined procedures.

Purpose:

1. Compare the current repository subset against the page-63 reference.
2. Evaluate whether each gap is already covered by an existing GitHub issue.

Status legend:

- Aligned: implemented in current subset.
- Partial: implemented in reduced form.
- Missing: not implemented.

## Syntax alignment

| Reference item (page 63) | Current subset status | Alignment | Existing issue mapping | Evaluation |
| --- | --- | --- | --- | --- |
| `ident`, `integer` | Implemented | Aligned | n/a | No action needed |
| `selector` (`.` and `[ ]`) | Indexed and field selectors are implemented for the current subset | Partial | #8, #10 | Implemented for arrays and records; full selector parity remains broader |
| `factor` with `~` | Unary `~` is implemented for BOOLEAN expressions | Partial | #18 | Implemented for the current subset |
| `term` with `DIV`, `MOD`, `&` | `DIV`, `MOD`, and `&` are implemented with typed validation | Aligned | #18 | Implemented in current subset |
| `SimpleExpression` with unary sign and `OR` | Unary sign and `OR` are implemented | Aligned | #18 | Implemented in current subset |
| Relational operators in `expression` | Equality and ordering operators are implemented with numeric/boolean rules | Aligned | #19, #11 | Implemented in current subset |
| `assignment = ident selector := expression` | Indexed and record-field assignments are implemented | Partial | #8, #10 | Implemented for current subset selectors |
| `ProcedureCall = ident [ActualParameters \| "*"]` | Basic call forms only; no `*` form | Partial | #26 | Covered by dedicated issue |
| `IfStatement` with `ELSIF` | `IF/THEN/ELSE/END` implemented; no `ELSIF` | Partial | #20 | Covered by dedicated issue |
| `WhileStatement` | Implemented | Aligned | n/a | No action needed |
| `RepeatStatement` | Not implemented | Missing | #21 | Covered by dedicated issue |
| `StatementSequence` with semicolon-separated statements | Implemented | Aligned | n/a | No action needed |
| `ArrayType` | Implemented with constant-expression length folding | Aligned | #7 | Implemented in current subset |
| `RecordType` | Implemented with named fields | Aligned | #9 | Implemented in current subset |
| `type` non-terminal (`ident \| ArrayType \| RecordType`) | Builtin, array, named, qualified, and record type references are implemented in the current subset | Partial | #5, #7, #9 | Subset support is present, broader reference parity still continues |
| Scalar types `BOOLEAN`, `REAL`, `LONGREAL` | Implemented in declarations, expressions, and semantic/type rules | Aligned | #17 | Implemented in current subset |
| `FormalParameters` with typed sections and optional `VAR` | Typed parameter sections and `VAR` mode are implemented | Aligned | #22, #5 | Implemented in current subset |
| `ProcedureBody = declarations [BEGIN ...] END` | Procedure-local `VAR` declarations are implemented before `BEGIN`; broader local declaration forms remain pending | Partial | #16 | Partial subset support is implemented |
| `module` with optional `BEGIN` | Implemented | Aligned | #25 | Supported for declaration-only modules and normal module bodies |

## Predefined procedure/function alignment

| Reference item (page 63) | Current subset status | Alignment | Existing issue mapping | Evaluation |
| --- | --- | --- | --- | --- |
| `WriteInt(x, n)` | Implemented as `WriteInt(x)` without width parameter | Partial | #24 | Covered by dedicated issue |
| `WriteLn` | Implemented | Aligned | #12 | Covered and implemented |
| `ReadInt(x)` | Implemented as `ReadInt()` call expression (subset variant) | Partial | #13 | Implemented in subset; full p63 signature still differs |
| `eot()` / EOF check | Implemented as `EOF()` call expression | Partial | #13 | Implemented with naming variant (`eot` vs `EOF`) |
| `WriteChar(x)` | Not implemented | Missing | #23 | Covered by dedicated issue |
| `OpenInput` | Not implemented | Missing | #23 | Covered by dedicated issue |
| `LED(x)`, `Switch()` (teaching extensions) | Not implemented | Out of scope | n/a | Explicitly excluded from language scope |

## Issue coverage summary

Covered by existing issues:

1. String literals / `WriteString`: #2, #3.
2. IO baseline and follow-up split: #4, #12, #13.
3. Typed model and type-carrying pipeline: #5, #6, #61.
4. Arrays and indexed designators: #7, #8.
5. Records and field access: #9, #10 completed for the current subset.
6. Type-checking consolidation: #11 established the shared compatibility path used by arrays and records in the current subset.
7. Procedure-local declarations (`VAR` in procedure scope): #16.
8. Scalar builtin type coverage (`BOOLEAN`, `REAL`, `LONGREAL`): #17.

Previously uncovered items now tracked by dedicated issues:

1. Add boolean/logical and arithmetic Oberon operators (`DIV`, `MOD`, `&`, `OR`, unary `~`, unary sign handling): #18.
2. Add relational operators in parser and semantic/type rules: #19.
3. Add `ELSIF`: #20.
4. Add `REPEAT ... UNTIL`: #21.
5. Add typed formal parameters with optional `VAR`: #22.
6. Add `WriteChar` and `OpenInput` builtins: #23.
7. Support `WriteInt(x, n)` width parameter: #24.
8. Allow optional module-level `BEGIN`: #25.
9. Resolve and implement `ProcedureCall ... "*"`: #26.

Status after issue creation:

1. Operators (`DIV`, `MOD`, `&`, `OR`, unary `~`, unary sign): #18.
2. Relational operators and comparison type rules: #19.
3. `ELSIF`: #20.
4. `REPEAT ... UNTIL`: #21.
5. Typed formal parameters with optional `VAR` mode: #22.
6. `WriteChar` and `OpenInput`: #23.
7. `WriteInt(x, n)` width parameter: #24.
8. Optional module `BEGIN`: #25.
9. `ProcedureCall ... "*"` form: #26.
