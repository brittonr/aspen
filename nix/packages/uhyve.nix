{
  lib,
  rustPlatform,
  fetchFromGitHub,
}:

rustPlatform.buildRustPackage {
  pname = "uhyve";
  version = "0.8.0-unstable-2026-05-08";

  src = fetchFromGitHub {
    owner = "hermit-os";
    repo = "uhyve";
    rev = "8ca08f93a2294ce8283f0f9226ae6b21aea3d20d";
    hash = "sha256-IgkU6ENznhYJLI31eZ+S5VMmv2gkVuMAuyB9mVh4fh0=";
  };

  cargoHash = "sha256-uL90WbkW3cUyXwSbAElELKzpgIkxgdNm76dzr7twDWI=";

  # Upstream's test suite builds and runs nested Hermit kernels during checkPhase,
  # which requires network-capable cargo git setup and KVM. Aspen proves real
  # execution separately through the gated runtime-host product-path test.
  doCheck = false;

  meta = {
    description = "Specialized hypervisor for Hermit unikernels";
    homepage = "https://github.com/hermit-os/uhyve";
    license = lib.licenses.mit;
    mainProgram = "uhyve";
    platforms = ["x86_64-linux"];
  };
}
