use aspen_core::ContentDiscovery;

fn main() {
    let _ = core::mem::size_of::<Option<ContentDiscoveryPlaceholder>>();
}

type ContentDiscoveryPlaceholder = fn() -> &'static dyn ContentDiscovery;
