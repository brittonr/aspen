
fn parsed_parts(artifacts: &[ChainBundleArtifact]) -> Result<Parts<'_>> {
    let parts = artifacts
        .iter()
        .map(|artifact| (artifact.artifact_ref.clone(), artifact))
        .collect::<OrderedMap<_, _>>();
    let links = artifacts
        .iter()
        .filter(|artifact| artifact.kind == "chain-link")
        .map(|artifact| {
            crate::evidence_chain::parse_chain_link(&artifact.value).map(|link| (link.link_ref.clone(), link))
        })
        .collect::<Result<OrderedMap<_, _>>>()?;
    let predicates = artifacts
        .iter()
        .filter(|artifact| artifact.kind == "chain-predicate-receipt")
        .map(|artifact| {
            crate::evidence_chain::parse_chain_predicate_receipt(&artifact.value)
                .map(|receipt| (receipt.receipt_ref.clone(), receipt))
        })
        .collect::<Result<OrderedMap<_, _>>>()?;
    let mut has_forks = false;
    for artifact in artifacts.iter().filter(|artifact| artifact.kind == "chain-fork-evidence") {
        crate::evidence_chain::parse_chain_fork_evidence(&artifact.value)?;
        has_forks = true;
    }
    Ok(Parts {
        artifacts: parts,
        links,
        predicates,
        has_forks,
    })
}

fn anchors(chain: &crate::evidence_chain::ChainScope, artifacts: &[ChainBundleArtifact]) -> Result<()> {
    for artifact in artifacts.iter().filter(|artifact| artifact.kind == "chain-anchor") {
        let anchor = crate::evidence_chain::parse_chain_anchor(&artifact.value)?;
        if anchor.chain != *chain {
            return Err(MoltenError::invalid_harness("chain bundle anchor belongs to a different chain"));
        }
    }
    Ok(())
}
