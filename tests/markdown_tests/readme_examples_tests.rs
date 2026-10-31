// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Compiles Rust code fences from the crate README files.

use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::fs::TryLockError;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use toml::Table;
use toml::Value;
use toml::from_str;

use super::process::ProcessOutcome;
use super::process::run_with_timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExampleMode {
    Run,
    Check,
}

#[derive(Debug, PartialEq, Eq)]
struct MarkdownExample {
    source: String,
    source_path: String,
    line: usize,
    features: Vec<String>,
    mode: ExampleMode,
}

const ALLOWED_FEATURES: &[&str] = &["qubit-snowflake", "classic-snowflake", "sonyflake", "uuid", "serde"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DocumentSource {
    ReadmeEn,
    ReadmeZhCn,
    GuideEn,
    GuideZhCn,
}

impl DocumentSource {
    const ALL: [(Self, &'static str); 4] = [
        (Self::ReadmeEn, "README.md"),
        (Self::ReadmeZhCn, "README.zh_CN.md"),
        (Self::GuideEn, "doc/user_guide.md"),
        (Self::GuideZhCn, "doc/user_guide.zh_CN.md"),
    ];
}

/// Compiles and runs all Rust snippets in the README files and user guides.
///
/// # Panics
///
/// Panics when a README cannot be read, a temporary crate cannot be created,
/// or a snippet fails to compile or run.
#[test]
#[ignore = "complete documentation execution is run once by the project hook"]
fn test_review_documentation_examples() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_root = manifest_dir.join("target/markdown-doctest");
    fs::create_dir_all(&output_root).expect("failed to create markdown output directory");
    let _run_lock = acquire_run_lock(&output_root, Duration::from_secs(10)).unwrap_or_else(|error| panic!("{error}"));
    let run_id = format!("run-{}-{}", std::process::id(), timestamp_nonce());
    let run_dir = output_root.join(&run_id);
    fs::create_dir_all(run_dir.join("src")).expect("failed to create markdown run directory");
    copy_fixture(&manifest_dir, &run_dir).unwrap_or_else(|error| panic!("{error}"));
    let target_dir = output_root.join("build");
    let mut groups: BTreeMap<Vec<String>, Vec<MarkdownExample>> = BTreeMap::new();
    for (_, relative) in DocumentSource::ALL {
        let path = manifest_dir.join(relative);
        let content = fs::read_to_string(&path).expect("failed to read markdown document");
        let examples = parse_markdown_examples(&path.display().to_string(), &content).unwrap_or_else(|e| panic!("{e}"));
        assert!(!examples.is_empty(), "{} should contain Rust snippets", path.display());
        for example in examples {
            groups.entry(example.features.clone()).or_default().push(example);
        }
    }
    let mut executed = 0;
    for (features, examples) in groups {
        for example in examples {
            executed += 1;
            run_example(&run_dir, &target_dir, &features, &example).unwrap_or_else(|error| panic!("{error}"));
        }
    }
    let failure = MarkdownExample {
        source: "fn main() { panic!(\"expected failure\"); }".to_owned(),
        source_path: "controlled_failure".to_owned(),
        line: 1,
        features: Vec::new(),
        mode: ExampleMode::Run,
    };
    run_example(&run_dir, &target_dir, &[], &failure).unwrap_or_else(|error| panic!("{error}"));
    println!(
        "executed {executed} documentation snippets; shared target: {}",
        target_dir.display()
    );
    fs::remove_dir_all(&run_dir)
        .unwrap_or_else(|error| panic!("remove successful run directory {}: {error}", run_dir.display()));
}

fn run_example(
    run_dir: &Path,
    target_dir: &Path,
    features: &[String],
    example: &MarkdownExample,
) -> Result<(), String> {
    fs::write(run_dir.join("src/main.rs"), normalize_snippet(&example.source))
        .map_err(|error| format!("{}:{} write source: {error}", example.source_path, example.line))?;
    let label = format!("{}:{} features={features:?}", example.source_path, example.line);
    let offline = std::env::var_os("QUBIT_ID_MARKDOWN_OFFLINE").is_some_and(|value| value == "1");
    let cargo_stage = if example.mode == ExampleMode::Check {
        "check"
    } else {
        "build"
    };
    let mut cargo = Command::new("cargo");
    cargo
        .arg(cargo_stage)
        .arg("--locked")
        .arg("--no-default-features")
        .arg("--quiet");
    if !features.is_empty() {
        cargo.arg("--features").arg(features.join(","));
    }
    if offline {
        cargo.arg("--offline");
    }
    cargo.current_dir(run_dir).env("CARGO_TARGET_DIR", target_dir);
    let build_budget = Duration::from_secs(600);
    let build_log = run_dir.join(format!("{cargo_stage}.log"));
    let build = run_with_timeout(cargo, build_budget, &format!("{label} {cargo_stage}"), &build_log)?;
    ensure_success(&build, &format!("{label} {cargo_stage}"))?;
    if example.mode == ExampleMode::Run {
        let executable = target_dir
            .join("debug")
            .join(executable_name("qubit-id-markdown-examples"));
        let mut command = Command::new(executable);
        command.current_dir(run_dir);
        let log = run_dir.join("run.log");
        let outcome = run_with_timeout(command, Duration::from_secs(15), &format!("{label} run"), &log)?;
        if example.source.contains("expected failure") {
            if outcome.status.success() {
                return Err(format!("{label} controlled failure unexpectedly succeeded"));
            }
            let logs = fs::read_to_string(&outcome.log_path).unwrap_or_default();
            if !logs.contains("expected failure") {
                return Err(format!(
                    "{label} failure log omitted expected stderr: {}\n{logs}",
                    outcome.log_path.display()
                ));
            }
        } else {
            ensure_success(&outcome, &format!("{label} run"))?;
        }
    }
    Ok(())
}

fn ensure_success(outcome: &ProcessOutcome, stage: &str) -> Result<(), String> {
    if outcome.status.success() {
        Ok(())
    } else {
        let log = fs::read_to_string(&outcome.log_path).unwrap_or_default();
        Err(format!(
            "{stage}: exited {}. Log: {}\n{log}",
            outcome.status,
            outcome.log_path.display()
        ))
    }
}

fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

fn copy_fixture(manifest_dir: &Path, run_dir: &Path) -> Result<(), String> {
    let fixture = manifest_dir.join("tests/fixtures/markdown_examples");
    fs::copy(fixture.join("Cargo.toml"), run_dir.join("Cargo.toml")).map_err(|error| error.to_string())?;
    fs::copy(fixture.join("Cargo.lock"), run_dir.join("Cargo.lock")).map_err(|error| error.to_string())?;
    let source = fs::read_to_string(run_dir.join("Cargo.toml")).map_err(|error| error.to_string())?;
    let dependency_root = manifest_dir.canonicalize().map_err(|error| error.to_string())?;
    let rewritten = rewrite_fixture_manifest(&source, &dependency_root)?;
    fs::write(run_dir.join("Cargo.toml"), rewritten).map_err(|error| error.to_string())
}

fn rewrite_fixture_manifest(source: &str, dependency_root: &Path) -> Result<String, String> {
    let mut manifest: Table = from_str(source).map_err(|error| error.to_string())?;
    manifest["dependencies"]["qubit-id"]["path"] = Value::String(dependency_root.to_string_lossy().into_owned());
    toml::to_string(&manifest).map_err(|error| error.to_string())
}

#[derive(Debug)]
struct RunLock {
    _file: File,
}

fn acquire_run_lock(output_root: &Path, budget: Duration) -> Result<RunLock, String> {
    let path = output_root.join(".markdown-run.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|error| format!("open documentation run lock {}: {error}", path.display()))?;
    let started = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(RunLock { _file: file }),
            Err(TryLockError::WouldBlock) if started.elapsed() < budget => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(TryLockError::WouldBlock) => {
                return Err(format!("timed out acquiring documentation run lock {}", path.display()));
            }
            Err(TryLockError::Error(error)) => {
                return Err(format!("lock documentation run lock {}: {error}", path.display()));
            }
        }
    }
}

