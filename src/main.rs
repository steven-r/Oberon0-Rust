//! Command-line entry point for the Oberon0 to Rust compiler pipeline.

#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Parser;

use oberon0c::codegen::generate_rust_project;
use oberon0c::lower::lower_module;
use oberon0c::manifest::ExternalManifest;
use oberon0c::parser::parse_module;
use oberon0c::scanner::scan;
use oberon0c::semantic::analyze;

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
#[cfg_attr(coverage_nightly, coverage(off))]
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
mod tests;
