
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricsSnapshotInput {
    pub node: String,
    pub scrape_ref: String,
    pub policy_refs: Vec<String>,
    pub redaction_refs: Vec<String>,
    pub samples: Vec<MetricSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricsSnapshotDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub openmetrics: String,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalDiagnosticsBridgeInput {
    pub enabled: bool,
    pub mode: String,
    pub target_service_ref: Option<String>,
    pub capability_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub redaction_policy_refs: Vec<String>,
    pub api_secret_provenance_ref: Option<String>,
    pub operator_evidence_refs: Vec<String>,
    pub expiry_ref: Option<String>,
}

pub fn empty_protocol_registry() -> ProtocolRegistry {
    ProtocolRegistry::default()
}

struct RouterMutation {
    registry: ProtocolRegistry,
    outcome: String,
    generation: Option<u64>,
    previous_generation: Option<u64>,
}

struct RouterEvaluation {
    mutation: RouterMutation,
    diagnostics: Vec<String>,
}

struct RouterEvaluator<'a> {
    registry: &'a ProtocolRegistry,
    input: &'a RouterOperationInput,
    diagnostics: DiagnosticLog,
}

impl<'a> RouterEvaluator<'a> {
    fn new(registry: &'a ProtocolRegistry, input: &'a RouterOperationInput) -> Self {
        Self {
            registry,
            input,
            diagnostics: DiagnosticLog::new(),
        }
    }

    fn evaluate(mut self) -> crate::error::Result<RouterEvaluation> {
        let is_dispatchable = self.collect_admission_diagnostics()?;
        let mutation = if is_dispatchable {
            self.dispatch_operation()?
        } else {
            self.denied(None)
        };
        Ok(RouterEvaluation {
            mutation,
            diagnostics: self.diagnostics.into_values(),
        })
    }

    fn collect_admission_diagnostics(&mut self) -> crate::error::Result<bool> {
        let is_alpn_valid = collect_alpn_diagnostic(&self.input.alpn, &mut self.diagnostics).is_ok();
        let is_handler_valid = collect_handler_diagnostic(&self.input.handler_kind, &mut self.diagnostics).is_ok();
        collect_ref_diagnostics(&self.input.authority_refs, "authority", &mut self.diagnostics)?;
        collect_ref_diagnostics(&self.input.policy_refs, "policy", &mut self.diagnostics)?;
        collect_ref_diagnostics(&self.input.resource_refs, "resource", &mut self.diagnostics)?;
        collect_ref_diagnostics(&self.input.evidence_refs, "evidence", &mut self.diagnostics)?;
        if self.input.generation < MIN_GENERATION || self.input.generation > MAX_GENERATION {
            push_diagnostic(
                &mut self.diagnostics,
                format!("generation {} outside supported range", self.input.generation),
            )?;
        }
        if !self.has_admission() && self.input.operation != "unsupported-alpn" {
            push_diagnostic(
                &mut self.diagnostics,
                "router operation requires authority, policy, resource, and evidence refs",
            )?;
        }
        Ok(is_alpn_valid && is_handler_valid && self.diagnostics.is_empty())
    }

    fn has_admission(&self) -> bool {
        !self.input.authority_refs.is_empty()
            && !self.input.policy_refs.is_empty()
            && !self.input.resource_refs.is_empty()
            && !self.input.evidence_refs.is_empty()
    }

    fn dispatch_operation(&mut self) -> crate::error::Result<RouterMutation> {
        match self.input.operation.as_str() {
            "install" => self.install_operation(),
            "replace" => self.replace_operation(),
            "remove" => self.remove_operation(),
            "unsupported-alpn" => self.unsupported_alpn_operation(),
            other => {
                push_diagnostic(&mut self.diagnostics, format!("unsupported router operation {other}"))?;
                Ok(self.denied(None))
            }
        }
    }

    fn install_operation(&mut self) -> crate::error::Result<RouterMutation> {
        if let Some(current_generation) = self.current_generation() {
            push_diagnostic(&mut self.diagnostics, "ALPN already registered; use replace with current generation")?;
            return Ok(self.denied(Some(current_generation)));
        }
        let descriptor = descriptor_from_input(self.input)?;
        let generation = Some(descriptor.generation);
        let mut registry = self.registry.clone();
        registry.handlers.insert(self.input.alpn.clone(), descriptor);
        Ok(RouterMutation {
            registry,
            outcome: "inserted".to_string(),
            generation,
            previous_generation: None,
        })
    }

    fn replace_operation(&mut self) -> crate::error::Result<RouterMutation> {
        let Some(current_generation) = self.current_generation() else {
            push_diagnostic(&mut self.diagnostics, "cannot replace unknown ALPN")?;
            return Ok(self.denied(None));
        };
        let previous_generation = Some(current_generation);
        if !self.is_replacement_admitted(current_generation)? {
            return Ok(self.denied(previous_generation));
        }
        let descriptor = descriptor_from_input(self.input)?;
        let generation = Some(descriptor.generation);
        let mut registry = self.registry.clone();
        registry.handlers.insert(self.input.alpn.clone(), descriptor);
        Ok(RouterMutation {
            registry,
            outcome: "replaced".to_string(),
            generation,
            previous_generation,
        })
    }

