// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Validates TOML configuration blocks in Markdown documentation.

use std::fs;
use std::path::PathBuf;

const DOCUMENTS: &[&str] = &[
    "README.md",
    "README.zh_CN.md",
    "doc/user_guide.md",
    "doc/user_guide.zh_CN.md",
];

fn validate_toml_fences(path: &str, source: &str) -> Result<(), String> {
    let mut fence_start = None;
    let mut is_toml = false;
    let mut contents = String::new();

    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        if fence_start.is_none() {
            let trimmed = line.trim_start();
            if let Some(info) = trimmed.strip_prefix("```") {
                fence_start = Some(line_number);
                is_toml = info.trim() == "toml";
                contents.clear();
            }
        } else if line.trim() == "```" {
            let start = fence_start.take().expect("open fence has a start line");
            if is_toml {
                toml::from_str::<toml::Table>(&contents)
                    .map_err(|error| format!("{path}:{start}: invalid TOML fence: {error}"))?;
            }
            is_toml = false;
            contents.clear();
        } else if is_toml {
            contents.push_str(line);
            contents.push('\n');
        }
    }

    if let Some(start) = fence_start {
        return Err(format!("{path}:{start}: unclosed Markdown fence"));
    }
    Ok(())
}

#[test]
fn test_toml_configuration_documents_parse() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative_path in DOCUMENTS {
        let path = manifest_dir.join(relative_path);
        let source =
            fs::read_to_string(&path).unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        validate_toml_fences(relative_path, &source).unwrap_or_else(|error| panic!("{error}"));
    }
}

#[test]
fn test_toml_configuration_rejects_duplicate_dependency_keys() {
    let source = "```toml\n[dependencies]\nqubit-id = \"0.7\"\nqubit-id = \"0.7\"\n```\n";
    let error = validate_toml_fences("duplicate.md", source).expect_err("duplicate key is invalid TOML");
    assert!(
        error.contains("duplicate.md:1"),
        "error should identify fence location: {error}"
    );
    assert!(
        error.contains("invalid TOML"),
        "error should identify the parse failure: {error}"
    );
}

#[test]
fn test_toml_configuration_rejects_unclosed_fence() {
    let error = validate_toml_fences("unclosed.md", "```toml\nx = 1\n").expect_err("fence is unclosed");
    assert!(
        error.contains("unclosed.md:1"),
        "error should identify fence location: {error}"
    );
    assert!(
        error.contains("unclosed"),
        "error should identify the fence error: {error}"
    );
}

#[test]
fn test_toml_configuration_accepts_independent_dependency_blocks() {
    let source =
        "```toml\n[dependencies]\nqubit-id = \"0.7\"\n```\n\n```toml\n[dependencies]\nqubit-id = \"0.7\"\n```\n";
    validate_toml_fences("valid.md", source).expect("each dependency block is independent");
}
