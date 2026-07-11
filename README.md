# oberon0c

A minimal Oberon0 compiler prototype written in Rust.

Current pipeline:

1. Scan Oberon0 source using Logos
2. Parse to AST using Pest
3. Run semantic checks
4. Generate a Rust project
5. Build generated Rust project with Cargo (optional)

The generated Rust code is compiled by `rustc`/LLVM, so this project can target machine code without implementing a native backend yet.

## Prerequisites

- Rust toolchain (stable)
- Cargo

Check:

    rustc --version
    cargo --version

## Build the compiler

From the repository root:

    cargo build

For development checks:

    cargo check

Run tests/checks later as they are added.

Run all tests:

    cargo test

Run parser corpus tests:

    cargo test parser::tests

## Git workflow and commit policy

This repository uses:

- Conventional Commits for commit messages
- SemVer for versioning (`MAJOR.MINOR.PATCH`)
- `pre-commit` hooks to enforce checks

Set up hooks locally:

    pre-commit install --hook-type pre-commit --hook-type commit-msg

Run all hooks manually:

    pre-commit run --all-files

Versioning details are documented in:

    VERSIONING.md

Release process checklist:

    RELEASE_CHECKLIST.md

## Changelog and releases

Generate a changelog section from Conventional Commits:

    scripts/changelog.sh --from-tag "$(git describe --tags --abbrev=0)" --to-ref HEAD

Create a SemVer release (updates version, updates changelog, commits, tags):

    scripts/release.sh patch
    scripts/release.sh minor
    scripts/release.sh major

Preview a release without commit/tag:

    scripts/release.sh patch --dry-run

## Compile an Oberon0 file

Basic usage:

    cargo run -- <path-to-source.ob0>

Choose output directory for generated Rust project:

    cargo run -- <path-to-source.ob0> --out-dir target/generated

Generate and build immediately:

    cargo run -- <path-to-source.ob0> --out-dir target/generated --build

## Quick start with the included example

Compile example source:

    cargo run -- examples/hello-app/src/Main.ob0 --manifest examples/hello-app/oberon.toml --out-dir target/generated

Short wrapper command (project directory instead of full file arguments):

    scripts/oberon0 examples/hello-app

Build and run generated project in one step:

    scripts/oberon0 examples/hello-app --run

Optional Cargo-style entry point (after adding `scripts/` to `PATH`):

    cargo oberon0 examples/hello-app

Run the generated project:

    cd target/generated/Main
    cargo run

## Setting up an Oberon0 source project

Recommended layout:

    my-oberon-app/
      src/
        Main.ob0
      oberon.toml

Compile from repository root:

    cargo run -- my-oberon-app/src/Main.ob0 --manifest my-oberon-app/oberon.toml --out-dir target/generated

This repository includes the same layout as a runnable example at:

    examples/hello-app/

Additional focused feature examples are listed in:

    examples/README.md

This creates a generated Rust project at:

    target/generated/<ModuleName>

### Source file basics

Minimal valid structure:

    MODULE Main;
    BEGIN
      x := 1 + 2;
      WriteInt(x);
    END Main.

## External libraries via manifest

Use `--manifest` to map Oberon imports to Rust crates.

Example `oberon.toml`:

    [dependencies]
    Math = { crate = "num-traits", version = "0.2" }
    IO = { crate = "termcolor", version = "1.4" }

Then your Oberon source can import those names:

    MODULE Main;
    IMPORT Math, IO;
    BEGIN
      WriteInt(42);
    END Main.

Notes:

- `dependencies.<Name>` is the Oberon import name
- `crate` is the Rust crate package name
- `version` is passed into generated Cargo.toml
- Optional alias form in Oberon is supported: `IMPORT Local := External;`

## Current language subset

The parser currently supports a small MVP subset:

- `MODULE ... END ... .`
- Optional `IMPORT` section
- Statements:
  - assignment: `x := expr`
  - call: `Proc(...)` or `Proc`
- Expressions with integer literals, variables, parentheses
- Operators: `+`, `-`, `*`, `/`

More Oberon0 features (such as declarations, procedures, control flow) can be added incrementally.

Detailed subset and planning documents:

- docs/oberon0-v1-subset.md
- docs/milestone-a-backlog.md

## Troubleshooting

- Parsing errors: validate module structure and statement separators
- Semantic errors for imports: ensure every imported external name is present in `oberon.toml`
- Build errors in generated project: inspect `target/generated/<ModuleName>/Cargo.toml` and `src/main.rs`

## Repository structure

- Compiler entry point: `src/main.rs`
- Grammar: `src/oberon0.pest`
- Scanner: `src/scanner.rs`
- Parser: `src/parser.rs`
- AST: `src/ast.rs`
- Semantic checks: `src/semantic.rs`
- Manifest model: `src/manifest.rs`
- Rust code generation: `src/codegen.rs`
- Example sources: `examples/`
