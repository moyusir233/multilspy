use std::fs::File;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use rayon::prelude::*;
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Helpers — mirrors multilspy-rust/tests/client_tests.rs conventions
// ---------------------------------------------------------------------------

fn rust_analyzer_available() -> bool {
    Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .is_ok()
}

fn cli_binary_path() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_ra-lsp"));
    assert!(path.exists(), "multilspy binary must exist at {:?}", path);
    path
}

fn test_project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../multilspy-rust/test-rust-project")
        .canonicalize()
        .expect("test-rust-project must exist")
}

fn initialize_params_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../multilspy-rust/ra_initialize_params.json")
        .canonicalize()
        .expect("ra_initialize_params.json must exist")
}

fn file_uri() -> String {
    let main_rs = test_project_root()
        .join("src/main.rs")
        .canonicalize()
        .expect("test-rust-project/src/main.rs must exist");
    format!("file://{}", main_rs.display())
}

fn workspace_args() -> Vec<String> {
    vec![
        "--workspace".to_string(),
        test_project_root().display().to_string(),
        "--initialize-params".to_string(),
        initialize_params_path().display().to_string(),
        "--wait-work-done-progress-create-max-time".to_string(),
        "5".to_string(),
    ]
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "multilspy-cli-integration-{}-{}-{}",
        name,
        std::process::id(),
        timestamp
    ));
    std::fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

