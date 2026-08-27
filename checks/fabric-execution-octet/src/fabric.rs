#![allow(
    tigerstyle::path_segment_repetition,
    reason = "the focused shim preserves exact Molten fabric contract names for the real execution core"
)]

const REQUIRED_NON_CLAIM_COUNT: usize = 9;
const MAX_FABRIC_TEXT_CHARS: usize = 256;
const BLAKE3_REF_PREFIX: &str = "blake3:";
const BLAKE3_HEX_CHAR_COUNT: usize = 64;
const BLAKE3_REF_CHAR_COUNT: usize = BLAKE3_REF_PREFIX.len() + BLAKE3_HEX_CHAR_COUNT;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FabricNonClaim {
    DatabaseCorrectness,
    GlobalOrdering,
    GlobalConsensus,
    TransportDelivery,
    DurablePersistence,
    ByzantineTolerance,
    ProtocolCompatibility,
    ProductionReadiness,
    ExtensionSemanticCorrectness,
}

pub const REQUIRED_FABRIC_NON_CLAIMS: [FabricNonClaim; REQUIRED_NON_CLAIM_COUNT] = [
    FabricNonClaim::DatabaseCorrectness,
    FabricNonClaim::GlobalOrdering,
    FabricNonClaim::GlobalConsensus,
    FabricNonClaim::TransportDelivery,
    FabricNonClaim::DurablePersistence,
    FabricNonClaim::ByzantineTolerance,
    FabricNonClaim::ProtocolCompatibility,
    FabricNonClaim::ProductionReadiness,
    FabricNonClaim::ExtensionSemanticCorrectness,
];

pub(crate) fn valid_fabric_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_FABRIC_TEXT_CHARS
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':'))
}

pub(crate) fn valid_blake3_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return false;
    };
    value.len() == BLAKE3_REF_CHAR_COUNT
        && hex.len() == BLAKE3_HEX_CHAR_COUNT
        && hex.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
}
