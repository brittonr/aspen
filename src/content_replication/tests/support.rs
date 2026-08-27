use std::cell::RefCell;
use std::rc::Rc;

use molten_core::content_replication::*;

use super::super::*;
use crate::error::Result;

pub const DIGEST_HEX_LENGTH: usize = 64;
pub const GENERATION: u64 = 1;
pub const MEMBERSHIP_EPOCH: u64 = 2;
pub const PLACEMENT_EPOCH: u64 = 3;
pub const CONTENT_BYTES: u64 = 64;
pub const DEFAULT_REPLICAS: usize = 2;
pub const TRANSFER_LIMIT: usize = 4;
pub const TRANSFER_BYTE_LIMIT: u64 = 256;
pub const QUEUE_LIMIT: usize = 16;
pub const TIMER_LIMIT: usize = 4;
pub const DIAGNOSTIC_LIMIT: usize = 16;

pub type Events = Rc<RefCell<Vec<&'static str>>>;

pub fn digest(byte: char) -> String {
    format!("blake3:{}", byte.to_string().repeat(DIGEST_HEX_LENGTH))
}

pub fn manifest() -> Manifest {
    Manifest {
        service_id: "content-replication-shell-fixture".to_string(),
        generation: GENERATION,
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        authority_ref: digest('1'),
        identity_ref: digest('2'),
        content_profile_ref: digest('3'),
        transport_profile_ref: digest('4'),
        retention_policy_ref: digest('5'),
        evidence_profile_ref: digest('6'),
        ports: REQUIRED_PORTS.iter().map(ToString::to_string).collect(),
        policy: ReplicaPolicy {
            desired_replicas: DEFAULT_REPLICAS,
            minimum_verified_replicas: DEFAULT_REPLICAS,
            minimum_fault_domains: DEFAULT_REPLICAS,
        },
        repair: RepairPolicy {
            max_attempts: MAX_REPAIR_ATTEMPTS,
            allow_handoff: true,
            cleanup_after_handoff: true,
        },
        resources: ResourceLimits {
            max_concurrent_transfers: TRANSFER_LIMIT,
            max_transfer_bytes: TRANSFER_BYTE_LIMIT,
            max_queue_depth: QUEUE_LIMIT,
            max_timers: TIMER_LIMIT,
            max_diagnostics: DIAGNOSTIC_LIMIT,
        },
        contents: vec![ReplicaRule {
            content_ref: digest('7'),
            manifest_ref: digest('8'),
            encoded_bytes: CONTENT_BYTES,
            protected: true,
            transform_ref: Some(digest('9')),
            cleanup_authority_ref: Some(digest('a')),
        }],
        non_claims: NON_CLAIMS.iter().map(ToString::to_string).collect(),
    }
}

pub fn peer(id: &str, domain: &str) -> Peer {
    Peer {
        peer_id: id.to_string(),
        fault_domain: domain.to_string(),
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        available: true,
        capacity_bytes: TRANSFER_BYTE_LIMIT,
    }
}

pub fn source_replica(manifest: &Manifest) -> Replica {
    Replica {
        content_ref: manifest.contents[0].content_ref.clone(),
        peer_id: "peer-a".to_string(),
        fault_domain: "zone-a".to_string(),
        generation: GENERATION,
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        present: true,
        identity_verified: true,
        pinned: true,
        protected: true,
        manifest_ref: manifest.contents[0].manifest_ref.clone(),
        cleanup_clearance_ref: None,
    }
}

pub struct Authority {
    pub admitted: bool,
    pub events: Events,
}

impl AuthorityPort for Authority {
    fn observe(&mut self, manifest: &Manifest) -> Result<AuthorityObservation> {
        self.events.borrow_mut().push("authority");
        Ok(AuthorityObservation {
            observation_ref: digest('b'),
            authority_ref: manifest.authority_ref.clone(),
            service_id: manifest.service_id.clone(),
            generation: manifest.generation,
            admitted: self.admitted,
        })
    }
}

pub struct Identity {
    pub current: bool,
    pub events: Events,
}

impl IdentityPort for Identity {
    fn observe(&mut self, manifest: &Manifest) -> Result<IdentityObservation> {
        self.events.borrow_mut().push("identity");
        Ok(IdentityObservation {
            observation_ref: digest('c'),
            identity_ref: manifest.identity_ref.clone(),
            service_id: manifest.service_id.clone(),
            generation: manifest.generation,
            current: self.current,
        })
    }
}

pub struct Membership {
    pub current: bool,
    pub peers: Vec<Peer>,
    pub events: Events,
}

impl MembershipPort for Membership {
    fn observe(&mut self, manifest: &Manifest) -> Result<MembershipObservation> {
        self.events.borrow_mut().push("membership");
        Ok(MembershipObservation {
            observation_ref: digest('d'),
            membership_epoch: manifest.membership_epoch,
            peers: self.peers.clone(),
            current: self.current,
        })
    }
}

pub struct Placement {
    pub current: bool,
    pub placement_epoch: u64,
    pub events: Events,
}

impl PlacementPort for Placement {
    fn observe(&mut self, manifest: &Manifest) -> Result<PlacementObservation> {
        self.events.borrow_mut().push("placement");
        Ok(PlacementObservation {
            observation_ref: digest('e'),
            membership_epoch: manifest.membership_epoch,
            placement_epoch: self.placement_epoch,
            current: self.current,
        })
    }
}

pub struct Clock {
    pub events: Events,
}

impl TimePort for Clock {
    fn observe(&mut self, _manifest: &Manifest) -> Result<TimeObservation> {
        self.events.borrow_mut().push("time");
        Ok(TimeObservation {
            observation_ref: digest('f'),
            observed_tick: 1,
        })
    }
}

pub struct Resources {
    pub admitted: bool,
    pub events: Events,
}

impl ResourcePort for Resources {
    fn reserve(&mut self, plan: &Plan) -> Result<ResourceObservation> {
        self.events.borrow_mut().push("resources");
        Ok(ResourceObservation {
            reservation_ref: digest('0'),
            plan_ref: plan.plan_ref.clone(),
            generation: plan.generation,
            admitted: self.admitted,
        })
    }
}

pub struct FactPorts {
    pub authority: Authority,
    pub identity: Identity,
    pub membership: Membership,
    pub placement: Placement,
    pub clock: Clock,
    pub resources: Resources,
}

impl FactPorts {
    pub fn admitted(events: &Events) -> Self {
        Self {
            authority: Authority {
                admitted: true,
                events: events.clone(),
            },
            identity: Identity {
                current: true,
                events: events.clone(),
            },
            membership: Membership {
                current: true,
                peers: vec![peer("peer-a", "zone-a"), peer("peer-b", "zone-b")],
                events: events.clone(),
            },
            placement: Placement {
                current: true,
                placement_epoch: PLACEMENT_EPOCH,
                events: events.clone(),
            },
            clock: Clock { events: events.clone() },
            resources: Resources {
                admitted: true,
                events: events.clone(),
            },
        }
    }

    pub fn activation(&mut self) -> ActivationPorts<'_> {
        ActivationPorts {
            authority: &mut self.authority,
            identity: &mut self.identity,
            membership: &mut self.membership,
            placement: &mut self.placement,
        }
    }
}
