mod ast;
mod codegen;
mod hir;
mod lower;
mod manifest;
mod parser;
mod scanner;
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
}

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

    analyze(&module, manifest.as_ref()).context("Semantic analysis failed")?;

    let hir = lower_module(&module).context("HIR lowering failed")?;

    let generated_dir = generate_rust_project(&hir, manifest.as_ref(), &cli.out_dir)
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