struct CliOutput {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

fn run_cli(args: &[&str]) -> CliOutput {
    let ws = workspace_args();
    let mut cmd = Command::new(cli_binary_path());
    for a in &ws {
        cmd.arg(a);
    }
    for a in args {
        cmd.arg(a);
    }
    let output = cmd.output().expect("failed to execute multilspy binary");
    CliOutput {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

fn run_cli_with_options(
    args: &[&str],
    include_workspace_args: bool,
    cwd: Option<&std::path::Path>,
    envs: &[(&str, &str)],
) -> CliOutput {
    let ws = workspace_args();
    let mut cmd = Command::new(cli_binary_path());
    if include_workspace_args {
        for a in &ws {
            cmd.arg(a);
        }
    }
    for a in args {
        cmd.arg(a);
    }
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    for (key, value) in envs {
        cmd.env(key, value);
    }
    let output = cmd.output().expect("failed to execute multilspy binary");
    CliOutput {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

fn run_cli_raw(args: &[&str]) -> CliOutput {
    run_cli_with_options(args, false, None, &[])
}

fn run_cli_raw_with_env(args: &[&str], envs: &[(&str, &str)]) -> CliOutput {
    run_cli_with_options(args, false, None, envs)
}

fn run_cli_in_dir(args: &[&str], cwd: &std::path::Path) -> CliOutput {
    run_cli_with_options(args, true, Some(cwd), &[])
}

fn parse_ipc_response(stdout: &str) -> Value {
    serde_json::from_str(stdout.trim()).expect("stdout should be valid JSON IpcResponse")
}

fn assert_success_result(stdout: &str) -> Value {
    let resp = parse_ipc_response(stdout);
    assert!(
        resp.get("error").is_none(),
        "expected success but got error: {}",
        stdout
    );
    resp.get("result")
        .expect("success response should have 'result' field")
        .clone()
}

#[allow(dead_code)]
fn assert_error_response(stdout: &str) -> Value {
    let resp = parse_ipc_response(stdout);
    assert!(
        resp.get("error").is_some(),
        "expected error but got success: {}",
        stdout
    );
    resp.get("error").unwrap().clone()
}

fn stop_daemon() {
    let status_out = run_cli(&["status"]);
    let pid = serde_json::from_str::<Value>(status_out.stdout.trim())
        .ok()
        .and_then(|v| v["result"]["pid"].as_u64())
        .map(|p| p as i32);

    let _ = run_cli(&["stop"]);

    if let Some(pid) = pid {
        for _ in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            let alive = unsafe { libc::kill(pid, 0) == 0 };
            if !alive {
                std::thread::sleep(std::time::Duration::from_millis(500));
                return;
            }
        }
    }
    std::thread::sleep(std::time::Duration::from_secs(2));
}

// ---------------------------------------------------------------------------
// File-lock based suite serialization
//
// The CLI integration helpers are executed by a single top-level test suite
// that holds one EXCLUSIVE lock for the daemon-backed portion of the run.
// This keeps one daemon alive across helper calls and guarantees shutdown when
// the suite drops the guard.
// ---------------------------------------------------------------------------

fn daemon_test_lock_path() -> PathBuf {
    std::env::temp_dir()
        .join("multilspy-cli")
        .join("_test_daemon.lock")
}

struct DaemonTestGuard {
    file: Option<File>,
}

fn acquire_exclusive_daemon_lock() -> DaemonTestGuard {
    std::fs::create_dir_all(daemon_test_lock_path().parent().unwrap()).ok();
    let file = File::options()
        .create(true)
        .write(true)
        .truncate(false)
        .open(daemon_test_lock_path())
        .expect("failed to open test lock file");

    use std::os::unix::io::AsRawFd;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    assert_eq!(rc, 0, "failed to acquire exclusive test lock");
    DaemonTestGuard { file: Some(file) }
}

impl Drop for DaemonTestGuard {
    fn drop(&mut self) {
        drop(self.file.take());
        stop_daemon();
    }
}

type TestFn = fn();

fn run_in_parallel(tests: &[TestFn]) {
    if tests.is_empty() {
        return;
    }

    // Cap fan-out so concurrent CLI processes do not overwhelm one daemon.
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4)
        .min(tests.len());

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .expect("failed to build rayon thread pool");

    pool.install(|| {
        tests.par_iter().for_each(|test| test());
    });
}

// ---------------------------------------------------------------------------
// CLI help and version — no daemon required, no lock needed
// ---------------------------------------------------------------------------

fn test_help_flag() {
    let out = run_cli_raw(&["--help"]);
    assert!(out.status.success());
    assert!(out.stdout.contains("LSP CLI for AI agents"));
    assert!(
        out.stdout
            .contains("--wait-work-done-progress-create-max-time")
    );
    assert!(out.stdout.contains("RA_LSP_INIT_PARAMS_PATH"));
    assert!(out.stdout.contains("--relative-path"));
    assert!(out.stdout.contains("JSON Output"));
    assert!(out.stdout.contains("definition"));
    assert!(out.stdout.contains("type-definition"));
    assert!(out.stdout.contains("implementation"));
    assert!(out.stdout.contains("references"));
    assert!(out.stdout.contains("document-symbols"));
    assert!(out.stdout.contains("workspace-symbols"));
    assert!(out.stdout.contains("workspace-symbol-resolve"));
    assert!(out.stdout.contains("incoming-calls"));
    assert!(out.stdout.contains("outgoing-calls"));
    assert!(out.stdout.contains("incoming-calls-recursive"));
    assert!(out.stdout.contains("outgoing-calls-recursive"));
    assert!(out.stdout.contains("analyze-trait-impl-deps-graph"));
    assert!(out.stdout.contains("status"));
    assert!(out.stdout.contains("stop"));
}

fn test_version_flag() {
    let out = run_cli_raw(&["--version"]);
    assert!(out.status.success());
    assert!(out.stdout.contains("multilspy"));
}

fn test_subcommand_help_definition() {
    let out = run_cli_raw(&["definition", "--help"]);
    assert!(out.status.success());
    assert!(out.stdout.contains("--uri"));
    assert!(out.stdout.contains("--relative-path"));
    assert!(out.stdout.contains("--line"));
    assert!(out.stdout.contains("--character"));
    assert!(out.stdout.contains("RA_LSP_INIT_PARAMS_PATH"));
    assert!(out.stdout.contains("JSON Output"));
    assert!(out.stdout.contains("Location[]"));
}

fn test_subcommand_help_references() {
    let out = run_cli_raw(&["references", "--help"]);
    assert!(out.status.success());
    assert!(out.stdout.contains("--include-declaration"));
}

fn test_subcommand_help_workspace_symbols() {
    let out = run_cli_raw(&["workspace-symbols", "--help"]);
    assert!(out.status.success());
    assert!(out.stdout.contains("--query"));
    assert!(out.stdout.contains("--limit"));
    assert!(
        out.stdout
            .contains("WorkspaceSymbol[] | SymbolInformation[]")
    );
}

fn test_subcommand_help_workspace_symbol_resolve() {
    let out = run_cli_raw(&["workspace-symbol-resolve", "--help"]);
    assert!(out.status.success());
    assert!(out.stdout.contains("--symbol-json"));
    assert!(out.stdout.contains("--symbol-file"));
    assert!(out.stdout.contains("WorkspaceSymbol | null"));
}

fn test_subcommand_help_incoming_calls_recursive() {
    let out = run_cli_raw(&["incoming-calls-recursive", "--help"]);
    assert!(out.status.success());
    assert!(out.stdout.contains("--max-depth"));
    assert!(out.stdout.contains("--relative-path"));
    assert!(
        out.stdout
            .contains("[[CallHierarchyItem, CallHierarchyIncomingCall[]], ...]")
    );
}

fn test_subcommand_help_status() {
    let out = run_cli_raw(&["status", "--help"]);
    assert!(out.status.success());
    assert!(out.stdout.contains("RA_LSP_INIT_PARAMS_PATH"));
    assert!(out.stdout.contains("JSON Output"));
    assert!(out.stdout.contains("\"workspace\": string"));
    assert!(
        out.stdout
            .contains("--relative-path` is not applicable to `status`")
    );
}

fn test_subcommand_help_analyze_trait_impl_deps_graph() {
    let out = run_cli_raw(&["analyze-trait-impl-deps-graph", "--help"]);
    assert!(out.status.success());
    assert!(out.stdout.contains("TRAIT... TARGET_DIR"));
    assert!(out.stdout.contains("JSON Output"));
}

// ---------------------------------------------------------------------------
// Missing / invalid argument handling — clap errors (no daemon required)
// ---------------------------------------------------------------------------

fn test_missing_subcommand_shows_help() {
    let out = run_cli_raw(&[]);
    assert!(!out.status.success());
    assert!(
        out.stderr.contains("Usage") || out.stdout.contains("Usage"),
        "should show usage info"
    );
}

fn test_definition_missing_uri() {
    let out = run_cli_raw(&["definition", "--line", "0", "--character", "0"]);
    assert!(!out.status.success());
}

fn test_definition_missing_line() {
    let out = run_cli_raw(&["definition", "--uri", "file:///test.rs", "--character", "0"]);
    assert!(!out.status.success());
}

fn test_definition_missing_character() {
    let out = run_cli_raw(&["definition", "--uri", "file:///test.rs", "--line", "0"]);
    assert!(!out.status.success());
}

fn test_unknown_subcommand() {
    let out = run_cli_raw(&["nonexistent-command"]);
    assert!(!out.status.success());
}

fn test_analyze_trait_impl_deps_graph_missing_target_dir() {
    let out = run_cli_raw(&["analyze-trait-impl-deps-graph", "Greeter"]);
    assert!(!out.status.success());
    assert!(
        out.stderr.contains("Usage") || out.stdout.contains("Usage"),
        "should show usage info"
    );
}

fn test_definition_conflicting_uri_and_relative_path() {
    let out = run_cli_raw(&[
        "definition",
        "--uri",
        "file:///test.rs",
        "--relative-path",
        "src/main.rs",
        "--line",
        "0",
        "--character",
        "0",
    ]);
    assert!(!out.status.success());
    assert!(out.stderr.contains("--relative-path"));
    assert!(out.stderr.contains("--uri"));
}

fn test_invalid_env_initialize_params_file_returns_json_error() {
    let missing_dir = unique_temp_dir("missing-init");
    let missing_path = missing_dir.join("missing.json");
    let workspace = test_project_root();
    let workspace_arg = workspace.display().to_string();
    let missing_path_arg = missing_path.display().to_string();

    let out = run_cli_raw_with_env(
        &["--workspace", &workspace_arg, "status"],
        &[("RA_LSP_INIT_PARAMS_PATH", &missing_path_arg)],
    );
    assert!(!out.status.success());
    let error = assert_error_response(&out.stdout);
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("file does not exist")
    );

    std::fs::remove_dir_all(missing_dir).unwrap();
}

fn test_invalid_env_initialize_params_json_returns_json_error() {
    let dir = unique_temp_dir("invalid-init-json");
    let invalid_path = dir.join("invalid.json");
    std::fs::write(&invalid_path, "{invalid json").unwrap();
    let workspace = test_project_root();
    let workspace_arg = workspace.display().to_string();
    let invalid_path_arg = invalid_path.display().to_string();

    let out = run_cli_raw_with_env(
        &["--workspace", &workspace_arg, "status"],
        &[("RA_LSP_INIT_PARAMS_PATH", &invalid_path_arg)],
    );
    assert!(!out.status.success());
    let error = assert_error_response(&out.stdout);
    assert!(error["message"].as_str().unwrap().contains("invalid JSON"));

    std::fs::remove_dir_all(dir).unwrap();
}

// ---------------------------------------------------------------------------
// JSON output structure validation — shared daemon lock
// ---------------------------------------------------------------------------

fn test_output_is_valid_json() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "definition",
        "--uri",
        &uri,
        "--line",
        "24",
        "--character",
        "5",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let parsed: Result<Value, _> = serde_json::from_str(out.stdout.trim());
    assert!(
        parsed.is_ok(),
        "stdout should be valid JSON: {}",
        out.stdout
    );
}

fn test_success_response_has_result_field() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "definition",
        "--uri",
        &uri,
        "--line",
        "24",
        "--character",
        "5",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let resp = parse_ipc_response(&out.stdout);
    assert!(
        resp.get("result").is_some(),
        "response should have 'result'"
    );
    assert!(
        resp.get("error").is_none(),
        "success response should not have 'error'"
    );
}

