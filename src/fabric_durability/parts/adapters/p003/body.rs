
fn decode_level(value: u8) -> crate::error::Result<DurabilityLevel> {
    match value {
        LEVEL_BUFFERED => Ok(DurabilityLevel::Buffered),
        LEVEL_PROCESS_LOSS => Ok(DurabilityLevel::ProcessLoss),
        LEVEL_MACHINE_LOSS => Ok(DurabilityLevel::MachineLoss),
        _ => Err(crate::error::MoltenError::invalid_harness(format!("unknown durability level code {value}"))),
    }
}

fn encode_snapshot_kind(kind: SnapshotKind) -> u8 {
    match kind {
        SnapshotKind::Snapshot => SNAPSHOT_KIND_SNAPSHOT,
        SnapshotKind::Checkpoint => SNAPSHOT_KIND_CHECKPOINT,
    }
}

fn decode_snapshot_kind(value: u8) -> crate::error::Result<SnapshotKind> {
    match value {
        SNAPSHOT_KIND_SNAPSHOT => Ok(SnapshotKind::Snapshot),
        SNAPSHOT_KIND_CHECKPOINT => Ok(SnapshotKind::Checkpoint),
        _ => Err(crate::error::MoltenError::invalid_harness(format!("unknown snapshot kind code {value}"))),
    }
}

fn encode_phase(phase: EffectTransactionPhase) -> u8 {
    match phase {
        EffectTransactionPhase::Reserved => PHASE_RESERVED,
        EffectTransactionPhase::Committed => PHASE_COMMITTED,
        EffectTransactionPhase::Aborted => PHASE_ABORTED,
        EffectTransactionPhase::Expired => PHASE_EXPIRED,
        EffectTransactionPhase::Uncertain => PHASE_UNCERTAIN,
        EffectTransactionPhase::ReconciledCommitted => PHASE_RECONCILED_COMMITTED,
        EffectTransactionPhase::ReconciledAborted => PHASE_RECONCILED_ABORTED,
    }
}

fn decode_phase(value: u8) -> crate::error::Result<EffectTransactionPhase> {
    match value {
        PHASE_RESERVED => Ok(EffectTransactionPhase::Reserved),
        PHASE_COMMITTED => Ok(EffectTransactionPhase::Committed),
        PHASE_ABORTED => Ok(EffectTransactionPhase::Aborted),
        PHASE_EXPIRED => Ok(EffectTransactionPhase::Expired),
        PHASE_UNCERTAIN => Ok(EffectTransactionPhase::Uncertain),
        PHASE_RECONCILED_COMMITTED => Ok(EffectTransactionPhase::ReconciledCommitted),
        PHASE_RECONCILED_ABORTED => Ok(EffectTransactionPhase::ReconciledAborted),
        _ => Err(crate::error::MoltenError::invalid_harness(format!("unknown effect phase code {value}"))),
    }
}

fn push_byte(bytes: &mut impl crate::bounded::VecSink<u8>, value: u8) {
    bytes.push_item(value);
}

fn push_u64(bytes: &mut impl crate::bounded::VecSink<u8>, value: u64) {
    bytes.extend_cloned_items(&value.to_be_bytes());
}

fn push_optional_u64(bytes: &mut impl crate::bounded::VecSink<u8>, value: Option<u64>) {
    push_byte(bytes, u8::from(value.is_some()));
    if let Some(value) = value {
        push_u64(bytes, value);
    }
}

fn push_blob(bytes: &mut impl crate::bounded::VecSink<u8>, value: &[u8]) -> crate::error::Result<()> {
    push_u64(bytes, byte_count(value.len())?);
    bytes.extend_cloned_items(value);
    Ok(())
}

struct ByteCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ByteCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take_byte(&mut self) -> crate::error::Result<u8> {
        let value = self
            .bytes
            .get(self.offset)
            .copied()
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("truncated durable adapter record"))?;
        self.offset = self
            .offset
            .checked_add(1)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("durable adapter cursor overflow"))?;
        Ok(value)
    }

    fn take_u64(&mut self) -> crate::error::Result<u64> {
        let end = self
            .offset
            .checked_add(LENGTH_PREFIX_BYTES)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("durable adapter cursor overflow"))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("truncated durable adapter integer"))?;
        let array: [u8; LENGTH_PREFIX_BYTES] = bytes
            .try_into()
            .map_err(|_| crate::error::MoltenError::invalid_harness("invalid durable adapter integer width"))?;
        self.offset = end;
        Ok(u64::from_be_bytes(array))
    }

    fn take_blob(&mut self) -> crate::error::Result<&'a [u8]> {
        let length = usize::try_from(self.take_u64()?)
            .map_err(|_| crate::error::MoltenError::invalid_harness("durable adapter blob length overflow"))?;
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("durable adapter cursor overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("truncated durable adapter blob"))?;
        self.offset = end;
        Ok(value)
    }

    fn take_string(&mut self) -> crate::error::Result<String> {
        String::from_utf8(self.take_blob()?.to_vec()).map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("durable adapter string is not UTF-8: {error}"))
        })
    }

    fn take_optional_u64(&mut self) -> crate::error::Result<Option<u64>> {
        if decode_bool(self.take_byte()?)? {
            self.take_u64().map(Some)
        } else {
            Ok(None)
        }
    }

    fn finish(&self) -> crate::error::Result<()> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(crate::error::MoltenError::invalid_harness("durable adapter record contains trailing bytes"))
        }
    }
}

fn decode_bool(value: u8) -> crate::error::Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(crate::error::MoltenError::invalid_harness(format!("invalid durable adapter boolean {value}"))),
    }
}

fn snapshot_file_stem(snapshot_ref: &str) -> crate::error::Result<&str> {
    snapshot_ref
        .strip_prefix("blake3:")
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("snapshot ref must be a BLAKE3 content ref"))
}

fn blake3_ref(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn byte_count(value: usize) -> crate::error::Result<u64> {
    u64::try_from(value).map_err(|_| crate::error::MoltenError::invalid_harness("durable adapter byte count overflow"))
}

fn checked_adapter_add(left: u64, right: u64) -> crate::error::Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("durable adapter byte accounting overflow"))
}

fn adapter_error(error: impl std::fmt::Display) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("durable adapter error: {error}"))
}

fn adapter_validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("durable adapter {label} denied: {issues:?}"))
}
