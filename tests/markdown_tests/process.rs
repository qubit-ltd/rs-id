// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::fs::File;
use std::io::Read;
use std::process::Child;
use std::process::Command;
use std::process::ExitStatus;
use std::thread;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug)]
pub(super) struct ProcessOutcome {
    pub(super) status: ExitStatus,
    pub(super) log_path: std::path::PathBuf,
}

pub(super) fn run_with_timeout(
    mut command: Command,
    budget: Duration,
    stage: &str,
    log_path: &std::path::Path,
) -> Result<ProcessOutcome, String> {
    let log = File::create(log_path).map_err(|error| format!("{stage}: create log {}: {error}", log_path.display()))?;
    let stderr = log
        .try_clone()
        .map_err(|error| format!("{stage}: clone log handle {}: {error}", log_path.display()))?;
    command.stdout(log).stderr(stderr);
    let mut child = command
        .spawn()
        .map_err(|error| format!("{stage}: spawn failed: {error}; log: {}", log_path.display()))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(ProcessOutcome {
                    status,
                    log_path: log_path.to_owned(),
                });
            }
            Ok(None) if started.elapsed() < budget => thread::sleep(Duration::from_millis(10)),
            Ok(None) => return terminate_timed_out_child(&mut child, stage, log_path),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{stage}: wait failed: {error}; log: {}", log_path.display()));
            }
        }
    }
}

fn terminate_timed_out_child(
    child: &mut Child,
    stage: &str,
    log_path: &std::path::Path,
) -> Result<ProcessOutcome, String> {
    child
        .kill()
        .map_err(|error| format!("{stage}: timeout kill failed: {error}; log: {}", log_path.display()))?;
    let status = child
        .wait()
        .map_err(|error| format!("{stage}: timeout reap failed: {error}; log: {}", log_path.display()))?;
    let tail = read_log_tail(log_path);
    Err(format!(
        "{stage}: timed out and direct child was reaped ({status}); log: {}\n{tail}",
        log_path.display()
    ))
}

fn read_log_tail(path: &std::path::Path) -> String {
    let mut content = Vec::new();
    if File::open(path)
        .and_then(|mut file| file.read_to_end(&mut content))
        .is_err()
    {
        return "(unable to read child log)".to_owned();
    }
    let start = content.len().saturating_sub(8 * 1024);
    String::from_utf8_lossy(&content[start..]).into_owned()
}
