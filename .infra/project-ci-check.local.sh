#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd -- "$project_root"
cargo test --locked --test mod --no-default-features test_review_documentation_examples -- --ignored --nocapture
