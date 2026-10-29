//! Record acceptance certainty before Session converts port failures to Denial.

use crate::system_extension::NativeCallbackValuePort;

pub(super) struct RecordingValuePort<V> {
    inner: V,
    recording: super::Recording,
    request_ref: Option<String>,
}

impl<V> RecordingValuePort<V> {
    pub(super) fn new(inner: V, recording: super::Recording) -> Self {
        Self {
            inner,
            recording,
            request_ref: None,
        }
    }

    pub(super) fn begin_boundary(&mut self, bytes: &[u8]) {
        self.request_ref = Some(crate::preserves_rail::content_ref_from_bytes(bytes));
    }
}

impl<V: NativeCallbackValuePort> NativeCallbackValuePort for RecordingValuePort<V> {
    fn materialize(
        &mut self,
        reference: &str,
        maximum: u64,
    ) -> Result<crate::system_extension::NativeCallbackValue, crate::system_extension::ValuePortFailure> {
        self.inner.materialize(reference, maximum)
    }

    fn publish(
        &mut self,
        value: &crate::system_extension::NativeCallbackValue,
        maximum: u64,
    ) -> Result<crate::system_extension::ValuePublicationReceipt, crate::system_extension::ValuePortFailure> {
        // Reserve the attempt before dispatch, but never hold the recording lock
        // across a product-owned port call (which may itself inspect readback).
        let index = {
            let mut records = self
                .recording
                .records
                .lock()
                .map_err(|_| rejected("component host publication recording lock unavailable"))?;
            if records.publications.len() >= self.recording.maximum_publications {
                return Err(rejected("component host publication recording capacity exhausted"));
            }
            let request_ref = self
                .request_ref
                .as_ref()
                .ok_or_else(|| rejected("component host publication has no admitted boundary"))?;
            let index = records.publications.len();
            records.publications.push(super::Publication {
                request_ref: request_ref.clone(),
                value_ref: value.value_ref.clone(),
                byte_count: value.bytes.len() as u64,
                receipt: None,
                failure: None,
            });
            index
        };
        let published = self.inner.publish(value, maximum);
        // Even a poisoned readback must retain the original post-acceptance
        // result. snapshot() still reports the poison and fences the consumer.
        let mut records = self.recording.records.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let attempt = &mut records.publications[index];
        match published {
            Err(failure) => {
                attempt.failure = Some(failure.clone());
                Err(failure)
            }
            Ok(receipt) => {
                if receipt.value_ref != value.value_ref
                    || receipt.byte_count != value.bytes.len() as u64
                    || crate::preserves_rail::validate_content_ref(&receipt.publication_ref).is_err()
                {
                    let failure = crate::system_extension::ValuePortFailure::new(
                        crate::system_extension::ValuePortFailureKind::UnknownAfterAcceptance,
                        "component host publication returned an invalid or mismatched accepted receipt",
                    );
                    attempt.receipt = Some(receipt);
                    attempt.failure = Some(failure.clone());
                    return Err(failure);
                }
                attempt.receipt = Some(receipt.clone());
                Ok(receipt)
            }
        }
    }
}

fn rejected(message: &str) -> crate::system_extension::ValuePortFailure {
    crate::system_extension::ValuePortFailure::new(
        crate::system_extension::ValuePortFailureKind::RejectedBeforeAcceptance,
        message,
    )
}

#[cfg(test)]
mod tests;