fn timestamp_nonce() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

#[test]
fn test_markdown_process_child() {
    match std::env::var("QUBIT_ID_MARKDOWN_CHILD_MODE").as_deref() {
        Ok("fail") => panic!("controlled child failure"),
        Ok("wait") => loop {
            thread::park_timeout(Duration::from_secs(1));
        },
        _ => {}
    }
}

#[test]
fn test_markdown_process_timeout_helper_handles_success_failure_and_hang() {
    let temp = std::env::temp_dir().join(format!("qubit-markdown-process-{}", timestamp_nonce()));
    fs::create_dir_all(&temp).unwrap();
    let success = controlled_child("success", &temp.join("success.log"), Duration::from_secs(2)).unwrap();
    assert!(success.status.success());
    let failure = controlled_child("fail", &temp.join("failure.log"), Duration::from_secs(2)).unwrap();
    assert!(!failure.status.success());
    let error = controlled_child("wait", &temp.join("wait.log"), Duration::from_millis(50)).unwrap_err();
    assert!(error.contains("controlled wait"), "{error}");
    let _ = fs::remove_dir_all(temp);
}

fn controlled_child(mode: &str, log: &Path, budget: Duration) -> Result<ProcessOutcome, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut command = Command::new(executable);
    command
        .arg("--exact")
        .arg("markdown_tests::readme_examples_tests::test_markdown_process_child")
        .arg("--nocapture")
        .env("QUBIT_ID_MARKDOWN_CHILD_MODE", mode);
    run_with_timeout(command, budget, &format!("controlled {mode}"), log)
}

