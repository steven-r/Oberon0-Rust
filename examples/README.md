# Oberon0 Examples

This directory contains small, focused example projects. Each example includes:

- `src/Main.ob0`
- a short `README.md`

Run any example from the repository root:

```bash
scripts/oberon0 <example-directory> --run
```

## Example list

- `hello-app`: minimal end-to-end starter with expression and output
- `imports-manifest`: import declarations plus `oberon.toml` crate mapping
- `expressions-basic`: operator precedence and parenthesized expressions
- `declarations-const-var`: `CONST` and `VAR` declarations
- `typed-declarations`: `TYPE` aliases and typed `VAR` declarations with `INTEGER`
- `control-if-else`: branching with `IF/ELSE`
- `control-while`: looping with `WHILE`
- `procedures-params`: parameter shadowing with distinct module and procedure state values
- `procedure-local-vars`: procedure-local `VAR` declarations and local loop/state handling
- `procedures-nested-control`: procedure body with nested control flow
- `shadowing-procedure-scope`: module-variable shadowing from procedure scope using local procedure bindings
- `shadowing-nested-control`: parameter-based shadowing combined with nested `WHILE` and `IF/ELSE` logic inside the procedure body
- `readint-basic`: integer input with `ReadInt()` and output echo
- `eof-check`: input-state branching with `EOF()` (`1` = EOF, `0` = input available)
