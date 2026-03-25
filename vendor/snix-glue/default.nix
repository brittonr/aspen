{
  depot,
  lib,
  pkgs,
  ...
}:
(depot.snix.crates.workspaceMembers.snix-glue.build.override {
  runTests = true;
  testPreRun = ''
    export SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
  '';
})
.overrideAttrs
(old: rec {
  meta.ci.targets = lib.filter (x: lib.hasPrefix "with-features" x || x == "no-features") (
    lib.attrNames passthru
  );
  passthru =
    old.passthru
    // (depot.snix.utils.mkFeaturePowerset {
      inherit (old) crateName;
      features = ["nix_tests"];
      override.testPreRun = ''
        export SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
      '';
    });
})
