// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Compiles Rust code fences from the crate README files.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use toml::Table;
use toml::Value;
use toml::from_str;

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

/// Compiles all Rust snippets in the English and Chinese README files.
///
/// # Panics
///
/// Panics when a README cannot be read, a temporary crate cannot be created,
/// or a snippet fails to compile.
#[test]
fn test_review_documentation_examples() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_dir = manifest_dir
        .join("target/markdown-doctest")
        .join(format!("run-{}", std::process::id()));
    fs::create_dir_all(&output_dir).expect("failed to create markdown output directory");
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
    for (index, (features, examples)) in groups.into_iter().enumerate() {
        let name = format!("group_{index}");
        let crate_dir = output_dir.join(&name);
        let bin_dir = crate_dir.join("src/bin");
        fs::create_dir_all(&bin_dir).expect("failed to create snippet bin directory");
        fs::write(
            crate_dir.join("Cargo.toml"),
            build_markdown_doctest_manifest(&name, &manifest_dir, &features),
        )
        .expect("failed to write snippet manifest");
        for mode in [ExampleMode::Check, ExampleMode::Run] {
            for (example_index, example) in examples.iter().filter(|e| e.mode == mode).enumerate() {
                let bin = format!(
                    "{}_{}",
                    if mode == ExampleMode::Run { "run" } else { "check" },
                    example_index
                );
                fs::write(bin_dir.join(format!("{bin}.rs")), normalize_snippet(&example.source))
                    .expect("failed to write snippet source");
                let action = if mode == ExampleMode::Run { "run" } else { "check" };
                let output = Command::new("cargo")
                    .arg(action)
                    .arg("--quiet")
                    .arg("--bin")
                    .arg(&bin)
                    .current_dir(&crate_dir)
                    .env("CARGO_TARGET_DIR", output_dir.join("target"))
                    .output()
                    .expect("failed to run cargo for markdown example");
                assert!(
                    output.status.success(),
                    "{}:{} {action} failed ({}):\n{}",
                    example.source_path,
                    example.line,
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
    let failure_dir = output_dir.join("expected_failure");
    let failure_bin = failure_dir.join("src/bin");
    fs::create_dir_all(&failure_bin).expect("failed to create failure example directory");
    fs::write(
        failure_dir.join("Cargo.toml"),
        build_markdown_doctest_manifest("expected_failure", &manifest_dir, &[]),
    )
    .expect("failed to write failure manifest");
    fs::write(
        failure_bin.join("expected_failure.rs"),
        "fn main() { panic!(\"expected failure\"); }\n",
    )
    .expect("failed to write failure example");
    let failure = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--bin")
        .arg("expected_failure")
        .current_dir(&failure_dir)
        .env("CARGO_TARGET_DIR", output_dir.join("target"))
        .output()
        .expect("failed to run controlled failure example");
    assert!(
        !failure.status.success(),
        "run mode must propagate a child panic as failure"
    );
    assert!(
        String::from_utf8_lossy(&failure.stderr).contains("expected failure"),
        "child stderr should be retained: {}",
        String::from_utf8_lossy(&failure.stderr)
    );
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

/// Builds a Cargo manifest for a temporary Markdown snippet crate.
///
/// # Parameters
///
/// * `name` - Stable name for the temporary crate.
/// * `manifest_dir` - Root directory of the `qubit-id` crate dependency.
/// * `features` - `qubit-id` features required by the snippets.
///
/// # Returns
///
/// A Cargo manifest for a private temporary crate.
fn build_markdown_doctest_manifest(name: &str, manifest_dir: &Path, features: &[String]) -> String {
    let manifest_path = toml_basic_string(&manifest_dir.display().to_string());
    let uses_qubit_snowflake = features.iter().any(|feature| feature == "qubit-snowflake");
    let features = format!(
        "[{}]",
        features
            .iter()
            .map(|feature| format!("\"{}\"", toml_basic_string(feature)))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let runtime_dependencies = if uses_qubit_snowflake {
        "qubit-clock = { version = \"0.13\", features = [\"tokio\"] }\ntokio = { version = \"1.54\", features = [\"macros\", \"rt\", \"time\"] }\n"
    } else {
        ""
    };

    format!(
        r#"[package]
name = "qubit-id-{name}-markdown-doctest"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
qubit-id = {{ path = "{manifest_path}", default-features = false, features = {features} }}
{runtime_dependencies}uuid = {{ version = "1", features = ["v4"] }}
serde_json = "1"
"#
    )
}

/// Escapes a value for use inside a TOML basic string.
///
/// # Parameters
///
/// * `value` - Unescaped string value.
///
/// # Returns
///
/// A TOML-safe basic string body.
fn toml_basic_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\u{0008}' => escaped.push_str("\\b"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\u{000C}' => escaped.push_str("\\f"),
            '\r' => escaped.push_str("\\r"),
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\u{0000}'..='\u{001F}' | '\u{007F}' => {
                write!(escaped, "\\u{:04X}", ch as u32).expect("writing TOML escape to String should not fail");
            }
            _ => escaped.push(ch),
        }
    }
    escaped
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

/// Verifies that Windows dependency paths are escaped in the generated TOML.
#[test]
fn test_build_markdown_doctest_manifest_escapes_windows_dependency_path() {
    let manifest = build_markdown_doctest_manifest("readme_en", Path::new(r"D:\a\rs-id\rs-id"), &[]);

    assert!(
        manifest.contains(r#"path = "D:\\a\\rs-id\\rs-id""#),
        "Windows backslashes must be escaped in the generated TOML manifest:\n{manifest}"
    );
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
fn test_review_markdown_manifest_enables_only_declared_features() {
    let manifest =
        build_markdown_doctest_manifest("classic", Path::new("/tmp/qubit-id"), &["classic-snowflake".to_owned()]);
    let parsed: Value = from_str(&manifest).unwrap();
    let features = parsed["dependencies"]["qubit-id"]["features"].as_array().unwrap();
    assert_eq!(features, &[Value::String("classic-snowflake".to_owned())]);
}

#[test]
fn test_build_markdown_doctest_manifest_adds_tokio_only_for_qubit_snowflake() {
    let manifest_dir = Path::new("/tmp/qubit-id");
    let qubit_manifest = build_markdown_doctest_manifest("qubit", manifest_dir, &["qubit-snowflake".to_owned()]);
    let qubit: Value = from_str(&qubit_manifest).unwrap();
    assert!(!qubit["dependencies"]["qubit-id"]["default-features"].as_bool().unwrap());
    assert_eq!(
        qubit["dependencies"]["qubit-id"]["features"].as_array().unwrap(),
        &[Value::String("qubit-snowflake".to_owned())]
    );
    assert_eq!(
        qubit["dependencies"]["qubit-clock"]["features"].as_array().unwrap(),
        &[Value::String("tokio".to_owned())]
    );
    assert_eq!(
        qubit["dependencies"]["tokio"]["features"].as_array().unwrap(),
        &[
            Value::String("macros".to_owned()),
            Value::String("rt".to_owned()),
            Value::String("time".to_owned()),
        ]
    );

    let classic_manifest = build_markdown_doctest_manifest("classic", manifest_dir, &["classic-snowflake".to_owned()]);
    let classic: Value = from_str(&classic_manifest).unwrap();
    assert!(classic["dependencies"].get("qubit-clock").is_none());
    assert!(classic["dependencies"].get("tokio").is_none());
}

#[test]
fn test_review_markdown_document_versions_match_package_minor() {
    assert_document_versions_match(&PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap();
}

/// Verifies that a temporary README manifest enables only its declared feature.
#[test]
fn test_build_markdown_doctest_manifest_enables_declared_features() {
    let manifest = build_markdown_doctest_manifest(
        "readme_en",
        Path::new("/workspace/rs-id"),
        &["classic-snowflake".to_owned()],
    );

    assert!(
        manifest.contains(r#"features = ["classic-snowflake"]"#),
        "temporary dependency must enable only its declared feature:\n{manifest}"
    );
}

/// Verifies that both READMEs explain feature selection, IoC capabilities,
/// deterministic clocks, generator lifetime, storage compatibility, restart
/// behavior, and UUID v4 output.
#[test]
fn test_readmes_document_feature_lifetime_and_benchmark_contracts() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let readmes = [
        (
            "README.md",
            "The default feature set enables only `qubit-snowflake`.",
            "`expires_at()` returns the exclusive expiration boundary.",
            [
                "`IdMode::Spread` always sets bit 63, so its IDs exceed `i64::MAX`.",
                "`try_generate()`, `generate()`, and `generate_async()`",
                "`RestartPolicy::Immediate` is the default for all Snowflake builders.",
                "`IdGenerationError::EpochAhead`",
                "`compose_at(time, sequence)`",
                "`max_clock_skew(...)`",
                "`IdGenerator`, `TryIdGenerator`, and `AsyncIdGenerator` independently",
                "JavaScript or JSON boundaries.",
            ],
        ),
        (
            "README.zh_CN.md",
            "默认 feature 集合只启用 `qubit-snowflake`。",
            "`expires_at()` 返回排他的到期边界。",
            [
                "`IdMode::Spread` 始终设置第 63 位，因此生成的 ID 必然超过 `i64::MAX`。",
                "`try_generate()`、`generate()` 与 `generate_async()`",
                "`RestartPolicy::Immediate` 是所有 Snowflake Builder 的默认值。",
                "`IdGenerationError::EpochAhead`",
                "`compose_at(time, sequence)`",
                "`max_clock_skew()`",
                "`IdGenerator`、`TryIdGenerator` 与 `AsyncIdGenerator` 分别提供阻塞、非阻塞和异步分配能力；",
                "ID 经过 JavaScript 或 JSON 边界时应使用",
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

    for (name, feature_summary, lifetime_summary, deployment_fragments) in readmes {
        let content = fs::read_to_string(manifest_dir.join(name))
            .unwrap_or_else(|error| panic!("failed to read {name}: {error}"));
        for fragment in [feature_summary, lifetime_summary] {
            assert!(content.contains(fragment), "{name} must contain `{fragment}`");
        }
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

/// Verifies that async generation documentation distinguishes its concrete
/// future from the object-safe boxed trait future.
#[test]
fn test_async_documentation_describes_unboxed_outer_future() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let readme = fs::read_to_string(manifest_dir.join("README.md")).expect("README.md should be readable");

    assert!(
        readme.contains("concrete asynchronous method has an unboxed outer Future"),
        "README.md must describe the concrete async method precisely"
    );
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
            "builders return `IdGenerationError::GeneratorExpired` when `now >= expires_at`",
            ["UUID generation panics", "builders panic"],
        ),
        (
            "README.zh_CN.md",
            "IdGenerationError::RandomSourceFailed",
            "构建器会在 `now >= expires_at` 时返回 `IdGenerationError::GeneratorExpired`",
            ["UUID 生成会 panic", "构建器会 panic"],
        ),
    ];

    for (name, random_failure, expiration_failure, forbidden_fragments) in readmes {
        let content = fs::read_to_string(manifest_dir.join(name))
            .unwrap_or_else(|error| panic!("failed to read {name}: {error}"));
        for fragment in [random_failure, expiration_failure] {
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
