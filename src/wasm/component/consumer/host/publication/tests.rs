use crate::system_extension::NativeCallbackValuePort;

struct ControlledPort {
    store: std::sync::Arc<std::sync::Mutex<crate::system_extension::InMemoryCallbackValuePort>>,
    recording: super::super::Recording,
    mode: u8,
}

impl NativeCallbackValuePort for ControlledPort {
    fn materialize(
        &mut self,
        reference: &str,
        maximum: u64,
    ) -> Result<crate::system_extension::NativeCallbackValue, crate::system_extension::ValuePortFailure> {
        self.store.lock().expect("store").materialize(reference, maximum)
    }

    fn publish(
        &mut self,
        value: &crate::system_extension::NativeCallbackValue,
        maximum: u64,
    ) -> Result<crate::system_extension::ValuePublicationReceipt, crate::system_extension::ValuePortFailure> {
        let before = self.recording.snapshot().expect("port can inspect readback without a held recording lock");
        assert_eq!(
            before.publications.last().expect("attempt must precede real port dispatch").value_ref,
            value.value_ref
        );
        if self.mode == 0 {
            return Err(crate::system_extension::ValuePortFailure::new(
                crate::system_extension::ValuePortFailureKind::RejectedBeforeAcceptance,
                "rejected before storage",
            ));
        }
        let mut receipt = self.store.lock().expect("store").publish(value, maximum)?;
        match self.mode {
            1 => {
                return Err(crate::system_extension::ValuePortFailure::new(
                    crate::system_extension::ValuePortFailureKind::UnknownAfterAcceptance,
                    "completion lost after storage",
                ));
            }
            2 => receipt.value_ref = crate::preserves_rail::content_ref_from_bytes(b"wrong-value"),
            3 => receipt.byte_count += 1,
            4 => receipt.publication_ref = "invalid-publication-ref".to_string(),
            _ => {}
        }
        Ok(receipt)
    }
}

fn value() -> crate::system_extension::NativeCallbackValue {
    let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::string("live-session-value"))
        .expect("canonical bytes");
    crate::system_extension::NativeCallbackValue {
        value_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        bytes,
    }
}

#[test]
fn retains_acceptance_certainty_and_invalid_original_receipts() {
    let value = value();
    for mode in 0..5 {
        let store =
            std::sync::Arc::new(std::sync::Mutex::new(crate::system_extension::InMemoryCallbackValuePort::empty()));
        let recording = super::super::Recording::with_capacity(1, None);
        let mut port = super::RecordingValuePort::new(
            ControlledPort {
                store: store.clone(),
                recording: recording.clone(),
                mode,
            },
            recording.clone(),
        );
        port.begin_boundary(b"selected-live-request");
        let failure = port.publish(&value, 65_536).expect_err("publication failure");
        let records = recording.snapshot().expect("host recording survives failed dispatch");
        assert_eq!(records.has_uncertain_publication(), mode != 0);
        assert_eq!(records.publications[0].failure.as_ref(), Some(&failure));
        assert_eq!(
            records.publications[0].request_ref,
            crate::preserves_rail::content_ref_from_bytes(b"selected-live-request")
        );
        assert_eq!(records.publications[0].value_ref, value.value_ref);
        assert_eq!(records.publications[0].byte_count, value.bytes.len() as u64);
        assert_eq!(store.lock().expect("store").contains(&value.value_ref), mode != 0);
        let receipt = records.publications[0].receipt.as_ref();
        assert_eq!(receipt.is_some(), mode >= 2);
        match mode {
            2 => assert_ne!(receipt.expect("original invalid receipt").value_ref, value.value_ref),
            3 => assert_eq!(receipt.expect("original invalid receipt").byte_count, value.bytes.len() as u64 + 1),
            4 => assert_eq!(receipt.expect("original invalid receipt").publication_ref, "invalid-publication-ref"),
            _ => {}
        }
        assert!(records.effects.is_empty(), "uncertainty is not a fabricated completed host effect");
    }
}

#[test]
fn capacity_denies_before_an_unrecordable_effect() {
    let recording = super::super::Recording::with_capacity(1, None);
    let mut port =
        super::RecordingValuePort::new(crate::system_extension::InMemoryCallbackValuePort::empty(), recording.clone());
    port.begin_boundary(b"selected-live-request");
    for _ in 0..recording.maximum_publications {
        port.publish(&value(), 65_536).expect("reserved publication slot");
    }
    let extra = crate::system_extension::NativeCallbackValue {
        value_ref: crate::preserves_rail::content_ref_from_bytes(b"extra"),
        bytes: b"extra".to_vec(),
    };
    assert_eq!(
        port.publish(&extra, 65_536).expect_err("capacity denial").kind,
        crate::system_extension::ValuePortFailureKind::RejectedBeforeAcceptance
    );
    assert!(!port.inner.contains(&extra.value_ref), "capacity denial occurred after live publication");
    assert_eq!(recording.snapshot().expect("recording").publications.len(), recording.maximum_publications);
}
