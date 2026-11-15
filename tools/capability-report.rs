#!/usr/bin/env -S cargo -q -Zscript
---
[dependencies]
molten = { path = ".." }
serde = { version = "=1.0.228", features = ["derive"] }
serde_json = "=1.0.149"
tempfile = "3"
---
//! Diagnostic readback, never an execution or release authority.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[path = "capability-report/model.rs"]
mod model;
#[path = "capability-report/artifacts.rs"]
mod artifacts;
#[path = "capability-report/report.rs"]
mod report;
#[path = "capability-report/controls.rs"]
mod controls;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("capability report denied: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut matrix = None;
    let mut input = None;
    let mut mapping = PathBuf::from("config/capability-report/capabilities.json");
    let mut markdown_out = None;
    let mut json_out = None;
    let mut canonical_out = None;
    while let Some(flag) = args.next() {
        if flag == "--self-test" {
            return controls::run();
        }
        let value = args.next().ok_or_else(|| format!("{flag} requires a path"))?;
        match flag.as_str() {
            "--matrix" => matrix = Some(PathBuf::from(value)),
            "--input" => input = Some(PathBuf::from(value)),
            "--mapping" => mapping = PathBuf::from(value),
            "--markdown-out" => markdown_out = Some(PathBuf::from(value)),
            "--json-out" => json_out = Some(PathBuf::from(value)),
            "--canonical-out" => canonical_out = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown argument: {flag}")),
        }
    }
    let matrix = matrix.ok_or("missing --matrix")?;
    let input_path = input.ok_or("missing --input")?;
    let mut input: model::Input = read_json(&input_path)?;
    report::validate_subject(&input.subject)?;
    input.subject.feature_set.sort(); input.subject.adapter_refs.sort();
    for capability in &mut input.capabilities {
        for evidence in &mut capability.evidence {
            evidence.subject.feature_set.sort(); evidence.subject.adapter_refs.sort();
        }
    }
    let mapping: model::Mapping = read_json(&mapping)?;
    let matrix: model::Matrix = read_json(&matrix)?;
    let observations = artifacts::read_bundle(&input, input_path.parent().unwrap_or(Path::new(".")));
    let result = report::build(&matrix, &mapping, &input, &observations)?;
    let json = serde_json::to_value(&result).map_err(|error| error.to_string())?;
    let canonical = molten::live_reliability::checker::canonical_value(&json)?;
    let report_ref = molten::preserves_rail::canonical_hash(&canonical).map_err(|error| error.to_string())?;
    let markdown = report::markdown(&result, &report_ref);
    if let Some(path) = markdown_out { write_file(&path, markdown.as_bytes())?; }
    if let Some(path) = json_out {
        let mut bytes = serde_json::to_vec_pretty(&json).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        write_file(&path, &bytes)?;
    }
    if let Some(path) = canonical_out {
        write_file(&path, &molten::preserves_rail::canonical_bytes(&canonical).map_err(|error| error.to_string())?)?;
    }
    print!("{markdown}");
    Ok(())
}

fn read_json<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
}
