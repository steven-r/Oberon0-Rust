# Shadowing with Nested Control Flow

This example demonstrates the same shadowing pattern as the basic procedure-scope example, but inside a procedure that also contains nested control flow.

In the current Oberon0 subset, procedures do not support their own `VAR` declarations yet.
Because of that, the procedure-scoped binding is modeled with a parameter that shadows the module variable of the same name.

## What it shows

- module variables `x` and `sum`
- procedure parameter `x` shadowing the module variable `x`
- nested `WHILE` and `IF/ELSE` inside the procedure body
- the shadowed procedure binding changing independently from the module variable
- final state output containing both bindings distinctly

## Source

- `src/Main.ob0`

## Run

```bash
scripts/oberon0 examples/shadowing-nested-control --emit-state --run
```

## Expected output

```text
3
2
1
3
105
State: {"Walk.x": 0, "sum": 105, "x": 3}
```

## State interpretation

- `x` is the module variable and remains `3`
- `Walk.x` is the procedure-scoped binding and counts down to `0`
- `sum` is a module variable updated from inside the procedure body
