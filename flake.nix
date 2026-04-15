{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix.url = "github:nix-community/fenix";
  };

  outputs = {
    self,
    flake-utils,
    naersk,
    nixpkgs,
    fenix,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = (import nixpkgs) {
          inherit system;
          overlays = [fenix.overlays.default];
        };

        naersk' = pkgs.callPackage naersk {};
      in {
        defaultPackage = naersk'.buildPackage {
          src = ./.;

          nativeBuildInputs = [ pkgs.installShellFiles ];

          postInstall = /*sh*/ ''
            installShellCompletion --cmd timetrack \
              --fish ${./assets/fish_completions.fish}
          '';
        };

        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            alejandra
            rust-analyzer
            (pkgs.fenix.stable.withComponents [
              "cargo"
              "clippy"
              "rust-src"
              "rustc"
              "rustfmt"
            ])
          ];
        };
      }
    );
}
