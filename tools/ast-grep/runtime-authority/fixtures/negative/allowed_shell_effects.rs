#![allow(dead_code)]

struct ShellEffectReceipt {
    authority_ref: String,
    policy_ref: String,
    evidence_ref: String,
}

fn shell_owned_receipt(authority_ref: &str, policy_ref: &str, evidence_ref: &str) -> ShellEffectReceipt {
    ShellEffectReceipt {
        authority_ref: authority_ref.to_string(),
        policy_ref: policy_ref.to_string(),
        evidence_ref: evidence_ref.to_string(),
    }
}

fn allowed_shell_effects() -> ShellEffectReceipt {
    shell_owned_receipt("blake3:authority", "blake3:policy", "blake3:evidence")
}
