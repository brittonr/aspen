#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Core,
    Codec,
    PolicyEvidence,
    Runtime,
    Adapter,
    Cli,
    Test,
    Integration,
    PublicApi,
    Generated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryRule<'a> {
    pub id: &'a str,
    pub owning_layer: Layer,
    pub source_prefix: &'a str,
    pub denied_target_prefixes: &'a [&'a str],
    pub guidance: &'a str,
    pub exemption_class: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFact<'a> {
    pub source_file: &'a str,
    pub target: &'a str,
    pub is_public_export: bool,
    pub exemption: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryDiagnostic {
    pub rule_id: String,
    pub source_file: String,
    pub forbidden_target: String,
    pub guidance: String,
}

pub fn validate_dependency_boundaries(
    rules: &[BoundaryRule<'_>],
    imports: &[ImportFact<'_>],
) -> Vec<BoundaryDiagnostic> {
    let mut diagnostics = Vec::new();
    for import in imports {
        for rule in rules {
            if import_is_exempt(import, rule) {
                continue;
            }
            if !import.source_file.starts_with(rule.source_prefix) {
                continue;
            }
            if !target_is_denied(import.target, rule.denied_target_prefixes) {
                continue;
            }
            diagnostics.push(BoundaryDiagnostic {
                rule_id: rule.id.to_string(),
                source_file: import.source_file.to_string(),
                forbidden_target: import.target.to_string(),
                guidance: rule.guidance.to_string(),
            });
        }
    }
    diagnostics
}

fn import_is_exempt(import: &ImportFact<'_>, rule: &BoundaryRule<'_>) -> bool {
    import.exemption.is_some_and(|exemption| exemption == rule.exemption_class)
}

fn target_is_denied(target: &str, denied_target_prefixes: &[&str]) -> bool {
    denied_target_prefixes.iter().any(|prefix| target.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORE_RULE_ID: &str = "core-no-adapter";
    const RUNTIME_RULE_ID: &str = "runtime-no-cli";
    const CORE_PREFIX: &str = "crates/molten-core/src/";
    const RUNTIME_PREFIX: &str = "src/runtime/";
    const ADAPTER_PREFIX: &str = "src/adapters/";
    const CLI_PREFIX: &str = "src/main.rs";
    const EXEMPTION_CLASS: &str = "generated-boundary-shim";

    fn rules() -> [BoundaryRule<'static>; 2] {
        [
            BoundaryRule {
                id: CORE_RULE_ID,
                owning_layer: Layer::Core,
                source_prefix: CORE_PREFIX,
                denied_target_prefixes: &[ADAPTER_PREFIX, CLI_PREFIX],
                guidance: "move IO behind an adapter port and pass parsed facts into the core",
                exemption_class: EXEMPTION_CLASS,
            },
            BoundaryRule {
                id: RUNTIME_RULE_ID,
                owning_layer: Layer::Runtime,
                source_prefix: RUNTIME_PREFIX,
                denied_target_prefixes: &[CLI_PREFIX],
                guidance: "runtime code must return plans instead of depending on CLI shells",
                exemption_class: EXEMPTION_CLASS,
            },
        ]
    }

    #[test]
    fn dependency_boundary_accepts_allowed_imports() {
        let imports = [ImportFact {
            source_file: "crates/molten-core/src/planning.rs",
            target: "crates/molten-core/src/policy.rs",
            is_public_export: false,
            exemption: None,
        }];

        assert!(validate_dependency_boundaries(&rules(), &imports).is_empty());
    }

    #[test]
    fn dependency_boundary_reports_forbidden_target_with_rule_context() {
        let imports = [ImportFact {
            source_file: "crates/molten-core/src/planning.rs",
            target: "src/adapters/redb.rs",
            is_public_export: false,
            exemption: None,
        }];

        let diagnostics = validate_dependency_boundaries(&rules(), &imports);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, CORE_RULE_ID);
        assert_eq!(diagnostics[0].forbidden_target, "src/adapters/redb.rs");
        assert!(diagnostics[0].guidance.contains("adapter port"));
    }

    #[test]
    fn dependency_boundary_honors_reviewed_exemption_class() {
        let imports = [ImportFact {
            source_file: "crates/molten-core/src/generated.rs",
            target: "src/adapters/redb.rs",
            is_public_export: false,
            exemption: Some(EXEMPTION_CLASS),
        }];

        assert!(validate_dependency_boundaries(&rules(), &imports).is_empty());
    }
}
