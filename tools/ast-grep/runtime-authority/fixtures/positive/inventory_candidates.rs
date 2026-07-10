#![allow(dead_code, unused_variables, unused_unsafe)]

struct AuthorityBypass;

impl AuthorityBypass {
    fn admit(scope: &str) {
        let _ = scope;
    }
}

fn inventory_candidates(path: &std::path::Path, address: &str) {
    let _ = std::fs::read_to_string(path);
    let _ = std::fs::read(path);
    let _ = std::fs::write(path, b"candidate");
    let _ = std::fs::remove_file(path);
    let _ = std::process::Command::new("molten");
    let _ = std::net::TcpListener::bind(address);
    let _ = std::net::TcpStream::connect(address);
    let _ = std::time::SystemTime::now();
    let _ = std::time::Instant::now();
    let _ = rand::thread_rng();
    let _ = std::env::var("MOLTEN_TOKEN");
    let _ = std::env::var_os("MOLTEN_TOKEN");
    let _ = unsafe { libloading::Library::new(path) };
    unsafe {
        let _ = core::ptr::read_volatile(&0);
    }
    panic!("candidate panic path");
    AuthorityBypass::admit("operator");
}
