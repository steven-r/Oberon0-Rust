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
        toml::from_str(&raw)
            .with_context(|| format!("Manifest is invalid: {}", path.display()))
    }

    /// Resolves an imported Oberon0 name to its crate binding.
    pub fn resolve(&self, import_name: &str) -> Option<&CrateBinding> {
        self.dependencies.get(import_name)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::ExternalManifest;

    fn temp_manifest_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("oberon0_manifest_{}_{}.toml", name, nanos))
    }

    #[test]
    fn parses_and_resolves_dependency_bindings() {
        let path = temp_manifest_path("valid");
        let content = r#"
[dependencies]
Math = { crate = "num-traits", version = "0.2" }
IO = { crate = "termcolor", package = "termcolor", version = "1.4", features = ["std"] }
"#;
        fs::write(&path, content).expect("failed to write temp manifest");

        let manifest = ExternalManifest::from_file(&path).expect("manifest should parse");
        let math = manifest.resolve("Math").expect("Math binding should exist");
        assert_eq!(math.crate_name, "num-traits");
        assert_eq!(math.version, "0.2");

        let io = manifest.resolve("IO").expect("IO binding should exist");
        assert_eq!(io.features, vec!["std"]);

        fs::remove_file(&path).expect("failed to remove temp manifest");
    }

    #[test]
    fn invalid_manifest_returns_error() {
        let path = temp_manifest_path("invalid");
        fs::write(&path, "[dependencies\nMath = { crate = \"x\" }")
            .expect("failed to write invalid temp manifest");

        let result = ExternalManifest::from_file(&path);
        assert!(result.is_err(), "invalid TOML should fail");

        fs::remove_file(&path).expect("failed to remove temp manifest");
    }
}
