# Shadowing in Procedure Scope

This example demonstrates shadowing of a module variable from inside a procedure scope.

In the current Oberon0 subset, procedures do not support their own `VAR` declarations yet.
Because of that, this example uses a procedure parameter as the procedure-local binding that shadows the module variable with the same name.

## What it shows

- module variables `x` and `y`
- procedure parameter `x` shadowing the module variable `x`
- module variable `y` still being updated from inside the procedure
- distinct final state entries for module and procedure scope

## Source

- `src/Main.ob0`

## Run

```bash
scripts/oberon0 examples/shadowing-procedure-scope --emit-state --run
```

## Expected output

```text
42
43
7
43
State: {"Mix.x": 42, "x": 7, "y": 43}
```

## State interpretation

- `x` is the module variable
- `Mix.x` is the procedure-scoped binding that shadows it inside `Mix`
- `y` remains a module variable and is updated by the procedure
