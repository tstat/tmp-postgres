{
  description = "Easily start a temporary PostgreSQL server for testing or exploration";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
  };

  outputs = { self, flake-utils, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ my-overlay ];
        };
        my-overlay = final: prev: { };
        package-name = "tmp-postgres";

      in
      {
        inherit pkgs;

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ cargo rust-analyzer rustfmt cargo-watch rustPackages.clippy ];
          inputsFrom = [ self.packages."${system}"."${package-name}" ];
        };
        packages = {
          default = self.packages."${system}"."${package-name}";
          # Read the docs at https://nixos.org/manual/nixpkgs/stable/#rust
          "${package-name}" = pkgs.rustPlatform.buildRustPackage {
            pname = package-name;
            version = "0.1.0";
            cargoHash = "sha256-L8cCKVhrXbazm4NvGKWU+i571lk8zA29bvaNdxU0BDM=";
            src = ./src;
            buildInputs =
              let
                darwin-frameworks = [ ];
                system-dependent = if pkgs.stdenv.isDarwin then darwin-frameworks else [ ];
              in
              system-dependent;
          };
        };
      });
}