// ---------------------------------------------------------------------------
// definition command — shared daemon lock
// ---------------------------------------------------------------------------

fn test_definition_of_function_call() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "definition",
        "--uri",
        &uri,
        "--line",
        "35",
        "--character",
        "12",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().expect("result should be an array");
    assert!(!locations.is_empty(), "should return at least one location");
    let loc = &locations[0];
    assert!(
        loc["uri"].as_str().unwrap().ends_with("main.rs"),
        "location should be in main.rs"
    );
    assert_eq!(loc["range"]["start"]["line"], 24);
}

fn test_definition_with_relative_path() {
    if !rust_analyzer_available() {
        return;
    }
    let workspace = test_project_root();
    let out = run_cli_in_dir(
        &[
            "definition",
            "--relative-path",
            "src/main.rs",
            "--line",
            "35",
            "--character",
            "12",
        ],
        &workspace,
    );
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().expect("result should be an array");
    assert!(!locations.is_empty(), "should return at least one location");
    assert!(
        locations[0]["uri"].as_str().unwrap().ends_with("main.rs"),
        "location should be in main.rs"
    );
    assert_eq!(locations[0]["range"]["start"]["line"], 24);
}

fn test_definition_of_trait_method_call() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "definition",
        "--uri",
        &uri,
        "--line",
        "31",
        "--character",
        "6",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().unwrap();
    assert!(!locations.is_empty());
    assert_eq!(locations[0]["range"]["start"]["line"], 1);
}

