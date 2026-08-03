
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

[compiler]
emit_state = true
"#;
    fs::write(&path, content).expect("failed to write temp manifest");

    let manifest = ExternalManifest::from_file(&path).expect("manifest should parse");
    let math = manifest.resolve("Math").expect("Math binding should exist");
    assert_eq!(math.crate_name, "num-traits");
    assert_eq!(math.version, "0.2");

    let io = manifest.resolve("IO").expect("IO binding should exist");
    assert_eq!(io.features, vec!["std"]);
    assert!(manifest.compiler.emit_state);

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

#[test]
fn manifest_defaults_state_output_to_disabled() {
    let path = temp_manifest_path("default_flags");
    let content = r#"
[dependencies]
Math = { crate = "num-traits", version = "0.2" }
"#;
    fs::write(&path, content).expect("failed to write temp manifest");

    let manifest = ExternalManifest::from_file(&path).expect("manifest should parse");
    assert!(!manifest.compiler.emit_state);

    fs::remove_file(&path).expect("failed to remove temp manifest");
}
