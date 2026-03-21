{
  pkgs ? import <nixpkgs> { },
}:
with pkgs;
let
  toolchain = (rust-bin.stable.latest.default.override {
    extensions = [ "rust-src" "rust-analyzer" ];
  });
in
mkShell {
  strictDeps = true;

  nativeBuildInputs = [
    toolchain

    taplo

    lldb
    yaml-language-server
    cargo-nextest
    just
    nix-output-monitor
  ];

  buildInputs = lib.optionals stdenv.isDarwin [
    libiconv
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  env = {
    NH_NOM = "1";
    NH_LOG = "nh=trace";
    RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
  };
}
