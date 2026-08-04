# IO Qualified EOF

This example demonstrates `IO.EOF()` in a branch condition.

## What it shows

- checking whether stdin already reached end-of-input
- branching on the `IO.EOF()` result (`1` = EOF, `0` = input available)
- writing a `hasInput` flag (`0` = no input available, `1` = input available)

## Source

- `src/Main.ob0`

## Run

From the repository root:

```bash
# No input provided: IO.EOF() == 1
scripts/oberon0 examples/io-qualified-eof --run < /dev/null

# Input provided: IO.EOF() == 0
printf "7\n" | scripts/oberon0 examples/io-qualified-eof --run
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
