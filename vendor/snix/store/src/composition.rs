//! This module provides a registry knowing about {Blob,Directory,PathInfo}
//! Services, as well as the [add_default_services] helper to seed new
//! registries with everything known here.
//! The composition machinery itself is defined in
//! [snix_castore::composition], which works generically with different kinds
//! of services.

use std::sync::LazyLock;

use snix_castore::composition::Registry;

/// The provided registry of snix_store, which has all the builtin
/// snix_castore (BlobStore/DirectoryStore) and snix_store
/// (PathInfoService) implementations.
pub static REG: LazyLock<&'static Registry> = LazyLock::new(|| {
    let mut reg = Default::default();
    add_default_services(&mut reg);
    // explicitly leak to get an &'static, so that we gain `&Registry: Send` from `Registry: Sync`
    Box::leak(Box::new(reg))
});

/// Register the builtin services of snix_castore (blob services and directory
/// services), as well as the ones from snix_store (PathInfo service) with the
/// given registry.
/// This can be used outside to create your own registry with the builtin types
/// _and_ extra third party types.
pub fn add_default_services(reg: &mut Registry) {
    snix_castore::composition::add_default_services(reg);
    crate::pathinfoservice::register_pathinfo_services(reg);
}
