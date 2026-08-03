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
    let parsed =
        Cli::try_parse_from(["oberon0c", "src/Main.ob0"]).expect("CLI parse should succeed");
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
    let manifest = oberon0c::manifest::ExternalManifest {
        dependencies: std::collections::BTreeMap::new(),
        compiler: oberon0c::manifest::CompilerConfig { emit_state: true },
    };
    let parsed = Cli::try_parse_from(["oberon0c", "src/Main.ob0", "--no-emit-state"])
        .expect("CLI parse should succeed");
    assert!(!super::resolve_emit_state(&parsed, Some(&manifest)));

    let manifest = oberon0c::manifest::ExternalManifest {
        dependencies: std::collections::BTreeMap::new(),
        compiler: oberon0c::manifest::CompilerConfig { emit_state: false },
    };
    let parsed = Cli::try_parse_from(["oberon0c", "src/Main.ob0", "--emit-state"])
        .expect("CLI parse should succeed");
    assert!(super::resolve_emit_state(&parsed, Some(&manifest)));
}

#[test]
fn cli_uses_manifest_state_setting_when_no_flags_are_present() {
    let parsed =
        Cli::try_parse_from(["oberon0c", "src/Main.ob0"]).expect("CLI parse should succeed");

    let enabled_manifest = oberon0c::manifest::ExternalManifest {
        dependencies: std::collections::BTreeMap::new(),
        compiler: oberon0c::manifest::CompilerConfig { emit_state: true },
    };
    assert!(super::resolve_emit_state(&parsed, Some(&enabled_manifest)));

    let disabled_manifest = oberon0c::manifest::ExternalManifest {
        dependencies: std::collections::BTreeMap::new(),
        compiler: oberon0c::manifest::CompilerConfig { emit_state: false },
    };
    assert!(!super::resolve_emit_state(
        &parsed,
        Some(&disabled_manifest)
    ));
}
