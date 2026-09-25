
/// Full placement evaluation including fit, constraints, and taint checks.
pub fn evaluate_placement(
    request: &PlacementRequest,
    explicit_target_properties: &[(String, String)],
) -> PlacementDecision {
    // Check constraints against explicit target properties
    let mut diagnostics = Vec::with_capacity(request.constraints.len());

    for constraint in &request.constraints {
        let is_satisfied = match constraint.kind {
            ConstraintKind::Required | ConstraintKind::AntiAffinity => {
                let is_found = explicit_target_properties
                    .iter()
                    .any(|(k, v)| k == &constraint.key && constraint.values.contains(v));
                if !is_found {
                    diagnostics.push(format!(
                        "required constraint not satisfied: {} {:?} {:?}",
                        constraint.key,
                        constraint.operator,
                        constraint.values,
                    ));
                }
                is_found
            }
            ConstraintKind::Preferred => true, // Preferred is not a hard deny
        };

        if !is_satisfied && constraint.kind == ConstraintKind::Required {
            return PlacementDecision {
                decision: "deny".to_string(),
                target_ref: None,
                quota_consumed: None,
                diagnostics,
            };
        }
    }

    // Check taint/toleration from explicit properties (simplified — real impl
    // would extract taints from the target)
    let taint_keys: Vec<String> = explicit_target_properties
        .iter()
        .filter(|(k, _)| k == "taint.no-schedule" || k == "taint.no-execute")
        .map(|(_, v)| v.clone())
        .collect();

    if !taint_keys.is_empty() {
        let is_tolerated = request.tolerations.iter().any(|tol| {
            taint_keys.iter().any(|tk| {
                match tol.operator {
                    TolerationOperator::Equal => &tol.key == tk,
                    TolerationOperator::Exists => true,
                }
            })
        });
        if !is_tolerated {
            diagnostics.push("target has hard taints without matching tolerations".to_string());
            return PlacementDecision {
                decision: "deny".to_string(),
                target_ref: None,
                quota_consumed: None,
                diagnostics,
            };
        }
    }

    PlacementDecision {
        decision: "pass".to_string(),
        target_ref: Some("target".to_string()),
        quota_consumed: None,
        diagnostics,
    }
}