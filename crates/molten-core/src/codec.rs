use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainArtifactInput<'a> {
    pub domain: &'a str,
    pub label: &'a str,
    pub schema: &'a str,
    pub artifact_ref: &'a str,
    pub expected_schema: &'a str,
    pub supported_labels: &'a [&'a str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainArtifactSummary {
    pub domain: String,
    pub label: String,
    pub artifact_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecIssue {
    EmptyDomain,
    UnsupportedLabel(String),
    SchemaMismatch { actual: String, expected: String },
    MalformedArtifactRef(String),
}

const BLAKE3_REF_PREFIX: &str = "blake3:";
const BLAKE3_HEX_CHAR_COUNT: usize = 64;
const BLAKE3_REF_CHAR_COUNT: usize = BLAKE3_REF_PREFIX.len() + BLAKE3_HEX_CHAR_COUNT;

pub fn validate_domain_artifact(input: &DomainArtifactInput<'_>) -> Result<DomainArtifactSummary, CodecIssue> {
    if input.domain.is_empty() {
        return Err(CodecIssue::EmptyDomain);
    }
    if !input.supported_labels.contains(&input.label) {
        return Err(CodecIssue::UnsupportedLabel(input.label.to_string()));
    }
    if input.schema != input.expected_schema {
        return Err(CodecIssue::SchemaMismatch {
            actual: input.schema.to_string(),
            expected: input.expected_schema.to_string(),
        });
    }
    if !valid_blake3_ref(input.artifact_ref) {
        return Err(CodecIssue::MalformedArtifactRef(input.artifact_ref.to_string()));
    }
    Ok(DomainArtifactSummary {
        domain: input.domain.to_string(),
        label: input.label.to_string(),
        artifact_ref: input.artifact_ref.to_string(),
    })
}

fn valid_blake3_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return false;
    };
    value.len() == BLAKE3_REF_CHAR_COUNT && hex.chars().all(|character| character.is_ascii_hexdigit())
}

impl fmt::Display for CodecIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDomain => write!(formatter, "domain must not be empty"),
            Self::UnsupportedLabel(label) => write!(formatter, "unsupported domain artifact label {label}"),
            Self::SchemaMismatch { actual, expected } => {
                write!(formatter, "schema mismatch: got {actual}, expected {expected}")
            }
            Self::MalformedArtifactRef(artifact_ref) => write!(formatter, "malformed artifact ref {artifact_ref}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHUNK_DOMAIN: &str = "chunk-store";
    const CHUNK_MANIFEST_LABEL: &str = "chunk-manifest-v1";
    const CHUNK_MANIFEST_SCHEMA: &str = "molten.chunk-store.manifest.v1";
    const UNSUPPORTED_LABEL: &str = "chunk-manifest-v2";
    const WRONG_SCHEMA: &str = "molten.chunk-store.manifest.v2";
    const VALID_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const MALFORMED_REF: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn valid_input() -> DomainArtifactInput<'static> {
        DomainArtifactInput {
            domain: CHUNK_DOMAIN,
            label: CHUNK_MANIFEST_LABEL,
            schema: CHUNK_MANIFEST_SCHEMA,
            artifact_ref: VALID_REF,
            expected_schema: CHUNK_MANIFEST_SCHEMA,
            supported_labels: &[CHUNK_MANIFEST_LABEL],
        }
    }

    #[test]
    fn domain_codec_facade_accepts_supported_artifact_identity() {
        let summary = validate_domain_artifact(&valid_input()).expect("valid domain artifact");

        assert_eq!(summary.domain, CHUNK_DOMAIN);
        assert_eq!(summary.label, CHUNK_MANIFEST_LABEL);
        assert_eq!(summary.artifact_ref, VALID_REF);
    }

    #[test]
    fn domain_codec_facade_rejects_wrong_label_schema_and_ref() {
        let mut wrong_label = valid_input();
        wrong_label.label = UNSUPPORTED_LABEL;
        assert_eq!(
            validate_domain_artifact(&wrong_label),
            Err(CodecIssue::UnsupportedLabel(UNSUPPORTED_LABEL.to_string()))
        );

        let mut wrong_schema = valid_input();
        wrong_schema.schema = WRONG_SCHEMA;
        assert_eq!(
            validate_domain_artifact(&wrong_schema),
            Err(CodecIssue::SchemaMismatch {
                actual: WRONG_SCHEMA.to_string(),
                expected: CHUNK_MANIFEST_SCHEMA.to_string(),
            })
        );

        let mut malformed_ref = valid_input();
        malformed_ref.artifact_ref = MALFORMED_REF;
        assert_eq!(
            validate_domain_artifact(&malformed_ref),
            Err(CodecIssue::MalformedArtifactRef(MALFORMED_REF.to_string()))
        );
    }

    #[test]
    fn domain_codec_facade_rejects_missing_domain() {
        let mut missing_domain = valid_input();
        missing_domain.domain = "";

        assert_eq!(validate_domain_artifact(&missing_domain), Err(CodecIssue::EmptyDomain));
    }
}
