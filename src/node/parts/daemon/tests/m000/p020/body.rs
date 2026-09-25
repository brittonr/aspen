
    struct DenyCase<'a> {
        name: &'a str,
        grant_peer: Option<&'a str>,
        grant_node: &'a str,
        grant_operations: &'a [&'a str],
        target_ref: Option<&'a str>,
        target_scope: &'a str,
        resource_scope: &'a str,
        epoch: u64,
        expires_at: Option<u64>,
        is_revoked: bool,
        sequence: u64,
        expected: &'a str,
    }

    struct DenyCaseRefs {
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        peer_bootstrap_refs: Vec<String>,
        authority_refs: Vec<String>,
    }
