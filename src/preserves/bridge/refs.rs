//! Content-reference and optional-reference rules from the kernel, reported with the shell's
//! established diagnostics.

/// The shell diagnostic for a string the kernel rejected as a content reference.
pub(super) fn ref_failure(value: &str, error: molten_preserves_core::identity::RefError) -> super::MoltenError {
    let prefix = molten_preserves_core::identity::BLAKE3_REF_PREFIX;
    super::MoltenError::invalid_harness(match error {
        molten_preserves_core::identity::RefError::MissingPrefix => {
            format!("content ref must start with {prefix}, got {value}")
        }
        molten_preserves_core::identity::RefError::DigestLength => {
            format!("content ref must be {prefix}<64 lowercase hex chars>, got {value}")
        }
        molten_preserves_core::identity::RefError::DigestCharacter => {
            format!("content ref must use lowercase hex chars, got {value}")
        }
    })
}

/// Why a value is not `<none>` or `<some "blake3:...">`.
pub(super) enum OptionalRefIssue {
    Shape,
    NotString,
    /// The `<some ...>` string is not a canonical reference; carries the reference diagnostic.
    InvalidRef(super::MoltenError),
    Conversion(super::MoltenError),
}

/// Decodes the single canonical optional-reference encoding with the kernel rule.
// r[impl molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
pub(super) fn decode_optional_ref(value: &super::IoValue) -> Result<Option<super::ContentRef>, OptionalRefIssue> {
    let packed = super::value_bridge::packed_from_iovalue(value).map_err(OptionalRefIssue::Conversion)?;
    match molten_preserves_core::identity::decode_optional_ref(&packed) {
        Ok(reference) => Ok(reference.map(super::ContentRef::from)),
        Err(molten_preserves_core::identity::OptionalRefError::Shape) => Err(OptionalRefIssue::Shape),
        Err(molten_preserves_core::identity::OptionalRefError::NotString) => Err(OptionalRefIssue::NotString),
        Err(molten_preserves_core::identity::OptionalRefError::InvalidRef(error)) => {
            let text = packed
                .symbol_record_fields(molten_preserves_core::identity::SOME_LABEL, 1)
                .and_then(<[_]>::first)
                .and_then(molten_preserves_core::model::PackedValue::as_str)
                .ok_or(OptionalRefIssue::Shape)?;
            Err(OptionalRefIssue::InvalidRef(ref_failure(text, error)))
        }
    }
}
