/// Constructed only by the authenticated startup shell, never from an evidence-carried key.
#[derive(Debug, Clone)]
pub struct ValidatedNodeSourceGate {
    pub(crate) binding_ref: String,
    pub(crate) gate_ref: String,
    pub(crate) validation_ref: String,
}

impl ValidatedNodeSourceGate {
    pub fn binding_ref(&self) -> &str {
        &self.binding_ref
    }

    pub fn validation_ref(&self) -> &str {
        &self.validation_ref
    }

    pub fn gate_ref(&self) -> &str {
        &self.gate_ref
    }
}

fn admitted_gate_scan(input: &NodeRuntimeStartInput, gate: &ValidatedNodeSourceGate) -> Result<GateScan> {
    if input.source_gate_receipt_refs.len() != 1
        || input.source_gate_receipt_refs[0] != gate.gate_ref
        || input.source_gate_receipt_values.len() != 1
        || canonical_hash(&input.source_gate_receipt_values[0])? != gate.gate_ref
    {
        return Err(MoltenError::invalid_harness("admitted source gate does not match runtime input"));
    }
    Ok(GateScan { validation_refs: vec![gate.validation_ref.clone()], diagnostics: Vec::new() })
}
