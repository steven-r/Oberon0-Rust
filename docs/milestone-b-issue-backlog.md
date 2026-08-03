# Milestone B Issue Backlog

This backlog mirrors the current Milestone B implementation order and keeps the completed foundation items grouped before the next feature slice.

## Completed foundation

1. #5: add `TYPE` declarations and typed `VAR` declarations.
2. #6: preserve resolved type information in symbols and HIR.
3. #17: add builtin scalar types `BOOLEAN`, `REAL`, and `LONGREAL`.
4. #61: formalize the shared type model foundation for later arrays and records.
5. #7: add array type declarations.
6. #8: add indexed designators.

## Next implementation slice

1. #9: add record type declarations.
2. #10: add field-selection designators.
3. #11: consolidate type checking around the shared model.

## Follow-on language-completeness work

1. #22: add typed formal parameters with optional `VAR` mode.
2. #18: add `DIV`, `MOD`, `&`, `OR`, unary `~`, and unary sign operators.
3. #19: add relational operators.

## Remaining roadmap items

1. #20: add `ELSIF`.
2. #21: add `REPEAT ... UNTIL`.
3. #23: add `WriteChar` and `OpenInput`.
4. #24: add `WriteInt(x, n)` width handling.
5. #25: allow an optional module-level `BEGIN`.
6. #26: resolve the star-form ProcedureCall decision.
