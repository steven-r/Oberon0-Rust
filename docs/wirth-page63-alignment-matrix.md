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
| `selector` (`.` and `[ ]`) | Not implemented | Missing | #8, #10 | Covered by planned issues |
| `factor` with `~` | Only `ident`, `integer`, `string`, `(...)` | Partial | #18 | Covered by dedicated issue |
| `term` with `DIV`, `MOD`, `&` | Only `*` and `/` | Partial | #18 | Covered by dedicated issue |
| `SimpleExpression` with unary sign and `OR` | Unary sign and `OR` not implemented | Partial | #18 | Covered by dedicated issue |
| Relational operators in `expression` | Not implemented | Missing | #19, #11 | Covered by dedicated issue |
| `assignment = ident selector := expression` | Assignment exists; selector part missing | Partial | #8, #10 | Covered by designator issues |
| `ProcedureCall = ident [ActualParameters \| "*"]` | Basic call forms only; no `*` form | Partial | #26 | Covered by dedicated issue |
| `IfStatement` with `ELSIF` | `IF/THEN/ELSE/END` implemented; no `ELSIF` | Partial | #20 | Covered by dedicated issue |
| `WhileStatement` | Implemented | Aligned | n/a | No action needed |
| `RepeatStatement` | Not implemented | Missing | #21 | Covered by dedicated issue |
| `StatementSequence` with semicolon-separated statements | Implemented | Aligned | n/a | No action needed |
| `ArrayType` | Not implemented yet | Missing | #7 | Covered by planned issue |
| `RecordType` | Not implemented yet | Missing | #9 | Covered by planned issue |
| `type` non-terminal (`ident \| ArrayType \| RecordType`) | Not implemented yet | Missing | #5, #7, #9 | Covered by planned issues |
| Scalar types `BOOLEAN`, `REAL`, `LONGREAL` | Not implemented yet | Missing | #17 | Covered by dedicated issue |
| `FormalParameters` with typed sections and optional `VAR` | Untyped ident list only; no `VAR` mode | Partial | #22, #5 | Covered by dedicated issue |
| `ProcedureBody = declarations [BEGIN ...] END` | Procedure body supports statements only; no local declarations | Partial | #16 | Covered by explicit issue |
| `module` with optional `BEGIN` | Current grammar requires `BEGIN` | Partial | #25 | Covered by dedicated issue |

## Predefined procedure/function alignment

| Reference item (page 63) | Current subset status | Alignment | Existing issue mapping | Evaluation |
| --- | --- | --- | --- | --- |
| `WriteInt(x, n)` | Implemented as `WriteInt(x)` without width parameter | Partial | #24 | Covered by dedicated issue |
| `WriteLn` | Implemented | Aligned | #12 | Covered and implemented |
| `ReadInt(x)` | Planned as builtin input support | Missing | #13 | Covered by planned issue |
| `eot()` / EOF check | Planned as `EOF()` | Missing | #13 | Covered by planned issue |
| `WriteChar(x)` | Not implemented | Missing | #23 | Covered by dedicated issue |
| `OpenInput` | Not implemented | Missing | #23 | Covered by dedicated issue |
| `LED(x)`, `Switch()` (teaching extensions) | Not implemented | Out of scope | n/a | Explicitly excluded from language scope |

## Issue coverage summary

Covered by existing issues:

1. String literals / `WriteString`: #2, #3.
2. IO baseline and follow-up split: #4, #12, #13.
3. Typed model and type-carrying pipeline: #5, #6.
4. Arrays and indexed designators: #7, #8.
5. Records and field access: #9, #10.
6. Type-checking consolidation: #11.
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
