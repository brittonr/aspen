/// The first `bash` on `PATH`, for tests that run a shell through the live bounded-process adapter.
/// That adapter starts a child through its measured handle path, which becomes the child's
/// `argv[0]`, so a multicall `/bin/sh` that selects its applet from that name (BusyBox, as the Nix
/// build sandbox mounts it) cannot stand in for a shell. stdenv puts `bash` on `PATH` in the build
/// sandbox, and the development shell provides it as well.
pub(crate) fn non_multicall_shell() -> std::path::PathBuf {
    let search_path = std::env::var_os("PATH").expect("PATH names the test shell");
    std::env::split_paths(&search_path)
        .map(|directory| directory.join("bash"))
        .find(|candidate| candidate.is_absolute() && candidate.is_file())
        .expect("an absolute bash on PATH")
}
