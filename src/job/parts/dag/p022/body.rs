
fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

fn reject_mobile_closure_config(config: &IoValue) -> Result<()> {
    if let Some(marker) = crate::preserves_rail::find_ambient_job_token(config)? {
        Err(MoltenError::invalid_harness(format!(
            "job stage config contains mobile/ambient token {}",
            marker.token
        )))
    } else {
        Ok(())
    }
}

fn local_ref(kind: &str, label: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("job-dag-local-ref", vec![
        crate::preserves_rail::string(kind),
        crate::preserves_rail::string(label),
    ]))
}

fn sorted_unique(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<OrderedSet<_>>().into_iter().collect()
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/job/parts/dag/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/job/parts/dag/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/job/parts/dag/tests/m000/p002/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/job/parts/dag/tests/m000/p003/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/job/parts/dag/tests/m000/p004/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/job/parts/dag/tests/m000/p005/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/job/parts/dag/tests/m000/p006/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/job/parts/dag/tests/m000/p007/body.rs"));
}
