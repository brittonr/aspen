// Enum codecs for the canonical compute-capacity records.
fn enum_value(value: &Field, name: &str) -> Result<String> {
    text(value, name)
}

fn health(value: &Field) -> Result<molten_core::compute_capacity::Health> {
    match enum_value(value, "health")?.as_str() {
        "healthy" => Ok(molten_core::compute_capacity::Health::Healthy),
        "degraded" => Ok(molten_core::compute_capacity::Health::Degraded),
        "unavailable" => Ok(molten_core::compute_capacity::Health::Unavailable),
        "unknown" => Ok(molten_core::compute_capacity::Health::Unknown),
        _ => Err(invalid("unknown compute health")),
    }
}

fn health_name(value: molten_core::compute_capacity::Health) -> &'static str {
    match value {
        molten_core::compute_capacity::Health::Healthy => "healthy",
        molten_core::compute_capacity::Health::Degraded => "degraded",
        molten_core::compute_capacity::Health::Unavailable => "unavailable",
        molten_core::compute_capacity::Health::Unknown => "unknown",
    }
}

fn granularity(value: &Field) -> Result<molten_core::compute_capacity::Granularity> {
    match enum_value(value, "granularity")?.as_str() {
        "chip" => Ok(molten_core::compute_capacity::Granularity::Chip),
        "mmio-bundle" => Ok(molten_core::compute_capacity::Granularity::MmioBundle),
        "tray" => Ok(molten_core::compute_capacity::Granularity::Tray),
        "mesh" => Ok(molten_core::compute_capacity::Granularity::Mesh),
        "partition" => Ok(molten_core::compute_capacity::Granularity::Partition),
        "alias" => Ok(molten_core::compute_capacity::Granularity::Alias),
        _ => Err(invalid("unknown compute granularity")),
    }
}

fn granularity_name(value: molten_core::compute_capacity::Granularity) -> &'static str {
    match value {
        molten_core::compute_capacity::Granularity::Chip => "chip",
        molten_core::compute_capacity::Granularity::MmioBundle => "mmio-bundle",
        molten_core::compute_capacity::Granularity::Tray => "tray",
        molten_core::compute_capacity::Granularity::Mesh => "mesh",
        molten_core::compute_capacity::Granularity::Partition => "partition",
        molten_core::compute_capacity::Granularity::Alias => "alias",
    }
}

fn pool_kind(value: &Field) -> Result<molten_core::compute_capacity::PoolKind> {
    match enum_value(value, "pool kind")?.as_str() {
        "dedicated-memory-bytes" => Ok(molten_core::compute_capacity::PoolKind::DedicatedMemoryBytes),
        "unified-memory-bytes" => Ok(molten_core::compute_capacity::PoolKind::UnifiedMemoryBytes),
        "chips" => Ok(molten_core::compute_capacity::PoolKind::Chips),
        _ => Err(invalid("unknown compute pool kind")),
    }
}

fn pool_kind_name(value: molten_core::compute_capacity::PoolKind) -> &'static str {
    match value {
        molten_core::compute_capacity::PoolKind::DedicatedMemoryBytes => "dedicated-memory-bytes",
        molten_core::compute_capacity::PoolKind::UnifiedMemoryBytes => "unified-memory-bytes",
        molten_core::compute_capacity::PoolKind::Chips => "chips",
    }
}

fn label_authority(value: &Field) -> Result<molten_core::fabric_membership::LabelAuthority> {
    match enum_value(value, "label authority")?.as_str() {
        "advisory" => Ok(molten_core::fabric_membership::LabelAuthority::Advisory),
        "measured" => Ok(molten_core::fabric_membership::LabelAuthority::Measured),
        "operator-declared" => Ok(molten_core::fabric_membership::LabelAuthority::OperatorDeclared),
        "authoritative" => Ok(molten_core::fabric_membership::LabelAuthority::Authoritative),
        _ => Err(invalid("unknown label authority")),
    }
}

fn allocation_kind(value: &Field) -> Result<molten_core::compute_capacity::AllocationKind> {
    match enum_value(value, "allocation kind")?.as_str() {
        "execution" => Ok(molten_core::compute_capacity::AllocationKind::Execution),
        "serving-deployment" => Ok(molten_core::compute_capacity::AllocationKind::ServingDeployment),
        _ => Err(invalid("unknown allocation kind")),
    }
}

fn allocation_kind_name(value: molten_core::compute_capacity::AllocationKind) -> &'static str {
    match value {
        molten_core::compute_capacity::AllocationKind::Execution => "execution",
        molten_core::compute_capacity::AllocationKind::ServingDeployment => "serving-deployment",
    }
}

fn status(value: &Field) -> Result<molten_core::compute_capacity::ReservationStatus> {
    match enum_value(value, "allocation status")?.as_str() {
        "intent" => Ok(molten_core::compute_capacity::ReservationStatus::Intent),
        "active" => Ok(molten_core::compute_capacity::ReservationStatus::Active),
        "quarantined" => Ok(molten_core::compute_capacity::ReservationStatus::Quarantined),
        "released" => Ok(molten_core::compute_capacity::ReservationStatus::Released),
        _ => Err(invalid("unknown allocation status")),
    }
}

fn status_name(value: molten_core::compute_capacity::ReservationStatus) -> &'static str {
    match value {
        molten_core::compute_capacity::ReservationStatus::Intent => "intent",
        molten_core::compute_capacity::ReservationStatus::Active => "active",
        molten_core::compute_capacity::ReservationStatus::Quarantined => "quarantined",
        molten_core::compute_capacity::ReservationStatus::Released => "released",
    }
}

fn enforcement(value: &Field) -> Result<molten_core::compute_capacity::EnforcementStrength> {
    match enum_value(value, "enforcement")?.as_str() {
        "logical-fence-only" => Ok(molten_core::compute_capacity::EnforcementStrength::LogicalFenceOnly),
        "backend-acknowledged" => Ok(molten_core::compute_capacity::EnforcementStrength::BackendAcknowledged),
        "backend-fenced" => Ok(molten_core::compute_capacity::EnforcementStrength::BackendFenced),
        _ => Err(invalid("unknown enforcement strength")),
    }
}

fn enforcement_name(value: molten_core::compute_capacity::EnforcementStrength) -> &'static str {
    match value {
        molten_core::compute_capacity::EnforcementStrength::LogicalFenceOnly => "logical-fence-only",
        molten_core::compute_capacity::EnforcementStrength::BackendAcknowledged => "backend-acknowledged",
        molten_core::compute_capacity::EnforcementStrength::BackendFenced => "backend-fenced",
    }
}
