// AST-only deny plants. Not a Cargo target: names and argument lists are intentionally unresolved.
use aya as planted_loader;

pub struct PlantedHandles {
    pub map: MapMut,
    pub link: LinkFd,
}

fn planted_calls() {
    libc::syscall(PLANTED_ONLY);
    std::fs::read_dir(PLANTED_ONLY);
    let _ = "/sys/fs/bpf";
    let _ = r"/proc/planted-only";
    caps::set(PLANTED_ONLY);
    planted_root.write(PLANTED_ONLY);
}
