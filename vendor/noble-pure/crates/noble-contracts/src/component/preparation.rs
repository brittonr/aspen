impl super::World {
    /// Compile a source body on an isolated stack containing exactly the WIT
    /// parameters. The complete inferred normal stack must equal the WIT result.
    /// All requests retain their versioned effects; no host executes here.
    pub fn prepare_export(
        &self,
        name: &str,
        source_bytes: &[u8],
        limits: crate::Limits,
    ) -> Result<super::CheckedExport, super::Error> {
        let index = match self
            .exports
            .iter()
            .position(|operation| operation.export_name == name)
        {
            Some(index) => index,
            None => {
                return Err(super::error(
                    super::Stage::Export,
                    crate::DiagnosticKind::Invalid,
                    "unknown selected-world export",
                ))
            }
        };
        let operation = &self.exports[index];
        let bindings = attempt!(super::bindings::make(self));
        let callable_effects = bindings.effects;
        let session = crate::source::Session::with_bindings(bindings);
        let input = attempt!(operation.input_types(self).ok_or_else(|| super::error(
            super::Stage::Binding,
            crate::DiagnosticKind::Invalid,
            "unsigned WIT requires the selected checked boundary",
        )));
        let output = attempt!(operation.output_types(self).ok_or_else(|| super::error(
            super::Stage::Binding,
            crate::DiagnosticKind::Invalid,
            "unsigned WIT requires the selected checked boundary",
        )));
        let prepared = match session.prepare(source_bytes, &input, limits) {
            Ok(prepared) => prepared,
            Err(error) => {
                return Err(super::Error {
                    stage: super::Stage::Export,
                    diagnostic: error.diagnostic().clone(),
                })
            }
        };
        if prepared.is_definition() || prepared.output() != output {
            return Err(super::error(
                super::Stage::Export,
                crate::DiagnosticKind::Invalid,
                "Noble export does not have the closed exact WIT interface",
            ));
        }
        let submission = attempt!(prepared.submission().ok_or_else(|| super::error(
            super::Stage::Export,
            crate::DiagnosticKind::Internal,
            "missing checked export body",
        )));
        // Type-known latent Program effects never become guest-callable imports.
        if !submission
            .request
            .expected
            .allowed_effects
            .as_slice()
            .iter()
            .all(|effect| {
                1u64.checked_shl(effect.0)
                    .is_some_and(|bit| bit & callable_effects != 0)
            })
        {
            return Err(super::error(
                super::Stage::Export,
                crate::DiagnosticKind::Invalid,
                "Noble export effect exceeds selected WIT imports",
            ));
        }
        Ok(super::CheckedExport {
            world: self.build_context(),
            index,
            source_bytes: source_bytes.to_vec(),
            prepared,
        })
    }

    /// Static effect claim check for the generated operation. A reviewed-pure
    /// host still requires this exact WitOpId. Import availability grants no authority.
    pub fn check_import_effect(&self, word: &str, claimed: &[&str]) -> Result<(), super::Error> {
        if claimed.len() > super::MAX_OPERATIONS {
            return Err(super::exhausted());
        }
        let index = match self
            .imports
            .iter()
            .position(|operation| operation.word == word)
        {
            Some(index) => index,
            None => {
                return Err(super::error(
                    super::Stage::Binding,
                    crate::DiagnosticKind::Invalid,
                    "unknown generated import word",
                ))
            }
        };
        let operation = match self.imports.get(index) {
            Some(operation) => operation,
            None => {
                return Err(super::error(
                    super::Stage::Binding,
                    crate::DiagnosticKind::Invalid,
                    "unknown generated import word",
                ))
            }
        };
        if claimed
            .iter()
            .any(|identity| *identity == operation.identity)
        {
            Ok(())
        } else {
            Err(super::error(
                super::Stage::Acceptance,
                crate::DiagnosticKind::Invalid,
                "import request effect is absent from the claimed bound",
            ))
        }
    }
}
