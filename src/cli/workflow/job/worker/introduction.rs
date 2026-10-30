type Outcome<T> = molten::error::Result<T>;

pub(super) fn refuse_sender<T>(out: &std::path::Path, receipt: &preserves::IOValue) -> Outcome<T> {
    write_receipt(out, "sender-denial.preserves", receipt)?;
    Err(molten::error::Failure::invalid_harness("job worker sender introduction denied"))
}

pub(super) fn receive(
    out: &std::path::Path,
    source: &crate::cli_remote::session::Source,
    envelope: &molten::remote_dataspace::Envelope,
) -> Outcome<()> {
    match source.receive(envelope)? {
        molten::remote_dataspace::RecordedSessionOutcome::Committed(commit) => {
            write_receipt(out, "receiver-observation.preserves", &commit.receipt_value)
        }
        molten::remote_dataspace::RecordedSessionOutcome::Denied { receipt_value, .. } => {
            write_receipt(out, "receiver-denial.preserves", &receipt_value)?;
            Err(molten::error::Failure::invalid_harness("job worker receiver introduction denied"))
        }
    }
}

fn write_receipt(out: &std::path::Path, leaf: &str, value: &preserves::IOValue) -> Outcome<()> {
    std::fs::create_dir_all(out).map_err(molten::error::Failure::from)?;
    super::super::io::write_file(&out.join(leaf), &molten::preserves_rail::to_text(value)?)
}
