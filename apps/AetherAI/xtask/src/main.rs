use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const VERSION: &str = "0.2.2";
const REPORT_NAME: &str = "AetherAI-v0.2.2-VERIFY.txt";
const LIVE_CHECKS: [&str; 4] = [
    "MODEL_SERVICE",
    "LIVE_MODEL_LOAD",
    "LIVE_GENERATION",
    "TOKEN_STREAM",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckResult {
    name: &'static str,
    passed: bool,
    code: Option<i32>,
    detail: Option<String>,
}

impl CheckResult {
    fn pass(name: &'static str) -> Self {
        Self {
            name,
            passed: true,
            code: None,
            detail: None,
        }
    }

    fn fail(name: &'static str, code: i32) -> Self {
        Self {
            name,
            passed: false,
            code: Some(code),
            detail: None,
        }
    }

    fn fail_detail(name: &'static str, code: i32, detail: impl Into<String>) -> Self {
        Self {
            name,
            passed: false,
            code: Some(code),
            detail: Some(detail.into()),
        }
    }
}

fn main() -> ExitCode {
    match env::args().nth(1).as_deref() {
        Some("verify") => match verify() {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::from(1),
            Err(error) => {
                eprintln!("AetherAI verifier error: {error}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("Usage: cargo run -q -p xtask -- verify");
            ExitCode::from(2)
        }
    }
}

fn verify() -> Result<bool, Box<dyn std::error::Error>> {
    let root = repo_root();
    let mut checks = vec![
        run_cargo(&root, "FMT", &["fmt", "--all", "--check"]),
        run_cargo(
            &root,
            "CLIPPY",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        run_cargo(&root, "TEST", &["test", "--workspace"]),
        run_cargo(&root, "BUILD", &["build", "--workspace", "--release"]),
        run_cli_smoke(&root),
        check_release_binary(&root, "DESKTOP_BUILD", "aetherai-desktop"),
        run_shell(&root, "DESKTOP_CONTRACT", "tests/test_desktop_contract.sh"),
        run_cargo(
            &root,
            "STORAGE_MIGRATION",
            &["test", "-p", "aether-storage"],
        ),
        run_cargo(
            &root,
            "UNIFIED_CHAT_WORKSPACE",
            &[
                "test",
                "-p",
                "aetherai-desktop",
                "unified_project_keeps_chat_identity",
            ],
        ),
        run_cargo(&root, "PERMISSION_SCOPE", &["test", "-p", "aether-tools"]),
        run_cargo(
            &root,
            "PROJECT_DISCOVERY",
            &["test", "-p", "aether-workspace"],
        ),
        run_cargo(&root, "PROCESS_AUDIT", &["test", "-p", "aether-tools"]),
        run_cargo(&root, "SYSTEM_CONTRACT", &["test", "-p", "aether-system"]),
        run_shell(&root, "NO_MOCK_RUNTIME", "tests/test_no_mock_runtime.sh"),
        run_cargo(&root, "MODEL_REGISTRY", &["test", "-p", "aether-model-api"]),
        run_cargo(
            &root,
            "NETWORK_POLICY",
            &["test", "-p", "aether-model-runtime"],
        ),
        run_shell(
            &root,
            "LIVE_MODEL_CONTRACT",
            "tests/test_live_model_contract.sh",
        ),
    ];
    checks.extend(run_live_model_checks(&root));

    let report = render_report(&checks);
    let report_path = root.join(REPORT_NAME);
    fs::write(&report_path, &report)?;
    print!("{report}");
    println!("AETHERAI_VERIFY_FILE={}", report_path.display());

    Ok(checks.iter().all(|check| check.passed))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must live one directory below repository root")
        .to_path_buf()
}

fn run_cargo(root: &Path, name: &'static str, args: &[&str]) -> CheckResult {
    println!("==> cargo {}", args.join(" "));
    match Command::new("cargo").current_dir(root).args(args).status() {
        Ok(status) if status.success() => CheckResult::pass(name),
        Ok(status) => CheckResult::fail(name, status.code().unwrap_or(1)),
        Err(error) => {
            eprintln!("failed to start cargo for {name}: {error}");
            CheckResult::fail_detail(name, 127, error.to_string())
        }
    }
}

fn run_shell(root: &Path, name: &'static str, relative: &str) -> CheckResult {
    println!("==> bash {relative}");
    match Command::new("bash")
        .current_dir(root)
        .arg(relative)
        .status()
    {
        Ok(status) if status.success() => CheckResult::pass(name),
        Ok(status) => CheckResult::fail(name, status.code().unwrap_or(1)),
        Err(error) => CheckResult::fail_detail(name, 127, error.to_string()),
    }
}

fn release_binary_path(root: &Path, name: &str) -> PathBuf {
    if cfg!(windows) {
        root.join("target/release").join(format!("{name}.exe"))
    } else {
        root.join("target/release").join(name)
    }
}

fn check_release_binary(root: &Path, name: &'static str, binary: &str) -> CheckResult {
    if release_binary_path(root, binary).is_file() {
        CheckResult::pass(name)
    } else {
        CheckResult::fail_detail(name, 1, format!("missing release binary {binary}"))
    }
}

fn run_cli_smoke(root: &Path) -> CheckResult {
    let binary = release_binary_path(root, "aetherai");
    let smoke_dir = env::temp_dir().join(format!("aetherai-v020-smoke-{}", std::process::id()));
    let _ = fs::remove_dir_all(&smoke_dir);
    let output = Command::new(&binary)
        .current_dir(root)
        .arg("--data-dir")
        .arg(&smoke_dir)
        .arg("--diagnostic")
        .output();
    let _ = fs::remove_dir_all(&smoke_dir);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if output.status.success()
                && stdout.lines().any(|line| line == "AETHERAI_RUNTIME=LIVE")
                && stdout
                    .lines()
                    .any(|line| line == "AETHERAI_SCHEMA_VERSION=2")
            {
                CheckResult::pass("CLI_SMOKE")
            } else {
                eprintln!("CLI diagnostic stdout:\n{stdout}");
                eprintln!(
                    "CLI diagnostic stderr:\n{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                CheckResult::fail("CLI_SMOKE", output.status.code().unwrap_or(1))
            }
        }
        Err(error) => CheckResult::fail_detail("CLI_SMOKE", 127, error.to_string()),
    }
}

fn run_live_model_checks(root: &Path) -> Vec<CheckResult> {
    println!("==> bash scripts/verify-live-model.sh");
    let output = Command::new("bash")
        .current_dir(root)
        .arg("scripts/verify-live-model.sh")
        .output();
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            print!("{stdout}");
            if !stderr.is_empty() {
                eprint!("{stderr}");
            }
            let code = output.status.code().unwrap_or(1);
            LIVE_CHECKS
                .into_iter()
                .map(|name| {
                    let expected = format!("AETHERAI_{name}=PASS");
                    if output.status.success() && stdout.lines().any(|line| line == expected) {
                        CheckResult::pass(name)
                    } else {
                        let fail_prefix = format!("AETHERAI_{name}=FAIL:");
                        let detail = stdout
                            .lines()
                            .find_map(|line| line.strip_prefix(&fail_prefix).map(str::to_string))
                            .unwrap_or_else(|| live_failure_detail(&stdout, &stderr));
                        CheckResult::fail_detail(name, code, detail)
                    }
                })
                .collect()
        }
        Err(error) => LIVE_CHECKS
            .into_iter()
            .map(|name| CheckResult::fail_detail(name, 127, error.to_string()))
            .collect(),
    }
}

fn live_failure_detail(stdout: &str, stderr: &str) -> String {
    stdout
        .lines()
        .chain(stderr.lines())
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("AETHERAI_"))
        .map(|line| line.strip_prefix("Error: ").unwrap_or(line).to_string())
        .unwrap_or_else(|| "LIVE_GATE_FAILED".to_string())
}

