//! Parsing utilities for OpenTofu/Terraform output
//!
//! This module provides functions to parse resource summaries from
//! OpenTofu command outputs.

/// Parse plan output for resource summary
///
/// Returns (created, updated, destroyed) counts
pub fn parse_plan_summary(output: &str) -> (i32, i32, i32) {
    let mut created = 0;
    let mut updated = 0;
    let mut destroyed = 0;

    // Look for the summary line in OpenTofu output
    // Example: "Plan: 3 to add, 2 to change, 1 to destroy."
    for line in output.lines() {
        if line.contains("Plan:") {
            // Parse the numbers
            if let Some(add_match) = line.find(" to add") {
                let start = line[..add_match].rfind(' ').map(|i| i + 1).unwrap_or(0);
                if let Ok(num) = line[start..add_match].trim().parse::<i32>() {
                    created = num;
                }
            }
            if let Some(change_match) = line.find(" to change") {
                let start = line[..change_match].rfind(' ').map(|i| i + 1).unwrap_or(0);
                if let Ok(num) = line[start..change_match].trim().parse::<i32>() {
                    updated = num;
                }
            }
            if let Some(destroy_match) = line.find(" to destroy") {
                let start = line[..destroy_match].rfind(' ').map(|i| i + 1).unwrap_or(0);
                if let Ok(num) = line[start..destroy_match].trim().parse::<i32>() {
                    destroyed = num;
                }
            }
            break;
        }
    }

    (created, updated, destroyed)
}

/// Parse apply output for resource summary
///
/// Returns (created, updated, destroyed) counts
pub fn parse_apply_summary(output: &str) -> (i32, i32, i32) {
    let mut created = 0;
    let mut updated = 0;
    let mut destroyed = 0;

    // Look for the summary line in OpenTofu apply output
    // Example: "Apply complete! Resources: 3 added, 2 changed, 1 destroyed."
    for line in output.lines() {
        if line.contains("Apply complete!") && line.contains("Resources:") {
            // Parse the numbers
            if let Some(added_match) = line.find(" added") {
                let start = line[..added_match].rfind(' ').map(|i| i + 1).unwrap_or(0);
                if let Ok(num) = line[start..added_match].trim().parse::<i32>() {
                    created = num;
                }
            }
            if let Some(changed_match) = line.find(" changed") {
                let start = line[..changed_match].rfind(' ').map(|i| i + 1).unwrap_or(0);
                if let Ok(num) = line[start..changed_match].trim().parse::<i32>() {
                    updated = num;
                }
            }
            if let Some(destroyed_match) = line.find(" destroyed") {
                let start = line[..destroyed_match].rfind(' ').map(|i| i + 1).unwrap_or(0);
                if let Ok(num) = line[start..destroyed_match].trim().parse::<i32>() {
                    destroyed = num;
                }
            }
            break;
        }
    }

    (created, updated, destroyed)
}
