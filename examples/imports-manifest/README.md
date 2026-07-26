# Imports and Manifest Mapping

This example demonstrates the project layout used when an Oberon0 source file includes an `IMPORT` section:

- module imports
- optional `oberon.toml` dependency mapping
- generated Cargo dependencies derived from imported names

## Source

- `src/Main.ob0`
- `oberon.toml`

## Run

From the repository root:

```bash
scripts/oberon0 examples/imports-manifest --run
```

## Expected output

```text
42
State: {"x": 42}
```

## What to inspect

After generation, inspect `target/generated/Main/Cargo.toml` to see the dependency emitted from `oberon.toml`.

For the full current workflow and known limits of translating module-based projects, see `docs/module-translation-workflow.md`.