fn render_report(checks: &[CheckResult]) -> String {
    let mut report = format!("AETHERAI_VERSION={VERSION}\n");
    for check in checks {
        if check.passed {
            report.push_str(&format!("AETHERAI_{}=PASS\n", check.name));
        } else if let Some(detail) = &check.detail {
            report.push_str(&format!("AETHERAI_{}=FAIL:{}\n", check.name, detail));
        } else {
            report.push_str(&format!(
                "AETHERAI_{}=FAIL:{}\n",
                check.name,
                check.code.unwrap_or(1)
            ));
        }
    }

    let non_live_pass = checks
        .iter()
        .filter(|check| !LIVE_CHECKS.contains(&check.name))
        .all(|check| check.passed);
    report.push_str(if non_live_pass {
        "AETHERAI_V0_2_2_BUILD_VERIFY=PASS\n"
    } else {
        "AETHERAI_V0_2_2_BUILD_VERIFY=FAIL\n"
    });
    report.push_str(if checks.iter().all(|check| check.passed) {
        "AETHERAI_V0_2_2_VERIFY=PASS\n"
    } else {
        "AETHERAI_V0_2_2_VERIFY=FAIL\n"
    });
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_ends_with_v0_2_2_pass_endpoint() {
        let mut checks = vec![
            CheckResult::pass("FMT"),
            CheckResult::pass("NO_MOCK_RUNTIME"),
        ];
        checks.extend(LIVE_CHECKS.into_iter().map(CheckResult::pass));
        let report = render_report(&checks);
        assert!(report.contains("AETHERAI_V0_2_2_BUILD_VERIFY=PASS\n"));
        assert!(report.ends_with("AETHERAI_V0_2_2_VERIFY=PASS\n"));
    }

    #[test]
    fn live_failure_detail_preserves_provider_error() {
        let stdout = concat!(
            "AETHERAI_MODEL_SERVICE=PASS\n",
            "Error: model provider error: runtime did not open loopback endpoint\n"
        );
        assert_eq!(
            live_failure_detail(stdout, ""),
            "model provider error: runtime did not open loopback endpoint"
        );
    }

    #[test]
    fn missing_live_model_keeps_build_pass_but_full_release_fail() {
        let checks = [
            CheckResult::pass("FMT"),
            CheckResult::fail_detail("MODEL_SERVICE", 3, "NO_TEST_MODEL"),
        ];
        let report = render_report(&checks);
        assert!(report.contains("AETHERAI_V0_2_2_BUILD_VERIFY=PASS\n"));
        assert!(report.ends_with("AETHERAI_V0_2_2_VERIFY=FAIL\n"));
    }
}
