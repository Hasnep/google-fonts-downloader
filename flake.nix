{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.treefmt-nix.flakeModule ];
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
      perSystem =
        { self', pkgs, ... }:
        let
          name = "google-fonts-downloader";
          nativeBuildInputs = [
            pkgs.rustc
            pkgs.pkg-config
          ];
          buildInputs = [ ];
        in
        {
          packages = {
            default = self'.packages.google-fonts-downloader;
            google-fonts-downloader = pkgs.rustPlatform.buildRustPackage {
              pname = name;
              version = "1.0.0";
              src = ./.;
              cargoLock.lockFile = ./Cargo.lock;
              nativeBuildInputs = nativeBuildInputs;
              buildInputs = buildInputs;
            };
          };

          apps = {
            default = self'.apps.google-fonts-downloader;
            google-fonts-downloader = {
              type = "app";
              program = "${self'.packages.google-fonts-downloader}/bin/${name}";
            };
          };

          devShells.default = pkgs.mkShell {
            packages =
              buildInputs
              ++ nativeBuildInputs
              ++ [
                pkgs.actionlint
                pkgs.cargo
                pkgs.clippy
                pkgs.pre-commit
                pkgs.python3Packages.pre-commit-hooks
                pkgs.nodePackages.prettier
                pkgs.rustfmt
                pkgs.taplo
                pkgs.nixfmt-rfc-style
                pkgs.ratchet
              ];
            shellHook = "pre-commit install --overwrite";
          };

          treefmt.programs = {
            rustfmt.enable = true;
            nixfmt.enable = true;
            taplo.enable = true;
            prettier = {
              enable = true;
              includes = [
                "*.yaml"
                "*.yml"
                "*.md"
              ];
            };
          };
        };
    };
}
