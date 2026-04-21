use aspen_core::NetworkTransport;

fn main() {
    let _ = core::any::type_name::<&'static str>();
    let _ = core::mem::size_of::<Option<&'static str>>();
    let _unused: Option<&'static str> = None;
    let _ = &_unused;
    let _ = core::mem::size_of::<usize>();
    let _ = core::mem::size_of::<Option<NetworkTransportPlaceholder>>();
}

type NetworkTransportPlaceholder = fn() -> &'static dyn NetworkTransport<Address = (), Endpoint = (), Gossip = (), SecretKey = ()>;