fn test_definition_of_struct_field() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "definition",
        "--uri",
        &uri,
        "--line",
        "10",
        "--character",
        "35",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().unwrap();
    assert!(!locations.is_empty());
    assert_eq!(locations[0]["range"]["start"]["line"], 5);
}

fn test_definition_at_definition_site_points_to_self() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "definition",
        "--uri",
        &uri,
        "--line",
        "24",
        "--character",
        "5",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().unwrap();
    assert!(!locations.is_empty());
    assert_eq!(locations[0]["range"]["start"]["line"], 24);
}

// ---------------------------------------------------------------------------
// type-definition command — shared daemon lock
// ---------------------------------------------------------------------------

fn test_type_definition_of_variable() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "type-definition",
        "--uri",
        &uri,
        "--line",
        "35",
        "--character",
        "8",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().unwrap();
    assert!(!locations.is_empty());
    assert!(locations[0]["uri"].as_str().unwrap().ends_with("main.rs"));
    assert_eq!(locations[0]["range"]["start"]["line"], 4);
}

fn test_type_definition_of_function_return() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "type-definition",
        "--uri",
        &uri,
        "--line",
        "40",
        "--character",
        "8",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().unwrap();
    assert!(!locations.is_empty());
}

// ---------------------------------------------------------------------------
// implementation command — shared daemon lock
// ---------------------------------------------------------------------------

fn test_implementation_of_trait() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "implementation",
        "--uri",
        &uri,
        "--line",
        "0",
        "--character",
        "6",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().unwrap();
    assert!(
        locations.len() >= 2,
        "Greeter should have >=2 implementations, got {}",
        locations.len()
    );
    let lines: Vec<u64> = locations
        .iter()
        .map(|l| l["range"]["start"]["line"].as_u64().unwrap())
        .collect();
    assert!(lines.contains(&8), "should contain impl at line 8 (Hello)");
    assert!(
        lines.contains(&18),
        "should contain impl at line 18 (Goodbye)"
    );
}

fn test_implementation_of_trait_method() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "implementation",
        "--uri",
        &uri,
        "--line",
        "1",
        "--character",
        "7",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().unwrap();
    assert!(locations.len() >= 2);
    let lines: Vec<u64> = locations
        .iter()
        .map(|l| l["range"]["start"]["line"].as_u64().unwrap())
        .collect();
    assert!(lines.contains(&9));
    assert!(lines.contains(&19));
}

fn test_implementation_of_struct() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "implementation",
        "--uri",
        &uri,
        "--line",
        "4",
        "--character",
        "7",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().unwrap();
    assert!(!locations.is_empty());
    let lines: Vec<u64> = locations
        .iter()
        .map(|l| l["range"]["start"]["line"].as_u64().unwrap())
        .collect();
    assert!(lines.contains(&8));
}

// ---------------------------------------------------------------------------
// references command — shared daemon lock
// ---------------------------------------------------------------------------

fn test_references_include_declaration() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "references",
        "--uri",
        &uri,
        "--line",
        "24",
        "--character",
        "5",
        "--include-declaration",
        "true",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().unwrap();
    assert!(
        locations.len() >= 2,
        "should return >=2 (declaration + usage), got {}",
        locations.len()
    );
    let lines: Vec<u64> = locations
        .iter()
        .map(|l| l["range"]["start"]["line"].as_u64().unwrap())
        .collect();
    assert!(lines.contains(&24), "should include declaration at line 24");
    assert!(lines.contains(&35), "should include usage at line 35");
}

fn test_references_exclude_declaration() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "references",
        "--uri",
        &uri,
        "--line",
        "24",
        "--character",
        "5",
        "--include-declaration",
        "false",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().unwrap();
    assert!(!locations.is_empty());
    let lines: Vec<u64> = locations
        .iter()
        .map(|l| l["range"]["start"]["line"].as_u64().unwrap())
        .collect();
    assert!(lines.contains(&35), "should include usage at line 35");
}

fn test_references_of_trait_method() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "references",
        "--uri",
        &uri,
        "--line",
        "1",
        "--character",
        "7",
        "--include-declaration",
        "true",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().unwrap();
    assert!(
        locations.len() >= 2,
        "trait method should have multiple references, got {}",
        locations.len()
    );
}

// ---------------------------------------------------------------------------
// document-symbols command — shared daemon lock
// ---------------------------------------------------------------------------

fn test_document_symbols() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&["document-symbols", "--uri", &uri]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let symbols = result.as_array().unwrap();
    assert!(!symbols.is_empty(), "should return symbols");

    let names: Vec<&str> = symbols
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Greeter"), "should contain Greeter trait");
    assert!(names.contains(&"Hello"), "should contain Hello struct");
    assert!(names.contains(&"Goodbye"), "should contain Goodbye struct");
    assert!(
        names.contains(&"create_hello"),
        "should contain create_hello"
    );
    assert!(names.contains(&"call_greet"), "should contain call_greet");
    assert!(names.contains(&"helper"), "should contain helper");
    assert!(names.contains(&"main"), "should contain main");
}

