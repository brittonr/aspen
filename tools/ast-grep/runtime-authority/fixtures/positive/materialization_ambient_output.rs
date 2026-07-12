fn ambient_member_write(output: &std::path::Path, name: &str, bytes: &[u8]) {
    let member = output.join(name);
    std::fs::create_dir_all(member.parent().expect("member parent")).expect("ambient parent");
    std::fs::write(member, bytes).expect("ambient member write");
}

fn generic_unpack<R: std::io::Read>(archive: &mut tar::Archive<R>, output: &std::path::Path) {
    archive.unpack(output).expect("generic archive unpack");
}
