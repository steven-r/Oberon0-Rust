# IO Qualified Basic

This example demonstrates qualified IO builtin calls.

## What it shows

- reading one integer token from stdin with `IO.ReadInt()`
- printing the same value with `IO.WriteInt` and `IO.WriteLn`

## Source

- `src/Main.ob0`

## Run

From the repository root:

```bash
printf "42\n" | scripts/oberon0 examples/io-qualified-basic --run
```

## Expected output

```text
42
```
