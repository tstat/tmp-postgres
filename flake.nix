{
  description = "basic rust project";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
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
            cargoSha256 = "sha256-o9pxSXyiDS7FFdZmXssY1qQvyya0tTg1nRpMPXd9ZO4=";
            src = ./src;
            buildInputs =
              let
                darwin-frameworks = with pkgs.darwin.apple_sdk.frameworks;
                  [
                    CoreFoundation
                    CoreServices
                    SystemConfiguration
                    pkgs.libiconv
                  ];
              in
              if system == "aarch64-darwin" then darwin-frameworks else [ ];
          };
        };
      });
}
