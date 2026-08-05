# Milestone B Issue Backlog

This backlog mirrors the current Milestone B implementation order and keeps the completed foundation items grouped before the next feature slice.

## Completed foundation

1. #5: add `TYPE` declarations and typed `VAR` declarations.
2. #6: preserve resolved type information in symbols and HIR.
3. #17: add builtin scalar types `BOOLEAN`, `REAL`, and `LONGREAL`.
4. #61: formalize the shared type model foundation for later arrays and records.
5. #7: add array type declarations.
6. #8: add indexed designators.
7. #22: add typed formal parameters with optional `VAR` mode.
8. #9: add record type declarations.
9. #10: add field-selection designators.
10. #11: consolidate type checking around the shared model.

## Next implementation slice

1. #20: add `ELSIF`.
2. #21: add `REPEAT ... UNTIL`.
3. #23: add `WriteChar` and `OpenInput`.

## Follow-on language-completeness work

1. #24: add `WriteInt(x, n)` width handling.
2. #26: resolve the star-form ProcedureCall decision.
3. Broaden selector/designator parity beyond the current subset where needed.

## Remaining roadmap items

1. Keep extending Wirth page-63 parity beyond the current subset boundaries.
2. Revisit cross-module qualified-variable/designator semantics when module translation broadens.

Related concept:

1. docs/internal-builtin-modules-concept.md
