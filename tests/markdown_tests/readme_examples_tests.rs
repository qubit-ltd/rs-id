// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Compiles Rust code fences from the crate README files.

use std::fmt::Write as _;
use std::fs;
use std::path::{
    Path,
    PathBuf,
};
use std::process::Command;

/// Compiles all Rust snippets in the English and Chinese README files.
///
/// # Panics
///
/// Panics when a README cannot be read, a temporary crate cannot be created,
/// or a snippet fails to compile.
#[test]
fn test_readme_rust_examples_compile() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_dir = manifest_dir.join("target/markdown-doctest");
    recreate_dir(&output_dir);

    let readmes = [
        ("readme_en", manifest_dir.join("README.md")),
        ("readme_zh_cn", manifest_dir.join("README.zh_CN.md")),
    ];

    for (name, path) in readmes {
        let snippets = extract_rust_snippets(&path);
        assert!(
            !snippets.is_empty(),
            "{} should contain Rust snippets",
            path.display()
        );
        compile_snippets(&manifest_dir, &output_dir, name, &snippets);
    }
}

/// Recreates the directory used by temporary Markdown example crates.
///
/// # Arguments
///
/// * `path` - Directory to remove if present and then recreate.
///
/// # Panics
///
/// Panics when the directory cannot be removed or created.
fn recreate_dir(path: &Path) {
    if path.exists() {
        fs::remove_dir_all(path)
            .expect("failed to remove old markdown doctest directory");
    }
    fs::create_dir_all(path)
        .expect("failed to create markdown doctest directory");
}

/// Extracts Rust code fences from a Markdown file.
///
/// # Arguments
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
fn extract_rust_snippets(path: &Path) -> Vec<String> {
    let content =
        fs::read_to_string(path).expect("failed to read markdown file");
    let mut snippets = Vec::new();
    let mut in_rust = false;
    let mut current = String::new();

    for line in content.lines() {
        if let Some(language) = line.trim_start().strip_prefix("```") {
            if in_rust {
                snippets.push(current.trim().to_owned());
                current.clear();
                in_rust = false;
                continue;
            }
            in_rust = is_rust_fence(language);
            continue;
        }

        if in_rust {
            current.push_str(line);
            current.push('\n');
        }
    }

    snippets
}

/// Returns whether a Markdown fence language identifies Rust source code.
///
/// # Arguments
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

/// Compiles a README's Rust snippets in an isolated temporary crate.
///
/// # Arguments
///
/// * `manifest_dir` - Root directory of the `qubit-id` crate.
/// * `output_dir` - Parent directory for generated temporary crates and build
///   output.
/// * `name` - Stable name for the temporary crate.
/// * `snippets` - Rust source snippets to compile as binary targets.
///
/// # Panics
///
/// Panics when temporary crate files cannot be written or `cargo check` fails.
fn compile_snippets(
    manifest_dir: &Path,
    output_dir: &Path,
    name: &str,
    snippets: &[String],
) {
    let crate_dir = output_dir.join(name);
    let bin_dir = crate_dir.join("src/bin");
    fs::create_dir_all(&bin_dir)
        .expect("failed to create snippet bin directory");

    let manifest = build_markdown_doctest_manifest(name, manifest_dir);
    fs::write(crate_dir.join("Cargo.toml"), manifest)
        .expect("failed to write snippet Cargo.toml");

    for (index, snippet) in snippets.iter().enumerate() {
        let source = normalize_snippet(snippet);
        fs::write(bin_dir.join(format!("snippet_{index}.rs")), source)
            .expect("failed to write snippet source");
    }

    let status = Command::new("cargo")
        .arg("check")
        .arg("--quiet")
        .arg("--bins")
        .current_dir(&crate_dir)
        .env("CARGO_TARGET_DIR", output_dir.join("target"))
        .status()
        .expect("failed to run cargo check for markdown snippets");

    assert!(
        status.success(),
        "markdown Rust snippets failed to compile for {name}"
    );
}

