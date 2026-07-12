# Procedure-Local Variables

This example demonstrates procedure-local `VAR` declarations.

## What it shows

- module variable `base`
- procedure-local variables `i` and `total` declared before `BEGIN`
- counting loop over a procedure-local variable
- module variable remains unchanged after procedure execution

## Source

- `src/Main.ob0`

## Run

```bash
scripts/oberon0 examples/procedure-local-vars --run
```

## Expected output

```text
15
5
```