fn test_document_symbols_kinds() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&["document-symbols", "--uri", &uri]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let symbols = result.as_array().unwrap();

    let greeter = symbols.iter().find(|s| s["name"] == "Greeter").unwrap();
    assert_eq!(greeter["kind"], 11, "Greeter should be Interface (11)");

    let hello = symbols.iter().find(|s| s["name"] == "Hello").unwrap();
    assert_eq!(hello["kind"], 23, "Hello should be Struct (23)");

    let main_fn = symbols.iter().find(|s| s["name"] == "main").unwrap();
    assert_eq!(main_fn["kind"], 12, "main should be Function (12)");
}

fn test_document_symbols_children() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&["document-symbols", "--uri", &uri]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let symbols = result.as_array().unwrap();

    let hello = symbols.iter().find(|s| s["name"] == "Hello").unwrap();
    let children = hello["children"]
        .as_array()
        .expect("Hello should have children");
    let child_names: Vec<&str> = children
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert!(
        child_names.contains(&"name"),
        "Hello should have 'name' child"
    );
}

// ---------------------------------------------------------------------------
// workspace-symbols / workspace-symbol-resolve commands — shared daemon lock
// ---------------------------------------------------------------------------

fn test_workspace_symbols_query() {
    if !rust_analyzer_available() {
        return;
    }
    let out = run_cli(&["workspace-symbols", "--query", "helper"]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let symbols = result.as_array().expect("result should be an array");
    assert!(!symbols.is_empty(), "should return workspace symbols");

    let helper = symbols
        .iter()
        .find(|symbol| symbol["name"] == "helper")
        .expect("should contain helper symbol");
    assert_eq!(helper["kind"], json!(12));
}

fn test_workspace_symbols_limit() {
    if !rust_analyzer_available() {
        return;
    }
    let out = run_cli(&["workspace-symbols", "--query", "e", "--limit", "1"]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let symbols = result.as_array().expect("result should be an array");
    assert!(symbols.len() <= 1, "limit should cap result length");
}

fn test_workspace_symbols_blank_query_returns_json_error() {
    if !rust_analyzer_available() {
        return;
    }
    let out = run_cli(&["workspace-symbols", "--query", "   "]);
    assert!(!out.status.success());
    let error = assert_error_response(&out.stdout);
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("must not be empty or whitespace-only")
    );
}

fn test_workspace_symbol_resolve() {
    if !rust_analyzer_available() {
        return;
    }

    let search = run_cli(&["workspace-symbols", "--query", "helper"]);
    assert!(search.status.success(), "stderr: {}", search.stderr);
    let result = assert_success_result(&search.stdout);
    let symbols = result.as_array().expect("result should be an array");
    let helper = symbols
        .iter()
        .find(|symbol| symbol["name"] == "helper")
        .expect("should contain helper symbol");
    let helper_json = serde_json::to_string(helper).unwrap();

    let resolve = run_cli(&["workspace-symbol-resolve", "--symbol-json", &helper_json]);
    if resolve.status.success() {
        let resolved = assert_success_result(&resolve.stdout);
        assert_eq!(resolved["name"], json!("helper"));
        assert!(
            resolved["location"]["uri"]
                .as_str()
                .unwrap()
                .ends_with("main.rs")
        );
        assert_eq!(resolved["location"]["range"]["start"]["line"], json!(34));
    } else {
        let error = assert_error_response(&resolve.stdout);
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("does not advertise support for workspaceSymbol/resolve"),
            "unexpected resolve error: {}",
            resolve.stdout
        );
    }
}

// ---------------------------------------------------------------------------
// incoming-calls command — shared daemon lock
// ---------------------------------------------------------------------------

fn test_incoming_calls() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "incoming-calls",
        "--uri",
        &uri,
        "--line",
        "34",
        "--character",
        "5",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let calls = result.as_array().unwrap();
    assert!(!calls.is_empty(), "helper should have incoming callers");

    let caller_names: Vec<&str> = calls
        .iter()
        .map(|c| c["from"]["name"].as_str().unwrap())
        .collect();
    assert!(caller_names.contains(&"main"), "main should call helper");
}

fn test_incoming_calls_of_leaf_function() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "incoming-calls",
        "--uri",
        &uri,
        "--line",
        "39",
        "--character",
        "5",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let calls = result.as_array().unwrap();
    assert!(calls.is_empty(), "main should have no incoming calls");
}

// ---------------------------------------------------------------------------
// outgoing-calls command — shared daemon lock
// ---------------------------------------------------------------------------

fn test_outgoing_calls() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "outgoing-calls",
        "--uri",
        &uri,
        "--line",
        "34",
        "--character",
        "5",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let calls = result.as_array().unwrap();
    assert!(!calls.is_empty());

    let callee_names: Vec<&str> = calls
        .iter()
        .map(|c| c["to"]["name"].as_str().unwrap())
        .collect();
    assert!(callee_names.contains(&"create_hello"));
    assert!(callee_names.contains(&"call_greet"));
}

fn test_outgoing_calls_of_leaf() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "outgoing-calls",
        "--uri",
        &uri,
        "--line",
        "24",
        "--character",
        "5",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let calls = result.as_array().unwrap();
    assert!(
        !calls.is_empty(),
        "create_hello should have at least one outgoing call (to_string)"
    );
}

