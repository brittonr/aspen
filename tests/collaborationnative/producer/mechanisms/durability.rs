//! Explicit mechanism levels required by buffered Raft append plus machine-loss flush.

pub(in crate::producer) fn buffered(
    namespace: &str,
) -> super::super::Result<molten::live_reliability::service::profiles::Experimental> {
    let mut mechanisms = super::experimental(namespace)?;
    mechanisms.durable.supported_levels = vec!["buffered".into(), "machine-loss".into()];
    mechanisms.durable.profile_ref = super::definition_ref(&mechanisms.durable)?;
    mechanisms.namespace.profile_ref = mechanisms.durable.profile_ref.clone();
    mechanisms.namespace.atomicity_domain.supported_levels = mechanisms.durable.supported_levels.clone();
    let profile = mechanisms.durable.clone().into_profile()?;
    mechanisms.namespace.clone().into_descriptor(&profile)?;
    Ok(mechanisms)
}
