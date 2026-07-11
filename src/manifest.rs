use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ExternalManifest {
    #[serde(default)]
    pub dependencies: BTreeMap<String, CrateBinding>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CrateBinding {
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub version: String,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
}

impl ExternalManifest {
    pub fn from_file(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Could not read manifest: {}", path.display()))?;
        toml::from_str(&raw)
            .with_context(|| format!("Manifest is invalid: {}", path.display()))
    }

    pub fn resolve(&self, import_name: &str) -> Option<&CrateBinding> {
        self.dependencies.get(import_name)
    }
}
