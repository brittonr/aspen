pub const ID: &str = "spacewasm-core-mvp-pilot";
pub const REVISION: &str = "e24cf09355a90497148eb5029fdb8e3400bd63e3";
pub const ABI: &str = "molten.wasm.abi.v1";
pub const NON_CLAIMS: &[&str] = &[
    "not-component-compatible",
    "not-portable-state",
    "not-cross-host-migration",
    "not-runtime-correctness",
    "not-sandbox-completeness",
    "not-production-ready",
    "not-release-eligible",
];

// Match the integer and transfer envelopes in config/spacewasm/contracts.ncl.
const MAX_CHECKED_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_TRANSFER_BYTES: u64 = 16 * 1024 * 1024;
const MAX_CONTINUATIONS: u32 = 65_536;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub allocation_bytes: u64,
    pub ir_bytes: u64,
    pub stack_bytes: u64,
    pub memory_pages: u32,
    pub table_elements: u32,
    pub segment_fuel: u64,
    pub total_fuel: u64,
    pub hostcall_bytes: u64,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub continuation_count: u32,
    pub continuation_bytes: u64,
    pub retention_turns: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            allocation_bytes: 33_554_432,
            ir_bytes: 65_536,
            stack_bytes: 524_288,
            memory_pages: 256,
            table_elements: 0,
            segment_fuel: 4096,
            total_fuel: 1_048_576,
            hostcall_bytes: 65_536,
            continuation_count: 16,
            input_bytes: 65_536,
            output_bytes: 65_536,
            continuation_bytes: 536_870_912,
            retention_turns: 64,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    pub source_revision: String,
    pub abi: String,
    pub limits: Limits,
    pub production: bool,
    pub release: bool,
    pub non_claims: Vec<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: ID.into(),
            source_revision: REVISION.into(),
            abi: ABI.into(),
            limits: Limits::default(),
            production: false,
            release: false,
            non_claims: NON_CLAIMS.iter().map(|value| (*value).into()).collect(),
        }
    }
}

impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        if self.name != ID || self.source_revision != REVISION || self.abi != ABI {
            return Err("profile identity or source cohort mismatch".into());
        }
        if self.production
            || self.release
            || self.non_claims != NON_CLAIMS.iter().map(|value| (*value).to_owned()).collect::<Vec<_>>()
        {
            return Err("unsupported pilot promotion or missing non-claims".into());
        }
        let l = &self.limits;
        let bounds = [
            (l.allocation_bytes, i32::MAX as u64),
            (l.ir_bytes, 65_536),
            (l.stack_bytes, i32::MAX as u64),
            (l.segment_fuel, MAX_CHECKED_INTEGER),
            (l.total_fuel, MAX_CHECKED_INTEGER),
            (l.hostcall_bytes, MAX_TRANSFER_BYTES),
            (l.input_bytes, MAX_TRANSFER_BYTES),
            (l.output_bytes, MAX_TRANSFER_BYTES),
            (l.continuation_bytes, MAX_CHECKED_INTEGER),
            (l.retention_turns, MAX_CHECKED_INTEGER),
        ];
        if bounds
            .iter()
            .any(|&(value, limit)| !(1..=limit).contains(&value) || usize::try_from(value).is_err())
            || l.memory_pages == 0
            || l.memory_pages > 256
            || l.table_elements != 0
            || !(1..=MAX_CONTINUATIONS).contains(&l.continuation_count)
            || l.segment_fuel > l.total_fuel
            || l.ir_bytes > l.allocation_bytes
            || l.stack_bytes > l.allocation_bytes
            || l.hostcall_bytes > u64::from(l.memory_pages) * 65536
            || !l.stack_bytes.is_multiple_of(4)
        {
            return Err("invalid independent resource envelope".into());
        }
        Ok(())
    }

    pub fn content_ref(&self) -> Result<String, String> {
        let l = &self.limits;
        let bounds = [
            l.allocation_bytes,
            l.ir_bytes,
            l.stack_bytes,
            u64::from(l.memory_pages),
            u64::from(l.table_elements),
            l.segment_fuel,
            l.total_fuel,
            l.hostcall_bytes,
            l.input_bytes,
            l.output_bytes,
            u64::from(l.continuation_count),
            l.continuation_bytes,
            l.retention_turns,
        ];
        let value = crate::codec::record("molten-spacewasm-profile-v1", vec![
            crate::codec::string(&self.name),
            crate::codec::string(&self.source_revision),
            crate::codec::string(&self.abi),
            crate::codec::sequence(bounds.into_iter().map(crate::codec::u64_value).collect()),
            crate::codec::bool_value(self.production),
            crate::codec::bool_value(self.release),
            crate::codec::sequence(self.non_claims.iter().map(crate::codec::string).collect()),
        ]);
        crate::codec::canonical_hash(&value).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    // r[verify molten.spacewasm_core.resources]
    #[test]
    fn checked_profile_limits_reject_out_of_envelope_requests() {
        for (field, maximum) in [
            ("input_bytes", 16_777_216_u64),
            ("output_bytes", 16_777_216),
            ("continuation_count", 65_536),
            ("total_fuel", 9_007_199_254_740_991),
            ("continuation_bytes", 9_007_199_254_740_991),
            ("retention_turns", 9_007_199_254_740_991),
        ] {
            let mut checked = serde_json::to_value(super::Profile::default()).expect("valid profile JSON");
            checked["limits"][field] = serde_json::json!(maximum.min(usize::MAX as u64));
            let admissible: super::Profile = serde_json::from_value(checked.clone()).expect("bounded profile");
            admissible.validate().unwrap_or_else(|error| panic!("{field} at the bound denied: {error}"));
            checked["limits"][field] = serde_json::json!(maximum + 1);
            let excessive: super::Profile = serde_json::from_value(checked).expect("oversized numeric field");
            assert!(excessive.validate().is_err(), "{field} above the bound accepted");
        }
    }
}
