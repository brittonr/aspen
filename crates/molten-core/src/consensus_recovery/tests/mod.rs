use std::collections::BTreeSet;

use super::*;

mod progress;
mod repair;

const INDEX: u64 = 7;
const SELF: &str = "node-a";

fn entry(term: u64, label: &str) -> EntryIdentity {
    EntryIdentity {
        term,
        entry_ref: format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex()),
    }
}

fn voters() -> BTreeSet<String> {
    ["node-a", "node-b", "node-c"].into_iter().map(ToString::to_string).collect()
}

fn report(peer: &str, held: Option<EntryIdentity>, commit_index: u64) -> PeerEntryReport {
    PeerEntryReport {
        peer_id: peer.to_string(),
        entry: held,
        commit_index,
    }
}

fn input(local: LocalEntry, local_commit_index: u64, peer_reports: Vec<PeerEntryReport>) -> EntryRepairInput {
    EntryRepairInput {
        index: INDEX,
        local,
        local_commit_index,
        voters: voters(),
        participant_id: SELF.to_string(),
        peer_reports,
    }
}