/// Extracts Rust code fences from a Markdown file.
///
/// # Parameters
///
/// * `path` - Markdown file to read.
///
/// # Returns
///
/// One source string for each Rust or `rs` code fence.
///
/// # Panics
///
/// Panics when the Markdown file cannot be read.
fn parse_markdown_examples(path: &str, content: &str) -> Result<Vec<MarkdownExample>, String> {
    let mut examples = Vec::new();
    let mut pending: Option<(usize, Vec<String>, ExampleMode)> = None;
    let mut in_rust = false;
    let mut other_fence_line = None;
    let mut current = String::new();
    let mut source_line = 0;
    for (index, line) in content.lines().enumerate() {
        let number = index + 1;
        let trimmed = line.trim();
        if other_fence_line.is_some() {
            if trimmed.starts_with("```") {
                other_fence_line = None;
            }
            continue;
        }
        if in_rust {
            if trimmed.starts_with("```") {
                let (_, features, mode) = pending.take().unwrap();
                examples.push(MarkdownExample {
                    source: current.trim().to_owned(),
                    source_path: path.to_owned(),
                    line: source_line,
                    features,
                    mode,
                });
                current.clear();
                in_rust = false;
            } else {
                current.push_str(line);
                current.push('\n');
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("<!-- qubit-id-example") {
            if pending.is_some() {
                return Err(format!("{path}:{number}: previous metadata was not consumed"));
            }
            let body = rest
                .strip_suffix("-->")
                .ok_or_else(|| format!("{path}:{number}: malformed metadata"))?
                .trim();
            let fields = body.split_whitespace().collect::<Vec<_>>();
            if fields.len() != 2 {
                return Err(format!("{path}:{number}: expected features and mode"));
            }
            let raw = fields[0]
                .strip_prefix("features=\"")
                .and_then(|s| s.strip_suffix('"'))
                .ok_or_else(|| format!("{path}:{number}: malformed features"))?;
            let mode_raw = fields[1]
                .strip_prefix("mode=\"")
                .and_then(|s| s.strip_suffix('"'))
                .ok_or_else(|| format!("{path}:{number}: malformed mode"))?;
            let mode = match mode_raw {
                "run" => ExampleMode::Run,
                "check" => ExampleMode::Check,
                _ => return Err(format!("{path}:{number}: unknown mode {mode_raw}")),
            };
            let mut features = Vec::new();
            for feature in raw.split(',').filter(|s| !s.is_empty()) {
                if !ALLOWED_FEATURES.contains(&feature) {
                    return Err(format!("{path}:{number}: unknown feature {feature}"));
                }
                if features.iter().any(|f| f == feature) {
                    return Err(format!("{path}:{number}: duplicate feature {feature}"));
                }
                features.push(feature.to_owned());
            }
            features.sort();
            pending = Some((number, features, mode));
            continue;
        }
        if let Some(language) = trimmed.strip_prefix("```") {
            if let Some((line, _, _)) = pending.as_ref().filter(|_| !is_rust_fence(language)) {
                return Err(format!("{path}:{line}: metadata must bind to a Rust fence"));
            }
            if is_rust_fence(language) {
                if pending.is_none() {
                    return Err(format!("{path}:{number}: Rust fence has no metadata"));
                }
                in_rust = true;
                source_line = number + 1;
            } else {
                if pending.is_some() {
                    return Err(format!("{path}:{number}: metadata was not consumed"));
                }
                other_fence_line = Some(number);
            }
            continue;
        }
        if let Some((line, _, _)) = pending.as_ref().filter(|_| !trimmed.is_empty()) {
            return Err(format!("{path}:{line}: metadata was not followed by a Rust fence"));
        }
    }
    if in_rust {
        return Err(format!("{path}:{source_line}: unclosed Rust fence"));
    }
    if let Some(line) = other_fence_line {
        return Err(format!("{path}:{line}: unclosed non-Rust fence"));
    }
    if let Some((line, _, _)) = pending {
        return Err(format!("{path}:{line}: metadata was not consumed"));
    }
    Ok(examples)
}

/// Returns whether a Markdown fence language identifies Rust source code.
///
/// # Parameters
///
/// * `language` - Text immediately following the opening fence.
///
/// # Returns
///
/// `true` for the `rust` and `rs` language tags, including fence attributes.
fn is_rust_fence(language: &str) -> bool {
    let tag = language
        .trim()
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .next()
        .unwrap_or_default();
    matches!(tag, "rust" | "rs")
}

/// Normalizes a Markdown snippet into a standalone binary source file.
///
/// # Parameters
///
/// * `snippet` - Rust code extracted from a README fence.
///
/// # Returns
///
/// Source code that can be compiled as a binary target.
fn normalize_snippet(snippet: &str) -> String {
    let allow_example_noise = "#![allow(dead_code, unused_imports, unused_variables)]\n";
    if snippet.contains("fn main") {
        format!("{allow_example_noise}{snippet}\n")
    } else {
        format!("{allow_example_noise}fn main() {{\n{snippet}\n}}\n")
    }
}

fn assert_document_versions_match(manifest_dir: &Path) -> Result<(), String> {
    let cargo: Value = from_str(&fs::read_to_string(manifest_dir.join("Cargo.toml")).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let package_version = cargo["package"]["version"]
        .as_str()
        .ok_or("Cargo package.version missing")?;
    let expected = package_version.split('.').take(2).collect::<Vec<_>>().join(".");
    for relative in [
        "README.md",
        "README.zh_CN.md",
        "doc/user_guide.md",
        "doc/user_guide.zh_CN.md",
    ] {
        let content = fs::read_to_string(manifest_dir.join(relative)).map_err(|e| e.to_string())?;
        let mut found = Vec::new();
        for line in content
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("qubit-id ="))
        {
            let parsed: Table = from_str(line).map_err(|e| format!("{relative}: invalid qubit-id dependency: {e}"))?;
            let dependency = &parsed["qubit-id"];
            let version = dependency
                .as_str()
                .or_else(|| dependency.get("version").and_then(Value::as_str))
                .ok_or_else(|| format!("{relative}: qubit-id dependency has no version"))?;
            found.push(version.to_owned());
        }
        if found.is_empty() || found.iter().any(|version| version != &expected) {
            return Err(format!(
                "{relative}: qubit-id dependency versions {found:?} do not match {expected}"
            ));
        }
        if relative.starts_with("doc/")
            && !content.lines().any(|line| {
                line.contains("qubit-id")
                    && line.contains(&expected)
                    && (line.contains("series") || line.contains("系列"))
            })
        {
            return Err(format!(
                "{relative}: guide version declaration does not match {expected}"
            ));
        }
    }
    Ok(())
}

#[test]
fn test_markdown_fixture_feature_mapping() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/markdown_examples/Cargo.toml");
    let source = fs::read_to_string(fixture).unwrap();
    let parsed: Table = from_str(&source).unwrap();
    let features = parsed["features"].as_table().unwrap();
    assert!(features["default"].as_array().unwrap().is_empty());
    let qubit = features["qubit-snowflake"].as_array().unwrap();
    assert_eq!(qubit.len(), 3);
    for expected in ["qubit-id/qubit-snowflake", "dep:qubit-clock", "dep:tokio"] {
        assert!(qubit.iter().any(|value| value.as_str() == Some(expected)));
    }
    for (feature, forwarded) in [
        ("classic-snowflake", "qubit-id/classic-snowflake"),
        ("sonyflake", "qubit-id/sonyflake"),
        ("uuid", "qubit-id/uuid"),
        ("serde", "qubit-id/serde"),
    ] {
        assert_eq!(
            features[feature].as_array().unwrap(),
            &[Value::String(forwarded.to_owned())]
        );
    }
    let dependencies = parsed["dependencies"].as_table().unwrap();
    assert_eq!(dependencies["qubit-id"]["default-features"].as_bool(), Some(false));
    assert_eq!(dependencies["qubit-clock"]["optional"].as_bool(), Some(true));
    assert_eq!(dependencies["tokio"]["optional"].as_bool(), Some(true));
}

#[test]
fn test_markdown_fixture_path_round_trip() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join("tests/fixtures/markdown_examples/Cargo.toml");
    let source = fs::read_to_string(&fixture).unwrap();
    let original: Table = from_str(&source).unwrap();
    let local_root = manifest_dir.canonicalize().unwrap();
    let roots = [
        local_root.to_string_lossy().into_owned(),
        r#"D:\work\quoted"name\rs-id"#.to_owned(),
        r#"/tmp/quoted"name/rs-id"#.to_owned(),
    ];
    for root in roots {
        let rewritten = rewrite_fixture_manifest(&source, Path::new(&root)).unwrap();
        let mut parsed: Table = from_str(&rewritten).unwrap();
        assert_eq!(parsed["dependencies"]["qubit-id"]["path"].as_str(), Some(root.as_str()));
        parsed["dependencies"]["qubit-id"]["path"] = original["dependencies"]["qubit-id"]["path"].clone();
        assert_eq!(parsed, original);
    }

    let output_root = std::env::temp_dir().join(format!("qubit-markdown-fixture-{}", timestamp_nonce()));
    fs::create_dir_all(&output_root).unwrap();
    let result = copy_fixture(&manifest_dir, &output_root);
    assert!(result.is_ok(), "copy_fixture failed: {result:?}");
    let mut output_manifest: Table = from_str(&fs::read_to_string(output_root.join("Cargo.toml")).unwrap()).unwrap();
    assert_eq!(
        output_manifest["dependencies"]["qubit-id"]["path"].as_str(),
        Some(manifest_dir.canonicalize().unwrap().to_string_lossy().as_ref())
    );
    output_manifest["dependencies"]["qubit-id"]["path"] = original["dependencies"]["qubit-id"]["path"].clone();
    assert_eq!(output_manifest, original);
    assert_eq!(
        fs::read(output_root.join("Cargo.lock")).unwrap(),
        fs::read(manifest_dir.join("tests/fixtures/markdown_examples/Cargo.lock")).unwrap()
    );
    fs::remove_dir_all(output_root).unwrap();
}

