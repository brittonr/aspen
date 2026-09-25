
fn claim_manifest(
    dest_root: &CapabilityChunkRoot,
    ticket: &str,
    expected_manifest_ref: Option<&str>,
) -> Result<String> {
    let advertised_manifest_ref = match ticket.strip_prefix("iroh-local-chunk:") {
        Some(manifest_ref) => manifest_ref,
        None => {
            let receipt_value = denial_receipt_value(
                "iroh-fetch",
                None,
                &[],
                "unsupported Iroh chunk ticket; expected iroh-local-chunk:<manifest-ref>",
                vec![("ticket-shape", "fail"), ("deny-unsupported-ticket", "pass")],
            );
            store_receipt(dest_root, &receipt_value)?;
            return Err(MoltenError::invalid_harness(
                "unsupported Iroh chunk ticket; expected iroh-local-chunk:<manifest-ref>",
            ));
        }
    };
    if let Some(expected) = expected_manifest_ref
        && expected != advertised_manifest_ref
    {
        let message = format!("Iroh chunk ticket advertises manifest {advertised_manifest_ref}, expected {expected}");
        let receipt_value = denial_receipt_value("iroh-fetch", Some(advertised_manifest_ref), &[], &message, vec![
            ("ticket-manifest-binding", "fail"),
            ("deny-wrong-manifest", "pass"),
        ]);
        store_receipt(dest_root, &receipt_value)?;
        return Err(MoltenError::invalid_harness(message));
    }
    Ok(advertised_manifest_ref.to_string())
}
