use std::fs;
use std::path::Path;

use oberon0c::codegen::generate_rust_project;
use oberon0c::lower::lower_module;
use oberon0c::manifest::ExternalManifest;
use oberon0c::parser::parse_module;
use oberon0c::scanner::scan;
use oberon0c::semantic::analyze;

fn run_golden_case(case_name: &str, case_dir: &Path) -> datatest_stable::Result<()> {
    let config_file = case_dir.join("config.toml");
    let config = if config_file.exists() {
        fs::read_to_string(&config_file).expect("golden case config.toml should be readable")
    } else {
        String::new()
    };
    let emit_state = if config.trim().is_empty() {
        case_name.contains("emit_state")
    } else {
        toml::from_str::<ExternalManifest>(&config)
            .map(|manifest| manifest.compiler.emit_state)
            .unwrap_or_else(|_| case_name.contains("emit_state"))
    };
    let source = fs::read_to_string(case_dir.join("source.ob0"))
        .expect("golden case source.ob0 should exist");
    let expected_stdout_path = case_dir.join("expected_stdout.txt");
    let expected_stdout = if expected_stdout_path.exists() {
        Some(
            fs::read_to_string(&expected_stdout_path)
                .expect("golden case expected_stdout.txt should be readable"),
        )
    } else {
        None
    };

    let expected_exit_code_path = case_dir.join("expected_exit_code.txt");
    let expected_exit_code = if expected_exit_code_path.exists() {
        fs::read_to_string(&expected_exit_code_path)
            .expect("golden case expected_exit_code.txt should be readable")
            .trim()
            .parse::<i32>()
            .expect("golden expected exit code must be a valid i32")
    } else {
        0
    };

    scan(&source).expect("golden source should scan");
    let module = parse_module(&source).expect("golden source should parse");
    analyze(&module, None).expect("golden source should pass semantic analysis");
    let hir = lower_module(&module).expect("golden source should lower");

    let out_root = temp_codegen_dir(&format!("golden_{}", case_name));
    let project_dir = generate_rust_project(&hir, None, &out_root, emit_state)
        .expect("golden project generation should succeed");

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run")
        .current_dir(&project_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().expect("golden generated project should start");

    let stdin_path = case_dir.join("stdin.txt");
    if stdin_path.exists() {
        let input = fs::read(stdin_path).expect("golden stdin.txt should be readable");
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("stdin should be piped");
        stdin
            .write_all(&input)
            .expect("should write stdin for golden case");
    }

    let output = child
        .wait_with_output()
        .expect("golden generated project should finish");
    let actual_exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(
        actual_exit_code,
        expected_exit_code,
        "golden exit code mismatch for case {case_name}; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("golden stdout should be utf-8");
    if let Some(expected_stdout) = expected_stdout {
        assert_eq!(
            stdout, expected_stdout,
            "golden stdout mismatch for case {case_name}"
        );
    }

    let stderr = String::from_utf8(output.stderr).expect("golden stderr should be utf-8");

    let expected_stdout_contains_path = case_dir.join("expected_stdout_contains.txt");
    if expected_stdout_contains_path.exists() {
        let expected_stdout_contains = fs::read_to_string(&expected_stdout_contains_path)
            .expect("golden case expected_stdout_contains.txt should be readable")
            .trim()
            .to_string();
        assert!(
            stdout.contains(&expected_stdout_contains),
            "golden stdout does not contain expected substring for case {case_name}: {expected_stdout_contains}"
        );
    }

    let expected_stderr_contains_path = case_dir.join("expected_stderr_contains.txt");
    if expected_stderr_contains_path.exists() {
        let expected_stderr_contains = fs::read_to_string(&expected_stderr_contains_path)
            .expect("golden case expected_stderr_contains.txt should be readable")
            .trim()
            .to_string();
        assert!(
            stderr.contains(&expected_stderr_contains),
            "golden stderr does not contain expected substring for case {case_name}: {expected_stderr_contains}"
        );
    }

    let expected_main_path = case_dir.join("expected_main.rs");
    if expected_main_path.exists() {
        let expected_main = fs::read_to_string(&expected_main_path)
            .expect("golden expected_main.rs should be readable");
        let generated_main = fs::read_to_string(project_dir.join("src").join("main.rs"))
            .expect("generated main.rs should be readable");
        assert_eq!(
            generated_main, expected_main,
            "golden generated main.rs mismatch for case {case_name}"
        );
    }

    std::fs::remove_dir_all(&out_root).expect("golden temp codegen dir should be removable");
    Ok(())
}

fn temp_codegen_dir(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("oberon0_codegen_{}_{}", name, nanos))
}

fn run_golden_tests(path: &Path) -> datatest_stable::Result<()> {
    let case_dir = path
        .parent()
        .expect("golden test case should have a parent directory");
    let case_name = case_dir
        .file_name()
        .expect("golden test case directory should have a name")
        .to_string_lossy()
        .to_string();
    run_golden_case(&case_name, &case_dir)
}

datatest_stable::harness! {
    { test = run_golden_tests, root = "tests/codegen_golden", pattern = r".*/.*\.ob0" }
}
