use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
/// External dependency manifest that maps Oberon0 imports to Rust crates.
pub struct ExternalManifest {
    #[serde(default)]
    /// Import bindings keyed by the external import name.
    pub dependencies: BTreeMap<String, CrateBinding>,
    #[serde(default)]
    /// Optional compiler configuration applied during code generation.
    pub compiler: CompilerConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
/// Optional compiler settings loaded from `oberon.toml`.
pub struct CompilerConfig {
    #[serde(default)]
    /// Emits the generated `State: {...}` footer when true.
    pub emit_state: bool,
}

#[derive(Debug, Clone, Deserialize)]
/// Single manifest entry describing how one import maps to Cargo metadata.
pub struct CrateBinding {
    #[serde(rename = "crate")]
    /// Rust crate name used in generated code.
    pub crate_name: String,
    /// Cargo version requirement written into `Cargo.toml`.
    pub version: String,
    #[serde(default)]
    /// Optional package name when the dependency key differs from the package.
    pub package: Option<String>,
    #[serde(default)]
    /// Extra Cargo features enabled for the dependency.
    pub features: Vec<String>,
}

impl ExternalManifest {
    /// Loads and parses a manifest file from disk.
    pub fn from_file(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Could not read manifest: {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("Manifest is invalid: {}", path.display()))
    }

    /// Resolves an imported Oberon0 name to its crate binding.
    pub fn resolve(&self, import_name: &str) -> Option<&CrateBinding> {
        self.dependencies.get(import_name)
    }
}

#[cfg(test)]
mod tests;
