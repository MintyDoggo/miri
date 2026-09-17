{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.fenix.follows = "fenix";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      flake-utils,
      naersk,
      nixpkgs,
      fenix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        withComponents = fenix.packages.${system}.complete.withComponents;
        baseComponents = [
          "rustc"
          "cargo"
          "clippy"
        ];
        toolchain = withComponents baseComponents;
        devToolchain = withComponents (
          baseComponents
          ++ [
            "rust-src"
            "fmt"
            "rust-analyzer"
          ]
        );

        naersk' = pkgs.callPackage naersk {
          cargo = toolchain;
          rustc = toolchain;
        };

      in
      {
        # For `nix build` & `nix run`:
        packages.default = naersk'.buildPackage {
          src = ./.;
        };

        # For `nix develop`:
        shells.default = pkgs.mkShell {
          nativeBuildInputs = [
            devToolchain
          ];
        };
      }
    );
}
