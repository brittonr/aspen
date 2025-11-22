{
  depot,
  pkgs,
  lib,
  ...
}:

let
  mkImportCheck = p: expectedPath: {
    label = ":nix :import ${p} with snix-store import";
    needsOutput = true;
    command = pkgs.writeShellScript "snix-import-check" ''
      export BLOB_SERVICE_ADDR=memory://
      export DIRECTORY_SERVICE_ADDR=memory://
      export PATH_INFO_SERVICE_ADDR=memory://
      SNIX_STORE_OUTPUT=$(result/bin/snix-store import ${p})
      EXPECTED='${
        # the vebatim expected Snix output:
        expectedPath
      }'

      echo "snix-store output: ''${SNIX_STORE_OUTPUT}"
      if [ "$SNIX_STORE_OUTPUT" != "$EXPECTED" ]; then
        echo "Correct would have been ''${EXPECTED}"
        exit 1
      fi

      echo "Output was correct."
    '';
  };
in

(depot.snix.crates.workspaceMembers.snix-store.build.override (old: {
  runTests = true;
  testPreRun = ''
    export SSL_CERT_FILE=/dev/null
  '';
  features =
    old.features
    # virtiofs feature currently fails to build on Darwin
    ++ lib.optional pkgs.stdenv.isLinux "virtiofs";
})).overrideAttrs
  (old: rec {
    meta.ci = {
      targets = [
        "integration-tests"
      ]
      ++ lib.filter (x: lib.hasPrefix "with-features" x || x == "no-features") (lib.attrNames passthru);
      extraSteps.import-docs = (mkImportCheck "snix/docs/src/store" ../docs/src/store);
    };
    passthru =
      old.passthru
      // (depot.snix.utils.mkFeaturePowerset {
        inherit (old) crateName;
        features = (
          [
            "cloud"
            "fuse"
            "otlp"
            "tonic-reflection"
            "xp-composition-cli"
          ]
          # virtiofs feature currently fails to build on Darwin
          ++ lib.optional pkgs.stdenv.isLinux "virtiofs"
        );
        override.testPreRun = ''
          export SSL_CERT_FILE=/dev/null
        '';
      })
      // {
        integration-tests = depot.snix.crates.workspaceMembers.${old.crateName}.build.override (old: {
          runTests = true;
          testPreRun = ''
            export SSL_CERT_FILE=/dev/null
            export PATH="$PATH:${
              pkgs.lib.makeBinPath [
                pkgs.cbtemulator
                pkgs.google-cloud-bigtable-tool
              ]
            }"
          '';
          features = old.features ++ [ "integration" ];
        });
      };
  })
