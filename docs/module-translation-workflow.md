# Module Translation Workflow

This document explains the current workflow for translating projects that use `IMPORT`.

## Current state

The compiler currently translates one Oberon0 source file per invocation:

```bash
cargo run -- path/to/src/Main.ob0 --out-dir target/generated
```

When `IMPORT` is present, semantic analysis validates the imported alias mapping and selected qualified-name checks. The generated Rust project receives Cargo dependencies from `oberon.toml`.

## Recommended project layout

```text
my-app/
  src/
    Main.ob0
  oberon.toml
```

## Manifest mapping for imported modules

Use `oberon.toml` to map imported external names to Cargo dependencies:

```toml
[dependencies]
ModuleB = { crate = "moduleb-runtime", version = "0.1" }

[compiler]
emit_state = false
```

With this source:

```oberon
MODULE Main;
IMPORT B := ModuleB;
BEGIN
  B.HELLO
END Main.
```

run:

```bash
cargo run -- my-app/src/Main.ob0 --manifest my-app/oberon.toml --out-dir target/generated
```

The generated `Cargo.toml` contains a dependency derived from the import alias (`b`) and mapped package (`moduleb-runtime`).

## What "translate modules together" means today

The current implementation does not compile multiple Oberon0 modules in a single compiler invocation. Instead:

1. Translate one root module per run.
2. Provide import-to-crate mappings in `oberon.toml`.
3. Use generated Cargo dependencies to compose runtime pieces at the Rust/Cargo layer.

## Current limitations

- Qualified variable expressions (for example `x := B.value`) are rejected with semantic error `E015`.
- Qualified call and qualified type support is currently scoped to the present semantic/export model.
- There is no automatic interface extraction from other Oberon0 source files yet.
