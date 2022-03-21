{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  buildInputs = with pkgs;
    [
      cargo
      clangStdenv
      clippy
      libclang
      libiconv
      nixfmt
      rust-analyzer
      rustc
      rustfmt
    ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
      darwin.apple_sdk.frameworks.Security
      darwin.apple_sdk.frameworks.SystemConfiguration
    ];

  RUST_BACKTRACE = 1;
}
