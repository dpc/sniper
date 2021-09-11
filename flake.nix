{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";

    mozillapkgs = {
      url = "github:mozilla/nixpkgs-mozilla";
      flake = false;
    };

    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    # borked: https://github.com/nix-community/fenix/issues/20
    # fenix = {
    #   url = "github:nix-community/fenix";
    #   inputs.nixpkgs.follows = "nixpkgs";
    # };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, naersk, nixpkgs, flake-utils, flake-compat, mozillapkgs }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages."${system}";

      # Get a specific rust version
      mozilla = pkgs.callPackage (mozillapkgs + "/package-set.nix") {};
      channel = (mozilla.rustChannelOf {
        # date = "2020-01-01"; # get the current date with `date -I`
        # channel = "stable";
        # sha256 = "2NfCJiH3wk7sR1XlRf8+IZfY3S9sYKdL8TpMqk82Bq0=";
        # channel = "beta";
        # sha256 = "sha256-x7ljos+NgzB7+JU1OS/tFm2Ft6QigHOmhJ8fg9jcZyQ=";
        channel = "nightly";
        sha256 = "AgR5wPY/EbZd+bMo/Yx5/wJZm5j8egSRTWlviTyJLQo=";
      });
      rust = channel.rust;

      naersk-lib = naersk.lib."${system}".override {
        cargo = rust;
        rustc = rust;
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
              rust-analyzer
              rust
            ]);
          RUST_SRC_PATH = "${channel.rust-src}/lib/rustlib/src/rust/library";
        };
  });
}