    fn remove_operation(&mut self) -> crate::error::Result<RouterMutation> {
        let Some(current_generation) = self.current_generation() else {
            push_diagnostic(&mut self.diagnostics, "cannot remove unknown ALPN")?;
            return Ok(self.denied(None));
        };
        let previous_generation = Some(current_generation);
        if !self.is_remove_admitted(current_generation)? {
            return Ok(self.denied(previous_generation));
        }
        let mut registry = self.registry.clone();
        registry.handlers.remove(&self.input.alpn);
        Ok(RouterMutation {
            registry,
            outcome: "removed".to_string(),
            generation: Some(current_generation),
            previous_generation,
        })
    }

    fn unsupported_alpn_operation(&mut self) -> crate::error::Result<RouterMutation> {
        if self.current_generation().is_some() {
            push_diagnostic(&mut self.diagnostics, "ALPN is registered; unsupported-alpn denial is not applicable")?;
            return Ok(self.denied(None));
        }
        push_diagnostic(&mut self.diagnostics, "unsupported ALPN denied before frame delivery")?;
        Ok(RouterMutation {
            registry: self.registry.clone(),
            outcome: "unsupported-alpn".to_string(),
            generation: None,
            previous_generation: None,
        })
    }

    fn current_generation(&self) -> Option<u64> {
        self.registry.handlers.get(&self.input.alpn).map(|handler| handler.generation)
    }

    fn is_replacement_admitted(&mut self, current_generation: u64) -> crate::error::Result<bool> {
        let expected_generation = Some(current_generation);
        if self.input.prior_generation != expected_generation {
            push_diagnostic(
                &mut self.diagnostics,
                "stale-generation: replacement prior generation does not match registry",
            )?;
            return Ok(false);
        }
        if self.input.generation <= current_generation {
            push_diagnostic(&mut self.diagnostics, "replacement generation must advance")?;
            return Ok(false);
        }
        let Some(shutdown_ref) = self.input.shutdown_evidence_ref.as_deref() else {
            push_diagnostic(&mut self.diagnostics, "replacement requires shutdown evidence for previous handler")?;
            return Ok(false);
        };
        validate_optional_ref(Some(shutdown_ref), "shutdown evidence")?;
        Ok(true)
    }

    fn is_remove_admitted(&mut self, current_generation: u64) -> crate::error::Result<bool> {
        let expected_generation = Some(current_generation);
        if self.input.prior_generation != expected_generation {
            push_diagnostic(
                &mut self.diagnostics,
                "stale-generation: remove prior generation does not match registry",
            )?;
            return Ok(false);
        }
        let Some(shutdown_ref) = self.input.shutdown_evidence_ref.as_deref() else {
            push_diagnostic(&mut self.diagnostics, "remove requires shutdown evidence for previous handler")?;
            return Ok(false);
        };
        validate_optional_ref(Some(shutdown_ref), "shutdown evidence")?;
        Ok(true)
    }

    fn denied(&self, previous_generation: Option<u64>) -> RouterMutation {
        RouterMutation {
            registry: self.registry.clone(),
            outcome: "denied".to_string(),
            generation: None,
            previous_generation,
        }
    }
}

pub fn evaluate_router_operation(
    registry: &ProtocolRegistry,
    input: &RouterOperationInput,
) -> crate::error::Result<RouterDecision> {
    let evaluation = RouterEvaluator::new(registry, input).evaluate()?;
    let decision = if evaluation.diagnostics.is_empty() {
        "pass"
    } else {
        "deny"
    }
    .to_string();
    let receipt_value = router_receipt_value(RouterReceiptInput {
        decision: &decision,
        operation: &input.operation,
        outcome: &evaluation.mutation.outcome,
        alpn: &input.alpn,
        handler_kind: &input.handler_kind,
        generation: evaluation.mutation.generation,
        previous_generation: evaluation.mutation.previous_generation,
        authority_refs: &input.authority_refs,
        policy_refs: &input.policy_refs,
        resource_refs: &input.resource_refs,
        evidence_refs: &input.evidence_refs,
        shutdown_evidence_ref: input.shutdown_evidence_ref.as_deref(),
        diagnostics: &evaluation.diagnostics,
    })?;
    Ok(RouterDecision {
        decision,
        operation: input.operation.clone(),
        alpn: input.alpn.clone(),
        outcome: evaluation.mutation.outcome,
        generation: evaluation.mutation.generation,
        previous_generation: evaluation.mutation.previous_generation,
        diagnostics: evaluation.diagnostics,
        registry: evaluation.mutation.registry,
        receipt_value,
    })
}

struct FrameEvaluation {
    actual_ref: Option<String>,
    diagnostics: Vec<String>,
}

struct FrameEvaluator<'a> {
    registry: &'a ProtocolRegistry,
    input: &'a FramedEnvelopeInput,
    diagnostics: DiagnosticLog,
}
