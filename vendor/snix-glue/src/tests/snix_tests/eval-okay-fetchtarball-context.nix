(builtins.getContext (
  builtins.fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/91050ea1e57e50388fa87a3302ba12d188ef723a.tar.gz";
    sha256 = "1hf6cgaci1n186kkkjq106ryf8mmlq9vnwgfwh625wa8hfgdn4dm";
  }
))
