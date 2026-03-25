builtins.toXML {
  drv = derivation {
    name = "test";
    builder = "/bin/sh";
    system = "x86_64-linux";
  };
}
