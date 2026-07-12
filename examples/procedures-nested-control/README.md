# Procedures with Nested Control Flow

This example combines multiple language features in one program:

- procedure declaration with one parameter
- shared module variable updated from inside the procedure
- nested `WHILE` + `IF/ELSE`
- procedure call from module body

## Source

- `src/Main.ob0`

## Run

```bash
scripts/oberon0 examples/procedures-nested-control --run
```

## Expected output

```text
3
2
100
State: {"current": 0}
```
