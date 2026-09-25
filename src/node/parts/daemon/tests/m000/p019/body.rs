
    fn flow_admission(root: &Path, ticket: &ControlLiveTicket, policy_refs: &[String]) -> ControlLivePeerAdmission {
        admit_control_live_peer(&ControlLivePeerAdmitInput {
            state_root: root,
            ticket_value: &ticket.value,
            peer_id: "peer:live-bundle",
            sequence: 1,
            expires_at: Some(8),
            policy_refs,
            evidence_refs: &[],
        })
        .expect("admit peer")
    }

    fn flow_authority_value(policy_refs: &[String], operations: &[String]) -> IoValue {
        control_authority_grant_value(&ControlAuthorityGrantInput {
            peer_id: "peer:live-bundle",
            node_id: "node:live-bundle",
            operations,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: Some(8),
            policy_refs,
            revocation_refs: &[],
            evidence_refs: &[],
        })
        .expect("authority grant value")
    }
