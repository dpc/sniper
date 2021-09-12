{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";

    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, naersk, nixpkgs, flake-utils, flake-compat, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages."${system}";
      fenix-rust = { inherit (fenix.packages.${system}.minimal) cargo rustc; };
      naersk-lib = naersk.lib."${system}".override {
        cargo = fenix-rust.cargo;
        rustc = fenix-rust.rustc;
      };
    in rec {
      packages.sniper = naersk-lib.buildPackage ./.;

      defaultPackage = self.packages.${system}.sniper;
      defaultApp = self.packages.${system}.sniper;

      # `nix develop`
      devShell = pkgs.mkShell
        {
          inputsFrom = builtins.attrValues self.packages.${system};
          buildInputs = [ pkgs.libsodium pkgs.lzma pkgs.openssl ];
          nativeBuildInputs = (with pkgs;
            [
              pkgconfig
              fenix-rust.rust-analyzer
              fenix-rust.rustc
            ]);
          RUST_SRC_PATH = "${fenix-rust.rust-src}/lib/rustlib/src/rust/library";
        };
  });
}
