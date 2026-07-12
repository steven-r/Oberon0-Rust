//! Command-line entry point for the Oberon0 to Rust compiler pipeline.

mod ast;
mod codegen;
mod hir;
mod lower;
mod manifest;
mod parser;
mod scanner;
mod scope;
mod semantic;
mod symbols;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Parser;

use codegen::generate_rust_project;
use lower::lower_module;
use manifest::ExternalManifest;
use parser::parse_module;
use scanner::scan;
use semantic::analyze;

#[derive(Parser, Debug)]
#[command(name = "oberon0c")]
#[command(about = "Minimal Oberon0 compiler targeting Rust/LLVM")]
/// Command-line options for running the compiler pipeline.
struct Cli {
    /// Path to the Oberon0 source file
    input: PathBuf,

    /// Optional manifest mapping external Oberon imports to Rust crates
    #[arg(long)]
    manifest: Option<PathBuf>,

    /// Output directory for the generated Rust project
    #[arg(long, default_value = "target/generated")]
    out_dir: PathBuf,

    /// Build the generated project directly with cargo
    #[arg(long)]
    build: bool,

    /// Force generated programs to print their final runtime state
    #[arg(long, conflicts_with = "no_emit_state")]
    emit_state: bool,

    /// Force generated programs to suppress final runtime state output
    #[arg(long, conflicts_with = "emit_state")]
    no_emit_state: bool,
}

/// Runs scanning, parsing, semantic analysis, lowering, and Rust code generation.
fn main() -> Result<()> {
    let cli = Cli::parse();

    let source = fs::read_to_string(&cli.input)
        .with_context(|| format!("Failed to read input file: {}", cli.input.display()))?;

    let tokens = scan(&source)?;
    let module = parse_module(&source).context("Parsing failed")?;

    let manifest = match &cli.manifest {
        Some(path) => Some(ExternalManifest::from_file(path)?),
        None => None,
    };

    let emit_state = resolve_emit_state(&cli, manifest.as_ref());

    analyze(&module, manifest.as_ref()).context("Semantic analysis failed")?;

    let hir = lower_module(&module).context("HIR lowering failed")?;

    let generated_dir = generate_rust_project(&hir, manifest.as_ref(), &cli.out_dir, emit_state)
        .context("Code generation failed")?;

    println!("Scan: {} Tokens", tokens.len());
    println!("Parse: module '{}' succeeded", module.name);
    println!("Generated: {}", generated_dir.display());

    if cli.build {
        let status = Command::new("cargo")
            .arg("build")
            .current_dir(&generated_dir)
            .status()
            .context("Failed to start cargo build")?;

        if !status.success() {
            bail!("cargo build failed in generated project");
        }
        println!("Build: succeeded");
    }

    Ok(())
}

fn resolve_emit_state(cli: &Cli, manifest: Option<&ExternalManifest>) -> bool {
    if cli.emit_state {
        true
    } else if cli.no_emit_state {
        false
    } else {
        manifest.is_some_and(|manifest| manifest.compiler.emit_state)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::Cli;

    #[test]
    fn cli_requires_input_path() {
        let parsed = Cli::try_parse_from(["oberon0c"]);
        assert!(parsed.is_err(), "CLI should reject missing input path");
    }

    #[test]
    fn cli_uses_default_out_dir() {
        let parsed = Cli::try_parse_from(["oberon0c", "src/Main.ob0"])
            .expect("CLI parse should succeed");
        assert_eq!(parsed.input, PathBuf::from("src/Main.ob0"));
        assert_eq!(parsed.out_dir, PathBuf::from("target/generated"));
        assert!(parsed.manifest.is_none());
        assert!(!parsed.build);
        assert!(!parsed.emit_state);
        assert!(!parsed.no_emit_state);
    }

    #[test]
    fn cli_accepts_manifest_out_dir_build_and_emit_state_flag() {
        let parsed = Cli::try_parse_from([
            "oberon0c",
            "examples/hello-app/src/Main.ob0",
            "--manifest",
            "examples/hello-app/oberon.toml",
            "--out-dir",
            "target/generated-a",
            "--emit-state",
            "--build",
        ])
        .expect("CLI parse should succeed");

        assert_eq!(
            parsed.input,
            PathBuf::from("examples/hello-app/src/Main.ob0")
        );
        assert_eq!(
            parsed.manifest,
            Some(PathBuf::from("examples/hello-app/oberon.toml"))
        );
        assert_eq!(parsed.out_dir, PathBuf::from("target/generated-a"));
        assert!(parsed.build);
        assert!(parsed.emit_state);
        assert!(!parsed.no_emit_state);
    }

    #[test]
    fn cli_rejects_conflicting_state_flags() {
        let parsed = Cli::try_parse_from([
            "oberon0c",
            "src/Main.ob0",
            "--emit-state",
            "--no-emit-state",
        ]);
        assert!(parsed.is_err(), "CLI should reject conflicting state flags");
    }

    #[test]
    fn cli_state_flags_override_manifest_setting() {
        let manifest = crate::manifest::ExternalManifest {
            dependencies: std::collections::BTreeMap::new(),
            compiler: crate::manifest::CompilerConfig { emit_state: true },
        };
        let parsed = Cli::try_parse_from(["oberon0c", "src/Main.ob0", "--no-emit-state"])
            .expect("CLI parse should succeed");
        assert!(!super::resolve_emit_state(&parsed, Some(&manifest)));

        let manifest = crate::manifest::ExternalManifest {
            dependencies: std::collections::BTreeMap::new(),
            compiler: crate::manifest::CompilerConfig { emit_state: false },
        };
        let parsed = Cli::try_parse_from(["oberon0c", "src/Main.ob0", "--emit-state"])
            .expect("CLI parse should succeed");
        assert!(super::resolve_emit_state(&parsed, Some(&manifest)));
    }
}
