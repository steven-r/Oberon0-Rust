# Procedures with Parameters

This example demonstrates procedure-scope shadowing and state inspection:

- module variable declaration
- procedure parameter that shadows the module variable with the same name
- different runtime values for module and procedure scope

## Source

- `src/Main.ob0`

## Run

```bash
scripts/oberon0 examples/procedures-params --emit-state --run
```

## Expected output

```text
42
7
State: {"Show.x": 42, "x": 7}
```

The final state shows both bindings separately:

- `x` is the module variable
- `Show.x` is the procedure parameter that shadows it inside `Show`