// ---------------------------------------------------------------------------
// incoming-calls-recursive command — shared daemon lock
// ---------------------------------------------------------------------------

fn test_incoming_calls_recursive() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "incoming-calls-recursive",
        "--uri",
        &uri,
        "--line",
        "24",
        "--character",
        "5",
        "--max-depth",
        "10",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let items = result.as_array().unwrap();
    assert!(!items.is_empty());

    let all_caller_names: Vec<&str> = items
        .iter()
        .flat_map(|pair| {
            pair.as_array()
                .unwrap()
                .get(1)
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["from"]["name"].as_str().unwrap())
        })
        .collect();
    assert!(
        all_caller_names.contains(&"helper"),
        "should find helper as caller of create_hello"
    );
    assert!(
        all_caller_names.contains(&"main"),
        "should find main as transitive caller"
    );
}

fn test_incoming_calls_recursive_with_depth_limit() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "incoming-calls-recursive",
        "--uri",
        &uri,
        "--line",
        "24",
        "--character",
        "5",
        "--max-depth",
        "1",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let items = result.as_array().unwrap();
    assert!(!items.is_empty());

    let direct_callers: Vec<&str> = items
        .iter()
        .flat_map(|pair| {
            pair.as_array()
                .unwrap()
                .get(1)
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["from"]["name"].as_str().unwrap())
        })
        .collect();
    assert!(direct_callers.contains(&"helper"));
}

fn test_incoming_calls_recursive_without_max_depth() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "incoming-calls-recursive",
        "--uri",
        &uri,
        "--line",
        "24",
        "--character",
        "5",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let items = result.as_array().unwrap();
    assert!(!items.is_empty());
}

// ---------------------------------------------------------------------------
// outgoing-calls-recursive command — shared daemon lock
// ---------------------------------------------------------------------------

fn test_outgoing_calls_recursive() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "outgoing-calls-recursive",
        "--uri",
        &uri,
        "--line",
        "39",
        "--character",
        "5",
        "--max-depth",
        "10",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let items = result.as_array().unwrap();
    assert!(!items.is_empty());

    let all_callee_names: Vec<&str> = items
        .iter()
        .flat_map(|pair| {
            pair.as_array()
                .unwrap()
                .get(1)
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["to"]["name"].as_str().unwrap())
        })
        .collect();
    assert!(
        all_callee_names.contains(&"helper"),
        "should find helper as callee of main"
    );
    assert!(
        all_callee_names.contains(&"create_hello"),
        "should find create_hello as transitive callee"
    );
}

fn test_outgoing_calls_recursive_with_depth_limit() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "outgoing-calls-recursive",
        "--uri",
        &uri,
        "--line",
        "39",
        "--character",
        "5",
        "--max-depth",
        "1",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let items = result.as_array().unwrap();
    assert!(!items.is_empty());

    let direct_callees: Vec<&str> = items
        .iter()
        .flat_map(|pair| {
            pair.as_array()
                .unwrap()
                .get(1)
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["to"]["name"].as_str().unwrap())
        })
        .collect();
    assert!(direct_callees.contains(&"helper"));
}

// ---------------------------------------------------------------------------
// analyze-trait-impl-deps-graph command — shared daemon lock
// ---------------------------------------------------------------------------

fn src_dir_uri() -> String {
    let src_dir = test_project_root()
        .join("src")
        .canonicalize()
        .expect("test-rust-project/src must exist");
    format!("file://{}", src_dir.display())
}

fn test_analyze_trait_impl_deps_graph_invalid_target_dir_returns_json_error() {
    if !rust_analyzer_available() {
        return;
    }

    let missing_parent = unique_temp_dir("missing-analyze-dir");
    let missing_dir = missing_parent.join("does-not-exist");
    let out = run_cli(&[
        "analyze-trait-impl-deps-graph",
        "Greeter",
        &missing_dir.display().to_string(),
    ]);
    assert!(!out.status.success());
    let error = assert_error_response(&out.stdout);
    assert_eq!(error["code"], json!(-32602));

    let _ = std::fs::remove_dir_all(missing_parent);
}

