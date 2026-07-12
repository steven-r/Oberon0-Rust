# Typed Declarations

This example demonstrates the first typed declaration slice in Milestone B.

## What it shows

- `TYPE` alias declaration
- typed `VAR` declarations using `INTEGER`, `BOOLEAN`, `REAL`, and `LONGREAL`
- typed `VAR` declarations using named aliases
- user-defined type names that may be shadowed by procedure parameters, while built-in scalar names remain reserved
- ordinary integer assignment and output

## Source

- `src/Main.ob0`

## Run

```bash
scripts/oberon0 examples/typed-declarations --run
```

## Expected output

```text
36
```
