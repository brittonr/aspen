let
  pkgs = import /nix/store/0iprnlsgyf6kv98rvfjpxr60xqn42hsw-source { system = "x86_64-linux"; };
  validator = builtins.path {
    path = /dev/shm/aspen-open-issues-20261113/tools/sample-pgo/experiment.py;
    name = "aspen-owned-profile-validator.py";
  };
  bundle = builtins.storePath /nix/store/xj41rssnj874jgsig6zcd7fgx76yx9wv-0zk04glcl69ig83lva1d1gh5ms3cfjd1-profile-bundle;
  baseline = builtins.storePath /nix/store/pjyj70p6szibqg35y876cpg3bqgdh5vf-rust_molten-0.1.0;
  converter = builtins.storePath /nix/store/qpcj5lmxsrai5fyrh8w8br21msaxkij8-molten-sample-pgo-llvm-profgen-21cf28432798952d942bacc6bcee3a328faa3638;
in pkgs.runCommand "aspen-owned-profile-validation-control" {
  nativeBuildInputs = [ pkgs.python3 pkgs.time pkgs.binutils pkgs.linuxPackages.perf ];
} ''
  python3 ${validator} validate-profile \
    --bundle ${bundle} \
    --binary ${baseline}/bin/molten \
    --converter ${converter}/bin/llvm-profgen \
    --minimum-samples 100 --out "$TMPDIR/validation" || {
      printf '\nRegenerated association:\n'
      cat "$TMPDIR/validation/association.stdout"
      printf '\nOriginal association:\n'
      cat ${bundle}/association.txt
      exit 1
    }
  mkdir "$out"
  cp -R "$TMPDIR/validation" "$out/validation"
  stat -c '%u %n' ${bundle}/perf.data "$out/validation/perf.data" > "$out/ownership.txt"
  sha256sum ${bundle}/perf.data "$out/validation/perf.data" ${bundle}/sample.prof "$out/validation/regenerated.prof" > "$out/hashes.txt"
  cp ${bundle}/binding.json "$out/binding.json"
  cmp ${bundle}/sample.prof "$out/validation/regenerated.prof"
  test "$(stat -c %u ${bundle}/perf.data)" != "$(id -u)"
  test "$(stat -c %u "$out/validation/perf.data")" = "$(id -u)"
''
