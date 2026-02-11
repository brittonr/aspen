//! Watch mode for continuous validation.
//!
//! Monitors src/verified/ and verus/ directories for changes and
//! automatically re-runs validation when files are modified.

use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;

use crate::Severity;
use crate::VerificationEngine;
use crate::output::OutputFormat;
use crate::output::render;

/// Run the watch mode loop.
pub fn run_watch(root_dir: &Path, crate_names: &[String], debounce_ms: u64, clear_screen: bool) -> Result<()> {
    let (tx, rx) = channel();

    // Create a debouncer that waits for file system activity to settle
    let mut debouncer =
        new_debouncer(Duration::from_millis(debounce_ms), tx).context("Failed to create file watcher")?;

    // Watch all verified and verus directories
    for crate_name in crate_names {
        let crate_dir = root_dir.join("crates").join(crate_name);
        let verified_dir = crate_dir.join("src/verified");
        let verus_dir = crate_dir.join("verus");

        if verified_dir.exists() {
            debouncer
                .watcher()
                .watch(&verified_dir, RecursiveMode::Recursive)
                .context(format!("Failed to watch {}", verified_dir.display()))?;
        }

        if verus_dir.exists() {
            debouncer
                .watcher()
                .watch(&verus_dir, RecursiveMode::Recursive)
                .context(format!("Failed to watch {}", verus_dir.display()))?;
        }
    }

    println!("Watching for changes... (press Ctrl+C to stop)");
    println!();

    // Run initial check
    run_check(root_dir, crate_names, clear_screen)?;

    // Watch loop
    loop {
        match rx.recv() {
            Ok(result) => match result {
                Ok(events) => {
                    // Filter to only .rs files
                    let rs_changes = events.iter().any(|e| e.path.extension().is_some_and(|ext| ext == "rs"));

                    if rs_changes {
                        run_check(root_dir, crate_names, clear_screen)?;
                    }
                }
                Err(error) => {
                    eprintln!("Watch error: {}", error);
                }
            },
            Err(e) => {
                eprintln!("Channel error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Run a single check cycle.
fn run_check(root_dir: &Path, crate_names: &[String], clear_screen: bool) -> Result<()> {
    if clear_screen {
        // ANSI escape sequence to clear screen and move cursor to top
        print!("\x1B[2J\x1B[1;1H");
    }

    println!("[{}] Running Verus sync check...", chrono_lite_timestamp());
    println!();

    let engine = VerificationEngine::new(root_dir.to_path_buf(), crate_names.to_vec(), false);

    match engine.verify() {
        Ok(report) => {
            let output = render(&report, OutputFormat::Terminal, false, Severity::Warning);
            print!("{}", output);
        }
        Err(e) => {
            eprintln!("Verification error: {:#}", e);
        }
    }

    println!();
    println!("Watching for changes...");

    Ok(())
}

/// Generate a simple timestamp without requiring chrono crate.
fn chrono_lite_timestamp() -> String {
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();

    let secs = now.as_secs();

    // Very basic time formatting
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_format() {
        let ts = chrono_lite_timestamp();
        assert_eq!(ts.len(), 8);
        assert!(ts.contains(':'));
    }
}