#[test]
fn test_markdown_run_lock_normal_release_and_stale_text() {
    let root = std::env::temp_dir().join(format!("qubit-markdown-lock-normal-{}", timestamp_nonce()));
    fs::create_dir_all(&root).unwrap();
    let lock_path = root.join(".markdown-run.lock");
    fs::write(&lock_path, "stale pid=12345\n").unwrap();
    let first = acquire_run_lock(&root, Duration::from_millis(100)).unwrap();
    let error = acquire_run_lock(&root, Duration::from_millis(40)).unwrap_err();
    assert!(error.contains(&lock_path.display().to_string()), "{error}");
    assert!(lock_path.exists());
    drop(first);
    let second = acquire_run_lock(&root, Duration::from_millis(100)).unwrap();
    assert!(lock_path.exists());
    drop(second);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_markdown_run_lock_reports_io_error() {
    let parent = std::env::temp_dir().join(format!("qubit-markdown-lock-error-{}", timestamp_nonce()));
    fs::create_dir_all(&parent).unwrap();
    let lock_path = parent.join(".markdown-run.lock");
    fs::create_dir(&lock_path).unwrap();
    let error = acquire_run_lock(&parent, Duration::from_millis(50)).unwrap_err();
    assert!(error.contains(&lock_path.display().to_string()), "{error}");
    fs::remove_dir_all(parent).unwrap();
}

struct ChildProcessGuard(Option<std::process::Child>);

impl Drop for ChildProcessGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn test_markdown_run_lock_child() {
    let Ok(root) = std::env::var("QUBIT_ID_LOCK_CHILD_DIR") else {
        return;
    };
    let root = PathBuf::from(root);
    let _lock = acquire_run_lock(&root, Duration::from_secs(2)).expect("child should acquire lock");
    fs::write(root.join("ready"), "locked").expect("write child ready marker");
    loop {
        thread::park_timeout(Duration::from_secs(1));
    }
}

#[test]
fn test_markdown_run_lock_recovers_after_child_exit() {
    let root = std::env::temp_dir().join(format!("qubit-markdown-lock-child-{}", timestamp_nonce()));
    fs::create_dir_all(&root).unwrap();
    let ready = root.join("ready");
    let executable = std::env::current_exe().unwrap();
    let mut command = Command::new(executable);
    command
        .arg("--exact")
        .arg("markdown_tests::readme_examples_tests::test_markdown_run_lock_child")
        .arg("--nocapture")
        .env("QUBIT_ID_LOCK_CHILD_DIR", &root);
    let child = command.spawn().unwrap();
    let mut child = ChildProcessGuard(Some(child));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if ready.exists() {
            break;
        }
        if let Some(status) = child.0.as_mut().unwrap().try_wait().unwrap() {
            panic!("lock child exited before ready: {status}");
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for lock child ready marker"
        );
        thread::sleep(Duration::from_millis(10));
    }
    let lock_path = root.join(".markdown-run.lock");
    let error = acquire_run_lock(&root, Duration::from_millis(50)).unwrap_err();
    assert!(error.contains(&lock_path.display().to_string()), "{error}");
    child.0.as_mut().unwrap().kill().unwrap();
    child.0.as_mut().unwrap().wait().unwrap();
    child.0 = None;
    assert!(lock_path.exists(), "lock path must remain after child exit");
    let recovered = acquire_run_lock(&root, Duration::from_millis(500)).unwrap();
    assert!(lock_path.exists());
    drop(recovered);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_review_markdown_metadata_parses_source_features_and_mode() {
    let snippets = parse_markdown_examples(
        "README.md",
        "<!-- qubit-id-example features=\"classic-snowflake,uuid\" mode=\"run\" -->\n```rust\nfn main() {}\n```\n",
    )
    .unwrap();
    assert_eq!(snippets.len(), 1);
    assert_eq!(snippets[0].source, "fn main() {}");
    assert_eq!(snippets[0].features, ["classic-snowflake", "uuid"]);
    assert_eq!(snippets[0].mode, ExampleMode::Run);
    assert_eq!(snippets[0].source_path, "README.md");
    assert_eq!(snippets[0].line, 3);
}

#[test]
fn test_review_markdown_has_all_four_document_sources() {
    assert_eq!(
        DocumentSource::ALL
            .map(|(source, _)| source)
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    assert_eq!(
        DocumentSource::ALL.map(|(_, path)| path),
        [
            "README.md",
            "README.zh_CN.md",
            "doc/user_guide.md",
            "doc/user_guide.zh_CN.md"
        ]
    );
}

#[test]
fn test_review_markdown_metadata_groups_feature_lists_stably_and_accepts_empty() {
    let snippets = parse_markdown_examples(
        "guide.md",
        "<!-- qubit-id-example features=\"uuid,serde\" mode=\"check\" -->\n```rs\na\n```\n<!-- qubit-id-example features=\"serde,uuid\" mode=\"check\" -->\n```rust\nb\n```\n<!-- qubit-id-example features=\"\" mode=\"run\" -->\n```rust\nc\n```\n",
    )
    .unwrap();
    assert_eq!(snippets[0].features, ["serde", "uuid"]);
    assert_eq!(snippets[1].features, ["serde", "uuid"]);
    assert!(snippets[2].features.is_empty());
}

#[test]
fn test_review_markdown_metadata_rejects_invalid_or_unbound_metadata() {
    for content in [
        "<!-- qubit-id-example features=\"made-up\" mode=\"run\" -->\n```rust\nx\n```",
        "<!-- qubit-id-example features=\"uuid,uuid\" mode=\"run\" -->\n```rust\nx\n```",
        "<!-- qubit-id-example features=\"uuid\" mode=\"execute\" -->\n```rust\nx\n```",
        "```rust\nx\n```",
        "```toml\nx = 1",
        "<!-- qubit-id-example features=\"uuid\" mode=\"run\" -->\ntext",
        "<!-- qubit-id-example features=\"uuid\" mode=\"run\" -->\n```rust\nx",
    ] {
        let error = parse_markdown_examples("guide.md", content).unwrap_err();
        assert!(error.contains("guide.md:"), "{error}");
    }
}

#[test]
fn test_review_markdown_metadata_ignores_non_rust_fences() {
    let snippets = parse_markdown_examples(
        "guide.md",
        "```toml\n<!-- qubit-id-example features=\"unknown\" mode=\"bad\" -->\nx = 1\n```\n",
    )
    .unwrap();
    assert!(snippets.is_empty());
}

#[test]
fn test_review_markdown_document_versions_match_package_minor() {
    assert_document_versions_match(&PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap();
}

/// Verifies that both READMEs retain key feature, API, and benchmark
/// references.
#[test]
fn test_readmes_retain_feature_and_benchmark_contract_tokens() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let readmes = [
        (
            "README.md",
            [
                "`IdMode::Spread`",
                "`i64::MAX`",
                "`try_generate()`",
                "`generate()`",
                "`generate_async()`",
                "`RestartPolicy::Immediate`",
                "`IdGenerationError::EpochAhead`",
                "`compose_at(time, sequence)`",
                "max_clock_skew",
                "`IdGenerator`",
                "`TryIdGenerator`",
                "`AsyncIdGenerator`",
            ],
        ),
        (
            "README.zh_CN.md",
            [
                "`IdMode::Spread`",
                "`i64::MAX`",
                "`try_generate()`",
                "`generate()`",
                "`generate_async()`",
                "`RestartPolicy::Immediate`",
                "`IdGenerationError::EpochAhead`",
                "`compose_at(time, sequence)`",
                "max_clock_skew",
                "`IdGenerator`",
                "`TryIdGenerator`",
                "`AsyncIdGenerator`",
            ],
        ),
    ];
    let common_fragments = [
        r#"default-features = false, features = ["classic-snowflake"]"#,
        r#"default-features = false, features = ["sonyflake"]"#,
        r#"default-features = false, features = ["uuid"]"#,
        "`now >= expires_at`",
        "`Arc<dyn IdGenerator<Id>>`",
        "Arc<dyn IdGenerator<Uuid>>",
        "`Arc<dyn AsyncIdGenerator<Id>>`",
        "`ManualMonotonicClock`",
        "`UuidV4Generator`",
        "`ClassicalSnowflakeGenerator`",
        "cargo bench --no-default-features --features uuid --bench uuid_comparison",
    ];

    for (name, deployment_fragments) in readmes {
        let content = fs::read_to_string(manifest_dir.join(name))
            .unwrap_or_else(|error| panic!("failed to read {name}: {error}"));
        for fragment in common_fragments {
            assert!(content.contains(fragment), "{name} must contain `{fragment}`");
        }
        for fragment in deployment_fragments {
            assert!(content.contains(fragment), "{name} must contain `{fragment}`");
        }
        for stale_fragment in ["start_time", "StartTimeAhead"] {
            assert!(
                !content.contains(stale_fragment),
                "{name} must not contain obsolete API `{stale_fragment}`"
            );
        }
    }
}

/// Verifies that async documentation retains its API reference and rejects a
/// stale allocation claim.
#[test]
fn test_async_documentation_retains_api_and_rejects_stale_allocation_claim() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let readme = fs::read_to_string(manifest_dir.join("README.md")).expect("README.md should be readable");

    assert!(
        !readme.contains("allocation-free inherent Future"),
        "README.md must not claim that every async path is allocation-free"
    );

    for path in [
        "src/snowflake/qubit/snowflake_generator.rs",
        "src/snowflake/classical/classical_snowflake_generator.rs",
        "src/snowflake/sonyflake/sonyflake_generator.rs",
    ] {
        let source = fs::read_to_string(manifest_dir.join(path))
            .unwrap_or_else(|error| panic!("failed to read {path}: {error}"));
        assert!(
            source.contains("generate_async"),
            "{path} must expose its inherent async method"
        );
    }
}

/// Verifies that both READMEs describe generation failures as structured
/// errors rather than panics.
#[test]
fn test_readmes_document_structured_generation_failures() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let readmes = [
        (
            "README.md",
            "IdGenerationError::RandomSourceFailed",
            "IdGenerationError::GeneratorExpired",
            ["UUID generation panics", "builders panic"],
        ),
        (
            "README.zh_CN.md",
            "IdGenerationError::RandomSourceFailed",
            "IdGenerationError::GeneratorExpired",
            ["UUID 生成会 panic", "构建器会 panic"],
        ),
    ];

    for (name, random_failure, expired_error, forbidden_fragments) in readmes {
        let content = fs::read_to_string(manifest_dir.join(name))
            .unwrap_or_else(|error| panic!("failed to read {name}: {error}"));
        for fragment in [random_failure, expired_error] {
            assert!(content.contains(fragment), "{name} must contain `{fragment}`");
        }
        for fragment in forbidden_fragments {
            assert!(
                !content.contains(fragment),
                "{name} must not contain stale panic contract `{fragment}`"
            );
        }
    }
}

