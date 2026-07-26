# imports-qualified

Demonstrates the Issue #26 setup for imports plus exported declarations,
and documents the expected qualified-name behavior.

**Features**:

- Type declaration with export marker: `TYPE IntType* = INTEGER;`
- Procedure declaration with export marker: `PROCEDURE HELLO*;`
- Import alias in Main: `IMPORT B := ModuleB;`

**Current Status**:

- The parser accepts qualified syntax (`B.HELLO`, `B.IntType`).
- Semantic cross-module resolution for qualified names is not implemented yet.
- Therefore, `Main.ob0` keeps the target syntax in a comment and remains runnable.

**Issue #26 Expected Behavior**:

- `B.HELLO;` resolves to an exported procedure in module `ModuleB`.
- `VAR x: B.IntType;` resolves to an exported type in module `ModuleB`.
- Non-exported members are rejected when referenced through `B.<name>`.

**Expected Output**:

```text
99
```

Main calls `TestProcedure`, which writes `99`.

## Run

```bash
cargo run -- examples/imports-qualified/src/Main.ob0 \
  --manifest examples/imports-qualified/oberon.toml \
  --out-dir target/generated-qualified
cd target/generated-qualified/Main
cargo run
```
