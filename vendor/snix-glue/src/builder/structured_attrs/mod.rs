//! Contains code modifying BuildRequest to honor structuredAttrs functionality.
// NOTE: This is all quite annoying to debug, due to some of the following reasons:
//  - structured attrs only works with a real bash.
//    as `NIX_ATTRS_SH_FILE` uses `declare`, which is not present in the busybox sandbox shell.
//  - We must source the bash script, as neither `$PATH` nor `$out` will be present in the env.
// If you want to inspect file contents and/or env vars set, the following
// nix repl prompt might be a good starting point (invoked with `-f path/to/nixpkgs`):
// ```
// :b builtins.derivation { name = "script.sh"; system = builtins.currentSystem; PATH = lib.makeBinPath [pkgs.coreutils]; FOO = "bar"; builder="${bash}/bin/bash";
//    args = ["-xc" "source \${NIX_ATTRS_SH_FILE:-/dev/null}; cat \${NIX_ATTRS_JSON_FILE:-/dev/null}; out=\${out:-\${outputs[out]}}; cat \${NIX_ATTRS_SH_FILE:-/dev/null} >\$out; exit 0"];
//    __structuredAttrs = true;}
// ```
// NOTE: when inspecting the created env, the _ one comes from bash itself!
// > When  bash invokes an external command, the variable _ is set to the full
// > pathname of the command and passed to that command in its environment.

use std::collections::BTreeMap;

use bytes::Bytes;
use nix_compat::store_path::StorePath;
mod shell;

pub const JSON_KEY: &str = "__json";

pub const SH_FILE_ENV_NAME: &str = "NIX_ATTRS_SH_FILE";
pub const SH_FILE_PATH: &str = "/build/.attrs.sh";
pub const JSON_FILE_ENV_NAME: &str = "NIX_ATTRS_JSON_FILE";
pub const JSON_FILE_PATH: &str = "/build/.attrs.json";

// FUTUREWORK: on other architectures (MacOS) these paths might not be const…

/// handle structuredAttrs.
/// This takes the contents of the __json env in the Derivation,
/// and populates `environment_vars` and `additional_files`.
pub fn handle_structured_attrs<'a>(
    json_str: &[u8],
    outputs: impl Iterator<Item = (&'a str, StorePath<&'a str>)>,
    environment_vars: &mut BTreeMap<String, Vec<u8>>,
    additional_files: &mut BTreeMap<String, Bytes>,
) -> std::io::Result<()> {
    let mut map = match serde_json::from_slice::<serde_json::Value>(json_str) {
        Ok(serde_json::Value::Object(map)) => map,
        Ok(_) => panic!("Snix bug: produced non-object at __json key in Derivation"),
        Err(e) => return Err(e.into()),
    };

    // Nix adds outputs[output_name] = output_path to the JSON and shell script inside the build (but not in the ATerm)
    map.insert(
        "outputs".to_string(),
        serde_json::Value::Object(serde_json::Map::from_iter(outputs.map(
            |(output_name, output_path)| {
                (
                    output_name.to_string(),
                    serde_json::Value::String(output_path.to_absolute_path()),
                )
            },
        ))),
    );

    environment_vars.insert(JSON_FILE_ENV_NAME.to_owned(), JSON_FILE_PATH.into());
    additional_files.insert(
        JSON_FILE_PATH.to_string(),
        serde_json::to_string(&map)
            .expect("Snix bug: unable to serialize json")
            .into(),
    );

    environment_vars.insert(SH_FILE_ENV_NAME.to_owned(), SH_FILE_PATH.into());
    additional_files.insert(SH_FILE_PATH.to_string(), {
        let mut out = String::new();
        shell::write_attrs_sh_file(&mut out, map).unwrap();
        Bytes::from(out)
    });

    Ok(())
}
