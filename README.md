# Oberon0-Rust

A minimal Oberon0 compiler prototype written in Rust.

Compiler binary name: `oberon0c`.

## Language reference baseline

This project uses Niklaus Wirth's Compiler Construction books as the primary language-reference baseline for Oberon-0 grammar and staged compiler design:

1. [Compiler Construction 1 (PDF)](https://people.inf.ethz.ch/wirth/CompilerConstruction/CompilerConstruction1.pdf)
2. [Compiler Construction 2 (PDF)](https://people.inf.ethz.ch/wirth/CompilerConstruction/CompilerConstruction2.pdf)

The concrete repository-level alignment snapshot is tracked in:

- docs/wirth-page63-alignment-matrix.md

## AI-assisted development

Large parts of this compiler were built with help from GitHub Copilot (GPT-5.3-Codex).
The architecture, feature backlog, implementation iterations, and test expansion were significantly accelerated through AI-assisted development.

In short: this project is human-designed and reviewed, but heavily AI-boosted during implementation.

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

Run all tests:

    cargo test

Run parser corpus tests:

    cargo test parser::tests

Run scanner tests:

    cargo test scanner::tests

Run code generation tests:

    cargo test codegen::tests

Run CLI argument parsing tests:

    cargo test tests::cli_

## Git workflow and commit policy

This repository uses:

- Conventional Commits for commit messages
- SemVer for versioning (`MAJOR.MINOR.PATCH`)
- `pre-commit` hooks to enforce checks

Project conventions and persistent team decisions are documented in:

- docs/project-decisions.md
- CONTRIBUTING.md
- AGENTS.md
- .github/copilot-instructions.md

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

    cargo run -- examples/hello-app/src/Main.ob0 --out-dir target/generated

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

Compile from repository root:

        cargo run -- my-oberon-app/src/Main.ob0 --out-dir target/generated

Force state output on for a one-off run without editing the manifest:

    cargo run -- my-oberon-app/src/Main.ob0 --out-dir target/generated --emit-state

This repository includes the same layout as a runnable example at:

    examples/hello-app/

If your project uses `IMPORT`, add an optional manifest file:

        my-oberon-app/
            src/
                Main.ob0
            oberon.toml

Compile a manifest-backed project from repository root:

        cargo run -- my-oberon-app/src/Main.ob0 --manifest my-oberon-app/oberon.toml --out-dir target/generated

This repository includes a focused import/manifest example at:

        examples/imports-manifest/

Additional focused feature examples are listed in:

    examples/README.md

This creates a generated Rust project at:

    target/generated/<ModuleName>

### Source file basics

Minimal valid structure:

    MODULE Main;
        VAR x;
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

    [compiler]
    emit_state = true

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
- `compiler.emit_state = true` enables the generated `State: {...}` footer explicitly
- `--emit-state` and `--no-emit-state` override the manifest for a single compiler run
- Optional alias form in Oberon is supported: `IMPORT Local := External;`
- See `examples/imports-manifest/` for a focused project example using this layout.
- For the current multi-module translation workflow and limits, see `docs/module-translation-workflow.md`.

## Current language subset

The current compiler supports the Milestone A subset:

- `MODULE ... [BEGIN ...] END ... .`
- Optional `IMPORT` section
- Declarations include `CONST`, `TYPE`, `VAR`, and `PROCEDURE` declarations.
- `TYPE` declarations currently support built-in scalar targets `INTEGER`, `BOOLEAN`, `REAL`, `LONGREAL` and simple named aliases.
- `VAR` declarations may optionally carry declared types such as `VAR x: INTEGER;`, `VAR flag: BOOLEAN;`, or `VAR x: Count;`.
- User-defined type names remain reserved at module scope but may be shadowed by procedure parameters; built-in scalar names stay reserved, and a parameter cannot reuse the same user-defined type name in its own declaration as in `Count: Count`.
- Procedure declarations support optional local `VAR` sections before `BEGIN` (for example `PROCEDURE P; VAR x: INTEGER; BEGIN ... END P;`).
- Procedure-local `VAR` names may shadow user-defined module type aliases, but built-in scalar names stay reserved and declarations like `VAR Count: Count;` are rejected in procedure scope.
- Statements:
  - assignment: `x := expr`
  - call: `Proc(...)` or `Proc`
  - `IF ... THEN ... [ELSE ...] END`
  - `WHILE ... DO ... END`
- Expressions with integer literals, identifiers, and parentheses
- Operators: `+`, `-`, unary `+`, unary `-`, `*`, `/`, `DIV`, `MOD`, `OR`, `&`, unary `~`, `=`, `#`, `<`, `<=`, `>`, `>=`

Focused typed-declaration example:

- `examples/typed-declarations/`

Detailed subset and planning documents:

- docs/oberon0-v1-subset.md
- docs/operator-type-compatibility-matrix.md
- docs/milestone-a-backlog.md
- docs/milestone-b-roadmap.md
- docs/milestone-b-issue-backlog.md

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

## License

This repository is licensed under the MIT License.
See `LICENSE` for full text.
