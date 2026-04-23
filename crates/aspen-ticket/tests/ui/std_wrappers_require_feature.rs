use aspen_ticket::{AspenClusterTicket, ClusterTopicId, SignedAspenClusterTicket};

fn main() {
    let secret_key = iroh_base::SecretKey::from([2u8; 32]);
    let ticket = AspenClusterTicket::new(ClusterTopicId::from_bytes([3u8; 32]), "ui-signed".to_string());
    let signed = SignedAspenClusterTicket::sign(ticket, &secret_key).unwrap();
    let _ = signed.verify();
    let _ = signed.is_expired();
}
