//! Pure source-shape audit for maintained fabric boundaries.

#![allow(
    tigerstyle::path_segment_repetition,
    tigerstyle::bool_naming,
    tigerstyle::unbounded_collection_growth,
    reason = "the bounded architecture auditor reports source-path categories and established Rust file names"
)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FabricSource<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FabricBoundaryIssue {
    AdapterOwnsPort { path: String },
    RawPortError { path: String },
    HostEffectInCore { path: String, token: String },
    PolicyDuplicatedInAdapter { path: String },
    ConcreteAdapterInCore { path: String, token: String },
}

const HOST_EFFECT_TOKENS: &[&str] = &[
    "std::fs::",
    "std::env::",
    "std::process::",
    "SystemTime::now",
    "Instant::now",
    "std::thread::sleep",
    "iroh::Endpoint",
    "redb::Database",
];

const CONCRETE_ADAPTER_TOKENS: &[&str] = &[
    "LiveClockAdapter::new",
    "OperatingSystemEntropySource::default",
    "IrohTransportAdapter::new",
    "RedbDurableStateAdapter::",
];

// r[impl molten.modularity.fabric_boundary.enforcement]
// r[impl molten.modularity.fabric_boundary.composition]
pub fn audit_fabric_boundaries(files: &[FabricSource<'_>]) -> Vec<FabricBoundaryIssue> {
    let mut issues = Vec::new();
    for file in files {
        let normalized = file.path.replace('\\', "/");
        let adapter = normalized.ends_with("/adapters.rs");
        let port = normalized.ends_with("/ports.rs");
        let pure_core = normalized.contains("crates/molten-core/") || normalized.contains("/core/");

        if adapter && file.text.contains("pub trait ") {
            issues.push(FabricBoundaryIssue::AdapterOwnsPort {
                path: normalized.clone(),
            });
        }
        if port && has_raw_string_result(file.text) {
            issues.push(FabricBoundaryIssue::RawPortError {
                path: normalized.clone(),
            });
        }
        if adapter && file.text.contains("fn adapter_policy_decision") {
            issues.push(FabricBoundaryIssue::PolicyDuplicatedInAdapter {
                path: normalized.clone(),
            });
        }
        if pure_core {
            for token in HOST_EFFECT_TOKENS {
                if file.text.contains(token) {
                    issues.push(FabricBoundaryIssue::HostEffectInCore {
                        path: normalized.clone(),
                        token: (*token).to_string(),
                    });
                }
            }
            for token in CONCRETE_ADAPTER_TOKENS {
                if file.text.contains(token) {
                    issues.push(FabricBoundaryIssue::ConcreteAdapterInCore {
                        path: normalized.clone(),
                        token: (*token).to_string(),
                    });
                }
            }
        }
    }
    issues
}

fn has_raw_string_result(text: &str) -> bool {
    text.lines().any(|line| {
        let compact = line.split_whitespace().collect::<String>();
        compact.contains("Result<") && compact.contains(",String>")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compliant_positive_fixtures_pass() {
        let files = [
            FabricSource {
                path: "src/fabric_time/adapters.rs",
                text: "pub struct ClockMechanism; impl ClockPort for ClockMechanism {}",
            },
            FabricSource {
                path: "src/fabric_time/ports.rs",
                text: "pub trait ClockPort { fn observe(&mut self) -> FabricPortResult<Tick>; }",
            },
            FabricSource {
                path: "crates/molten-core/src/fabric_time/core.rs",
                text: "pub fn decide(observation: Tick) -> Plan { Plan::from(observation) }",
            },
        ];

        assert_eq!(audit_fabric_boundaries(&files), Vec::new());
    }

    #[test]
    fn negative_fixtures_report_each_forbidden_boundary() {
        let files = [
            FabricSource {
                path: "src/fabric_time/adapters.rs",
                text: "pub trait ClockPort {}\nfn adapter_policy_decision() {}",
            },
            FabricSource {
                path: "src/fabric_transport/ports.rs",
                text: "fn send() -> Result<Receipt, String>;",
            },
            FabricSource {
                path: "crates/molten-core/src/fabric_time/core.rs",
                text: "let _ = std::fs::read(path); let _ = LiveClockAdapter::new(profile, 0);",
            },
        ];

        let issues = audit_fabric_boundaries(&files);

        assert!(issues.iter().any(|issue| matches!(issue, FabricBoundaryIssue::AdapterOwnsPort { .. })));
        assert!(issues.iter().any(|issue| matches!(issue, FabricBoundaryIssue::RawPortError { .. })));
        assert!(issues.iter().any(|issue| matches!(issue, FabricBoundaryIssue::HostEffectInCore { .. })));
        assert!(issues.iter().any(|issue| matches!(issue, FabricBoundaryIssue::PolicyDuplicatedInAdapter { .. })));
        assert!(issues.iter().any(|issue| matches!(issue, FabricBoundaryIssue::ConcreteAdapterInCore { .. })));
    }
}
