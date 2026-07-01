pub(super) fn parse(command: super::Command) -> molten::error::Result<()> {
    let super::Command::Parse {
        markdown,
        out,
        dependency_refs,
        dependency_closure_hash,
        handler_profile_ref,
        policy_refs,
        capability_refs,
        revocation_refs,
        seed_ref,
        expected_refs,
    } = command
    else {
        return dispatch_mismatch("parse");
    };
    let source = std::fs::read_to_string(&markdown).map_err(molten::error::MoltenError::from)?;
    let transcript = molten::transcripts::parse_markdown(&source, &molten::transcripts::TranscriptParseInput {
        dependency_refs,
        dependency_closure_hash,
        handler_profile_ref,
        policy_refs,
        capability_refs,
        revocation_refs,
        seed_ref,
        expected_refs,
    })?;
    super::io::write_file(&out, &molten::preserves_rail::to_text(&transcript.value)?)?;
    println!(
        "transcript parse ok transcript={} stanzas={} out={}",
        transcript.transcript_ref,
        transcript.stanzas.len(),
        out.display()
    );
    Ok(())
}

pub(super) fn run(command: super::Command) -> molten::error::Result<()> {
    let super::Command::Run {
        transcript,
        cache,
        state,
        save_root,
        out,
        receipt_out,
        failure_out,
    } = command
    else {
        return dispatch_mismatch("run");
    };
    let artifact = match super::io::read_transcript_input(&transcript) {
        Ok(artifact) => artifact,
        Err(error) => {
            super::io::write_optional_failure(failure_out.as_ref(), "parse", &error, None)?;
            return Err(error);
        }
    };
    let mode = molten::transcripts::TranscriptRunMode::parse(&state)?;
    let run = molten::transcripts::run_transcript(&artifact, &molten::transcripts::TranscriptRunInput {
        mode,
        cache_root: cache,
        save_root,
    })?;
    if let Some(path) = out.as_ref() {
        super::io::write_file(path, &molten::transcripts::render_transcript(&artifact, Some(&run))?)?;
    }
    super::io::emit_named_receipt(receipt_out.as_ref(), "transcript run receipt", &run.receipt_value)?;
    eprintln!(
        "transcript run decision={} transcript={} receipt={}",
        run.decision, run.transcript_ref, run.receipt_ref
    );
    if run.decision == "deny" || run.decision == "error" {
        let error = molten::error::MoltenError::invalid_harness(format!("transcript run decision {}", run.decision));
        super::io::write_optional_failure(failure_out.as_ref(), "run", &error, Some(vec![run.receipt_value]))?;
        return Err(error);
    }
    Ok(())
}

pub(super) fn show(command: super::Command) -> molten::error::Result<()> {
    let super::Command::Show { transcript } = command else {
        return dispatch_mismatch("show");
    };
    let artifact = super::io::read_transcript_input(&transcript)?;
    println!("{}", molten::preserves_rail::to_text(&artifact.value)?);
    Ok(())
}

pub(super) fn render(command: super::Command) -> molten::error::Result<()> {
    let super::Command::Render {
        transcript,
        receipt,
        out,
    } = command
    else {
        return dispatch_mismatch("render");
    };
    let artifact = super::io::read_transcript_input(&transcript)?;
    let run = receipt
        .as_ref()
        .map(|path| {
            let receipt_value = super::io::read_preserves_file(path)?;
            let receipt = molten::transcripts::parse_transcript_run_receipt(&receipt_value)?;
            Ok::<molten::transcripts::TranscriptRun, molten::error::MoltenError>(molten::transcripts::TranscriptRun {
                transcript_ref: receipt.transcript_ref,
                decision: receipt.decision,
                stanza_outcomes: Vec::new(),
                receipt_ref: receipt.receipt_ref,
                receipt_value,
                cache_receipt_value: None,
                state_root: None,
            })
        })
        .transpose()?;
    super::io::write_file(&out, &molten::transcripts::render_transcript(&artifact, run.as_ref())?)?;
    println!("transcript render ok transcript={} out={}", artifact.transcript_ref, out.display());
    Ok(())
}

fn dispatch_mismatch(command: &str) -> molten::error::Result<()> {
    Err(molten::error::MoltenError::invalid_harness(format!("transcript {command} dispatch mismatch")))
}