fn test_analyze_trait_impl_deps_graph_trait_not_found_returns_empty() {
    if !rust_analyzer_available() {
        return;
    }

    let out = run_cli(&[
        "analyze-trait-impl-deps-graph",
        "DefinitelyNotATrait",
        &src_dir_uri(),
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let items = result.as_array().unwrap();
    assert!(items.is_empty());
}

fn test_analyze_trait_impl_deps_graph_empty_directory_returns_empty() {
    if !rust_analyzer_available() {
        return;
    }

    let empty_dir = test_project_root().join("src").join("_empty_analyze_dir");
    std::fs::create_dir_all(&empty_dir).expect("failed to create empty directory");
    let empty_uri = format!(
        "file://{}",
        empty_dir
            .canonicalize()
            .expect("empty directory must exist")
            .display()
    );

    let out = run_cli(&["analyze-trait-impl-deps-graph", "Greeter", &empty_uri]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let items = result.as_array().unwrap();
    assert!(items.is_empty());

    let _ = std::fs::remove_dir_all(empty_dir);
}

fn test_analyze_trait_impl_deps_graph_trait_with_impl_but_no_functions_returns_empty() {
    if !rust_analyzer_available() {
        return;
    }

    let out = run_cli(&["analyze-trait-impl-deps-graph", "Marker", &src_dir_uri()]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let items = result.as_array().unwrap();
    assert!(items.is_empty());
}

fn test_analyze_trait_impl_deps_graph_builds_dependency_edges_within_target_set() {
    if !rust_analyzer_available() {
        return;
    }

    let out = run_cli(&["analyze-trait-impl-deps-graph", "Chain", &src_dir_uri()]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let items = result.as_array().unwrap();
    assert!(!items.is_empty());

    let mut chain_a = None;
    let mut chain_b = None;
    for item in items {
        if item["trait_name"] == json!("Chain")
            && item["file_uri"].as_str().unwrap().ends_with("main.rs")
        {
            let name = item["function_name"].as_str().unwrap_or("");
            if name.ends_with("a") {
                chain_a = Some(item.clone());
            } else if name.ends_with("b") {
                chain_b = Some(item.clone());
            }
        }
    }

    let chain_a = chain_a.expect("should include a");
    let chain_b = chain_b.expect("should include b");

    let expected_dep = json!({
        "trait_name": chain_b["trait_name"].as_str().unwrap(),
        "file_uri": chain_b["file_uri"].as_str().unwrap(),
        "function_name": chain_b["function_name"].as_str().unwrap(),
    });

    let deps = chain_a["dependencies"].as_array().unwrap();
    assert!(
        deps.iter().any(|d| d == &expected_dep),
        "expected Chain::a to depend on Chain::b (missing dependency: {})",
        expected_dep
    );
}

fn test_analyze_trait_impl_deps_graph_multiple_traits_include_trait_name_field() {
    if !rust_analyzer_available() {
        return;
    }

    let out = run_cli(&[
        "analyze-trait-impl-deps-graph",
        "Greeter",
        "Chain",
        &src_dir_uri(),
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let items = result.as_array().unwrap();
    assert!(!items.is_empty());
    assert!(items.iter().any(|i| i["trait_name"] == json!("Greeter")));
    assert!(items.iter().any(|i| i["trait_name"] == json!("Chain")));
}

// ---------------------------------------------------------------------------
// Daemon lifecycle: status, stop, auto-spawn — EXCLUSIVE daemon lock
// ---------------------------------------------------------------------------

fn test_status_command() {
    if !rust_analyzer_available() {
        return;
    }
    let out = run_cli(&["status"]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    assert!(
        result.get("workspace").is_some(),
        "status should report workspace"
    );
    assert!(result.get("pid").is_some(), "status should report pid");
    assert!(result.get("port").is_some(), "status should report port");
    assert!(
        result.get("uptime_secs").is_some(),
        "status should report uptime_secs"
    );
}

fn test_subsequent_command_faster_than_first() {
    if !rust_analyzer_available() {
        return;
    }
    stop_daemon();

    let uri = file_uri();
    let args = [
        "definition",
        "--uri",
        &uri,
        "--line",
        "24",
        "--character",
        "5",
    ];

    let start_first = Instant::now();
    let first = run_cli(&args);
    let first_duration = start_first.elapsed();
    assert!(first.status.success(), "first run stderr: {}", first.stderr);

    let start_second = Instant::now();
    let second = run_cli(&args);
    let second_duration = start_second.elapsed();
    assert!(
        second.status.success(),
        "second run stderr: {}",
        second.stderr
    );

    assert_success_result(&second.stdout);
    assert!(
        second_duration < first_duration,
        "second run ({:?}) should be faster than first ({:?}) due to daemon reuse",
        second_duration,
        first_duration
    );
}

// ---------------------------------------------------------------------------
// Edge cases — shared daemon lock
// ---------------------------------------------------------------------------

fn test_definition_at_whitespace_returns_empty() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "definition",
        "--uri",
        &uri,
        "--line",
        "3",
        "--character",
        "0",
    ]);
    assert!(out.status.success(), "stderr: {}", out.stderr);
    let result = assert_success_result(&out.stdout);
    let locations = result.as_array().unwrap();
    assert!(
        locations.is_empty(),
        "definition on blank line should be empty"
    );
}

fn test_definition_out_of_range_position() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "definition",
        "--uri",
        &uri,
        "--line",
        "9999",
        "--character",
        "9999",
    ]);
    if out.status.success() {
        let result = assert_success_result(&out.stdout);
        let locations = result.as_array().unwrap();
        assert!(locations.is_empty());
    }
}

fn test_definition_with_invalid_uri() {
    if !rust_analyzer_available() {
        return;
    }
    let out = run_cli(&[
        "definition",
        "--uri",
        "file:///nonexistent/file.rs",
        "--line",
        "0",
        "--character",
        "0",
    ]);
    if out.status.success() {
        let result = assert_success_result(&out.stdout);
        let locations = result.as_array().unwrap();
        assert!(locations.is_empty());
    }
}

fn test_document_symbols_with_invalid_uri() {
    if !rust_analyzer_available() {
        return;
    }
    let out = run_cli(&["document-symbols", "--uri", "file:///nonexistent/file.rs"]);
    if out.status.success() {
        let result = assert_success_result(&out.stdout);
        let symbols = result.as_array().unwrap();
        assert!(symbols.is_empty());
    }
}

fn test_references_on_keyword_returns_empty_or_ok() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();
    let out = run_cli(&[
        "references",
        "--uri",
        &uri,
        "--line",
        "0",
        "--character",
        "0",
    ]);
    assert!(
        out.status.success() || !out.stdout.is_empty(),
        "should not crash on keyword position"
    );
}

// ---------------------------------------------------------------------------
// Multiple sequential commands reuse daemon — shared daemon lock
// ---------------------------------------------------------------------------

fn test_multiple_commands_same_daemon() {
    if !rust_analyzer_available() {
        return;
    }
    let uri = file_uri();

    let out1 = run_cli(&["status"]);
    assert!(out1.status.success(), "stderr: {}", out1.stderr);
    let status1 = assert_success_result(&out1.stdout);
    let pid1 = status1["pid"].as_u64().unwrap();
    let port1 = status1["port"].as_u64().unwrap();

    let out2 = run_cli(&[
        "definition",
        "--uri",
        &uri,
        "--line",
        "24",
        "--character",
        "5",
    ]);
    assert!(out2.status.success(), "stderr: {}", out2.stderr);

    let out3 = run_cli(&["status"]);
    assert!(out3.status.success(), "stderr: {}", out3.stderr);
    let status2 = assert_success_result(&out3.stdout);
    let pid2 = status2["pid"].as_u64().unwrap();
    let port2 = status2["port"].as_u64().unwrap();

    assert_eq!(pid1, pid2, "same daemon PID across commands");
    assert_eq!(port1, port2, "same daemon port across commands");
}

#[test]
fn test_cli_suite() {
    let non_daemon_tests: &[TestFn] = &[
        test_help_flag,
        test_version_flag,
        test_subcommand_help_definition,
        test_subcommand_help_references,
        test_subcommand_help_workspace_symbols,
        test_subcommand_help_workspace_symbol_resolve,
        test_subcommand_help_incoming_calls_recursive,
        test_subcommand_help_status,
        test_subcommand_help_analyze_trait_impl_deps_graph,
        test_missing_subcommand_shows_help,
        test_definition_missing_uri,
        test_definition_missing_line,
        test_definition_missing_character,
        test_unknown_subcommand,
        test_analyze_trait_impl_deps_graph_missing_target_dir,
        test_definition_conflicting_uri_and_relative_path,
        test_invalid_env_initialize_params_file_returns_json_error,
        test_invalid_env_initialize_params_json_returns_json_error,
    ];
    run_in_parallel(non_daemon_tests);

    if !rust_analyzer_available() {
        return;
    }

    let _guard = acquire_exclusive_daemon_lock();
    stop_daemon();
    test_status_command();

    let can_share_daemon_tests: &[TestFn] = &[
        test_output_is_valid_json,
        test_success_response_has_result_field,
        test_definition_of_function_call,
        test_definition_with_relative_path,
        test_definition_of_trait_method_call,
        test_definition_of_struct_field,
        test_definition_at_definition_site_points_to_self,
        test_type_definition_of_variable,
        test_type_definition_of_function_return,
        test_implementation_of_trait,
        test_implementation_of_trait_method,
        test_implementation_of_struct,
        test_references_include_declaration,
        test_references_exclude_declaration,
        test_references_of_trait_method,
        test_document_symbols,
        test_document_symbols_kinds,
        test_document_symbols_children,
        test_workspace_symbols_query,
        test_workspace_symbols_limit,
        test_workspace_symbols_blank_query_returns_json_error,
        test_workspace_symbol_resolve,
        test_incoming_calls,
        test_incoming_calls_of_leaf_function,
        test_outgoing_calls,
        test_outgoing_calls_of_leaf,
        test_incoming_calls_recursive,
        test_incoming_calls_recursive_with_depth_limit,
        test_incoming_calls_recursive_without_max_depth,
        test_outgoing_calls_recursive,
        test_outgoing_calls_recursive_with_depth_limit,
        test_definition_at_whitespace_returns_empty,
        test_definition_out_of_range_position,
        test_definition_with_invalid_uri,
        test_document_symbols_with_invalid_uri,
        test_references_on_keyword_returns_empty_or_ok,
        test_multiple_commands_same_daemon,
        test_analyze_trait_impl_deps_graph_invalid_target_dir_returns_json_error,
        test_analyze_trait_impl_deps_graph_trait_not_found_returns_empty,
        test_analyze_trait_impl_deps_graph_empty_directory_returns_empty,
        test_analyze_trait_impl_deps_graph_trait_with_impl_but_no_functions_returns_empty,
        test_analyze_trait_impl_deps_graph_builds_dependency_edges_within_target_set,
        test_analyze_trait_impl_deps_graph_multiple_traits_include_trait_name_field,
    ];
    run_in_parallel(can_share_daemon_tests);

    test_subsequent_command_faster_than_first();
}
