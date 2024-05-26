{ pkgs ? import <nixpkgs> {
    overlays = [
      (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  }
}:
let
  rust = pkgs.rust-bin.nightly.latest.default.override { extensions = [ "rust-src" ]; };
  nativeBuildInputs = with pkgs; [
    rust
    clang
    mold
    pkgconf
    cmake
  ];
  buildInputs = with pkgs;[
    openssl
    opusTools
    libopus
    yt-dlp
  ];
in
pkgs.mkShell {
  nativeBuildInputs = nativeBuildInputs;
  buildInputs = buildInputs;

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  LD_LIBRARY_PATH = "$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath buildInputs}";
}