/// Verifies that docs.rs builds the complete feature-gated public API.
#[test]
fn test_docs_rs_build_enables_all_features() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("Cargo.toml should be readable");
    let manifest_value = from_str::<Table>(&manifest).expect("Cargo.toml should be valid TOML");
    let package = manifest_value
        .get("package")
        .and_then(Value::as_table)
        .expect("Cargo.toml must define a package table");
    let docs_rs = package
        .get("metadata")
        .and_then(Value::as_table)
        .and_then(|metadata| metadata.get("docs"))
        .and_then(Value::as_table)
        .and_then(|docs| docs.get("rs"))
        .and_then(Value::as_table)
        .expect("Cargo.toml must define package.metadata.docs.rs");

    assert!(
        docs_rs
            .get("all-features")
            .and_then(Value::as_bool)
            .is_some_and(|all_features| all_features),
        "Cargo.toml must enable all features for docs.rs"
    );

    let rustdoc_args = docs_rs
        .get("rustdoc-args")
        .and_then(Value::as_array)
        .expect("Cargo.toml must define docs.rs rustdoc arguments");
    assert!(
        rustdoc_args.iter().filter_map(Value::as_str).eq(["--cfg", "docsrs"]),
        "Cargo.toml must enable docsrs cfg for feature annotations"
    );

    let includes_docs = package
        .get("include")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|path| path == "/doc/**");
    assert!(
        includes_docs,
        "Cargo.toml must include the user guide directory in packages"
    );

    let crate_root = fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("src/lib.rs should be readable");
    for fragment in [
        "#![cfg_attr(docsrs, feature(doc_cfg))]",
        "doc(cfg(feature = \"qubit-snowflake\"))",
        "doc(cfg(feature = \"classic-snowflake\"))",
        "doc(cfg(feature = \"sonyflake\"))",
        "doc(cfg(feature = \"uuid\"))",
    ] {
        assert!(
            crate_root.contains(fragment),
            "src/lib.rs must annotate feature-gated API with `{fragment}`"
        );
    }
}
