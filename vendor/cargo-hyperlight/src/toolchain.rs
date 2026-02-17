use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{Context, Result};
use regex::Regex;

use crate::cargo_cmd::{CargoCmd, cargo_cmd};
use crate::cli::Args;

#[derive(serde::Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoMetadataPackage>,
}

#[derive(serde::Deserialize)]
struct CargoMetadataPackage {
    name: String,
    manifest_path: PathBuf,
    #[allow(dead_code)]
    // we can use this if we ever change the include paths to be copied
    version: semver::Version,
}

pub fn prepare(args: &Args) -> Result<()> {
    let metadata = cargo_cmd()?
        .env_clear()
        .envs(args.env.iter())
        .current_dir(&args.current_dir)
        .arg("metadata")
        .manifest_path(&args.manifest_path)
        .arg("--format-version=1")
        .append_rustflags("--cfg=hyperlight")
        .append_rustflags("--check-cfg=cfg(hyperlight)")
        .checked_output()
        .context("Failed to get cargo metadata")?;

    let metadata = serde_json::from_slice::<CargoMetadata>(&metadata.stdout)
        .context("Failed to parse cargo metadata")?;

    let hyperlight_guest_bin = metadata
        .packages
        .into_iter()
        .find(|pkg| pkg.name == "hyperlight-guest-bin")
        .context("Could not find hyperlight-guest-bin package in cargo metadata")?;

    let hyperlight_guest_bin_dir = hyperlight_guest_bin
        .manifest_path
        .parent()
        .context("Failed to get directory for hyperlight-guest-bin")?;

    let include_dst_dir = args.includes_dir();

    std::fs::create_dir_all(&include_dst_dir)
        .context("Failed to create sysroot include directory")?;

    const INCLUDE_DIRS: &[&str] = &[
        "third_party/printf/",
        "third_party/musl/include",
        "third_party/musl/arch/generic",
        "third_party/musl/arch/x86_64",
        "third_party/musl/src/internal",
    ];

    for dir in INCLUDE_DIRS {
        let include_src_dir = hyperlight_guest_bin_dir.join(dir);
        let files = glob::glob(&format!("{}/**/*.h", include_src_dir.display()))
            .context("Failed to read include source directory")?;

        for file in files {
            let src = file.context("Failed to read include source file")?;
            let dst = src.strip_prefix(&include_src_dir).unwrap();
            let dst = include_dst_dir.join(dst);

            std::fs::create_dir_all(dst.parent().unwrap())
                .context("Failed to create include subdirectory")?;
            std::fs::copy(&src, &dst).context("Failed to copy include file")?;
        }
    }

    Ok(())
}

pub fn cflags(args: &Args) -> OsString {
    const FLAGS: &[&str] = &[
        // terrible hack, see
        // https://github.com/hyperlight-dev/hyperlight/blob/main/src/hyperlight_guest_bin/build.rs#L80
        "--target=x86_64-unknown-linux-none",
        "-U__linux__",
        // Our rust target also has this set since it based off "x86_64-unknown-none"
        "-fPIC",
        // We don't support stack protectors at the moment, but Arch Linux clang
        // auto-enables them for -linux platforms, so explicitly disable them.
        "-fno-stack-protector",
        "-fstack-clash-protection",
        "-mstack-probe-size=4096",
        "-mno-red-zone",
        "-nostdinc",
        // Define HYPERLIGHT as we use this to conditionally enable/disable code
        // in the libc headers
        "-DHYPERLIGHT=1",
        "-D__HYPERLIGHT__=1",
    ];

    let mut flags = OsString::new();
    for flag in FLAGS {
        flags.push(flag);
        flags.push(" ");
    }
    flags.push(" ");
    flags.push("-isystem");
    flags.push(" ");
    flags.push(args.includes_dir().as_os_str());
    flags
}

pub fn find_cc() -> Result<PathBuf> {
    if let Ok(path) = which::which("clang") {
        return Ok(path);
    }
    // try with postfixed version clang, e.g., clang-20
    let re = Regex::new(r"clang-\d+").unwrap();
    which::which_re(&re)
        .context("Could not find 'clang' in PATH")?
        .next()
        .context("Could not find 'clang' in PATH")
}

pub fn find_ar() -> Result<PathBuf> {
    if let Ok(path) = which::which("ar") {
        return Ok(path);
    }
    if let Ok(path) = which::which("llvm-ar") {
        return Ok(path);
    }
    // try with postfixed version llvm-ar, e.g., llvm-ar-20
    let re = Regex::new(r"llvm-ar-\d+").unwrap();
    which::which_re(&re)
        .context("Could not find 'ar' or 'llvm-ar' in PATH")?
        .next()
        .context("Could not find 'ar' or 'llvm-ar' in PATH")
}
