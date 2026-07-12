# EOF Check

This example demonstrates `EOF()` as an input-state check.

## What it shows

- checking whether stdin already reached end-of-input
- branching on the `EOF()` result (`1` = EOF, `0` = input available)
- writing a numeric flag (`0` or `1`)

## Source

- `src/Main.ob0`

## Run

From the repository root:

```bash
# No input provided: EOF() == 1
scripts/oberon0 examples/eof-check --run

# Input provided: EOF() == 0
printf "7\n" | scripts/oberon0 examples/eof-check --run
```

## Expected output

Without input:

```text
0
```

With input:

```text
1
```
