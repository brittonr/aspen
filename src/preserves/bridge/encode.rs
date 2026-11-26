//! Canonical encoding of `IOValue` by the kernel's incremental writer.

/// Drives the kernel's `CanonicalWriter` from `preserves::Writer` events: `IOValue` walks its own
/// tree by reference, so canonical encoding copies each atom payload once and builds no second
/// tree. Annotations are not written.
struct KernelWriter(molten_preserves_core::incremental::CanonicalWriter);

impl preserves::Writer for KernelWriter {
    fn boundary(&mut self, boundary: &preserves::boundary::Type) -> std::io::Result<()> {
        let is_item_closed = matches!(
            boundary.closing,
            Some(preserves::boundary::Item::SetValue | preserves::boundary::Item::DictionaryValue)
        );
        if is_item_closed {
            self.0.next_item();
        }
        Ok(())
    }

    fn should_write_annotations(&self) -> bool {
        false
    }

    fn start_annotations(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn end_annotations(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn write_bool(&mut self, value: bool) -> std::io::Result<()> {
        self.0.boolean(value);
        Ok(())
    }

    fn write_f64(&mut self, value: f64) -> std::io::Result<()> {
        self.0.double(value.to_bits());
        Ok(())
    }

    fn write_i128(&mut self, value: i128) -> std::io::Result<()> {
        self.0.signed(value);
        Ok(())
    }

    fn write_u128(&mut self, value: u128) -> std::io::Result<()> {
        self.0.unsigned(value);
        Ok(())
    }

    /// Only integers outside `i128`/`u128` reach here (the `SignedIntegerRepr::Big` invariant).
    fn write_int(&mut self, value: &num_bigint::BigInt) -> std::io::Result<()> {
        self.0.integer_bytes(&value.to_signed_bytes_be());
        Ok(())
    }

    fn write_string(&mut self, value: &str) -> std::io::Result<()> {
        self.0.string(value);
        Ok(())
    }

    fn write_bytes(&mut self, value: &[u8]) -> std::io::Result<()> {
        self.0.byte_string(value);
        Ok(())
    }

    fn write_symbol(&mut self, value: &str) -> std::io::Result<()> {
        self.0.symbol(value);
        Ok(())
    }

    fn start_record(&mut self) -> std::io::Result<()> {
        self.0.open_record();
        Ok(())
    }

    fn end_record(&mut self) -> std::io::Result<()> {
        self.0.close();
        Ok(())
    }

    fn start_sequence(&mut self) -> std::io::Result<()> {
        self.0.open_sequence();
        Ok(())
    }

    fn end_sequence(&mut self) -> std::io::Result<()> {
        self.0.close();
        Ok(())
    }

    fn start_set(&mut self) -> std::io::Result<()> {
        self.0.open_set();
        Ok(())
    }

    fn end_set(&mut self) -> std::io::Result<()> {
        self.0.close_sorted();
        Ok(())
    }

    fn start_dictionary(&mut self) -> std::io::Result<()> {
        self.0.open_dictionary();
        Ok(())
    }

    fn end_dictionary(&mut self) -> std::io::Result<()> {
        self.0.close_sorted();
        Ok(())
    }

    fn start_embedded(&mut self) -> std::io::Result<()> {
        self.0.embedded();
        Ok(())
    }

    fn end_embedded(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Canonical packed bytes of `value`, written by the kernel encoder from the value's own walk.
pub(super) fn encode_iovalue(value: &super::IoValue) -> super::Result<Vec<u8>> {
    let mut writer = KernelWriter(molten_preserves_core::incremental::CanonicalWriter::new());
    value
        .write(&mut writer, &mut preserves::IOValueDomainEncode)
        .map_err(|error| super::MoltenError::Preserves(error.to_string()))?;
    writer
        .0
        .finish()
        .ok_or_else(|| super::MoltenError::invalid_harness("canonical writer left a compound open"))
}