/// Builds the temporary Cargo manifest used to compile README snippets.
///
/// # Arguments
///
/// * `name` - Stable suffix identifying the README under test.
/// * `manifest_dir` - Root directory of the `qubit-id` crate dependency.
///
/// # Returns
///
/// A Cargo manifest for a private temporary crate.
fn build_markdown_doctest_manifest(name: &str, manifest_dir: &Path) -> String {
    let manifest_path = toml_basic_string(&manifest_dir.display().to_string());

    format!(
        r#"[package]
name = "qubit-id-{name}-markdown-doctest"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
qubit-id = {{ path = "{manifest_path}", default-features = false, features = ["classic-snowflake", "qubit-snowflake", "sonyflake", "uuid"] }}
"#
    )
}

/// Escapes a value for use inside a TOML basic string.
///
/// # Arguments
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
                write!(escaped, "\\u{:04X}", ch as u32)
                    .expect("writing TOML escape to String should not fail");
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Normalizes a Markdown snippet into a standalone binary source file.
///
/// # Arguments
///
/// * `snippet` - Rust code extracted from a README fence.
///
/// # Returns
///
/// Source code that can be compiled as a binary target.
fn normalize_snippet(snippet: &str) -> String {
    let allow_example_noise =
        "#![allow(dead_code, unused_imports, unused_variables)]\n";
    if snippet.contains("fn main") {
        format!("{allow_example_noise}{snippet}\n")
    } else {
        format!("{allow_example_noise}fn main() {{\n{snippet}\n}}\n")
    }
}

/// Verifies that Windows dependency paths are escaped in the generated TOML.
#[test]
fn test_build_markdown_doctest_manifest_escapes_windows_dependency_path() {
    let manifest = build_markdown_doctest_manifest(
        "readme_en",
        Path::new(r"D:\a\rs-id\rs-id"),
    );

    assert!(
        manifest.contains(r#"path = "D:\\a\\rs-id\\rs-id""#),
        "Windows backslashes must be escaped in the generated TOML manifest:\n{manifest}"
    );
}

/// Verifies that README examples can use every feature-gated public API.
#[test]
fn test_build_markdown_doctest_manifest_enables_all_example_features() {
    let manifest = build_markdown_doctest_manifest(
        "readme_en",
        Path::new("/workspace/rs-id"),
    );

    assert!(
        manifest.contains(
            r#"features = ["classic-snowflake", "qubit-snowflake", "sonyflake", "uuid"]"#,
        ),
        "README dependency must enable every documented feature:\n{manifest}"
    );
}

/// Verifies that both READMEs explain feature selection, IoC injection,
/// deterministic clocks, generator lifetime, and UUID v4 output.
#[test]
fn test_readmes_document_feature_lifetime_and_benchmark_contracts() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let readmes = [
        (
            "README.md",
            "The default feature set enables only `qubit-snowflake`.",
            "`expires_at()` returns the exclusive expiration boundary.",
        ),
        (
            "README.zh_CN.md",
            "默认 feature 集合只启用 `qubit-snowflake`。",
            "`expires_at()` 返回排他的到期边界。",
        ),
    ];
    let common_fragments = [
        r#"default-features = false, features = ["classic-snowflake"]"#,
        r#"default-features = false, features = ["sonyflake"]"#,
        r#"default-features = false, features = ["uuid"]"#,
        "`now >= expires_at`",
        "`Arc<dyn IdGenerator<u64>>`",
        "`Arc<dyn AsyncIdGenerator<u64>>`",
        "`ManualMonotonicClock`",
        "`UuidV4Generator`",
        "`UuidV4StringGenerator`",
        "cargo bench --no-default-features --features uuid --bench uuid_comparison",
    ];

    for (name, feature_summary, lifetime_summary) in readmes {
        let content = fs::read_to_string(manifest_dir.join(name))
            .unwrap_or_else(|error| panic!("failed to read {name}: {error}"));
        for fragment in [feature_summary, lifetime_summary] {
            assert!(
                content.contains(fragment),
                "{name} must contain `{fragment}`"
            );
        }
        for fragment in common_fragments {
            assert!(
                content.contains(fragment),
                "{name} must contain `{fragment}`"
            );
        }
    }
}
