{
  description = "cargo-warp: build and copy your project binary to a remote host";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs @ {flake-parts, ...}: let
    self = flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "x86_64-linux"
        "x86_64-darwin"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      perSystem = {
        config,
        pkgs,
        system,
        ...
      }: let
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [(import inputs.rust-overlay)];
        };

        makeRustInfo = {
          version,
          profile,
        }: let
          rust =
            if builtins.elem version ["stable" "beta" "nightly"]
            then pkgs.rust-bin.${version}.latest.${profile}.override {extensions = ["rust-src"];}
            else pkgs.rust-bin.stable.${version}.${profile}.override {extensions = ["rust-src"];};
        in {
          name = "rust-" + version + "-" + profile;

          # From https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/11
          path = "${rust}/lib/rustlib/src/rust/library";

          drvs = [
            pkgs.just
            pkgs.openssl
            pkgs.pkg-config
            pkgs.rust-analyzer
            rust
          ];
        };

        makeRustEnv = {
          version,
          profile,
        }: let
          rustInfo = makeRustInfo {
            inherit version profile;
          };
        in
          pkgs.buildEnv {
            name = rustInfo.name;
            paths = rustInfo.drvs;
          };

        matrix = {
          stable-default = {
            version = "1.80.0";
            profile = "default";
          };

          stable-minimal = {
            version = "1.80.0";
            profile = "minimal";
          };

          beta-default = {
            version = "beta";
            profile = "default";
          };

          beta-minimal = {
            version = "beta";
            profile = "minimal";
          };

          nightly-default = {
            version = "nightly";
            profile = "default";
          };

          nightly-minimal = {
            version = "nightly";
            profile = "minimal";
          };
        };
      in {
        formatter = pkgs.alejandra;

        packages = {
          cargo-warp = pkgs.callPackage ./nix/package.nix {};
          default = config.packages.cargo-warp;
        };

        devShells =
          builtins.mapAttrs
          (
            name: value: let
              version = value.version;
              profile = value.profile;
              rustInfo = makeRustInfo {
                inherit version profile;
              };
            in
              pkgs.mkShell {
                name = rustInfo.name;

                RUST_SRC_PATH = rustInfo.path;

                buildInputs = rustInfo.drvs;
              }
          )
          matrix
          // {
            default = let
              version = matrix.stable-default.version;
              profile = matrix.stable-default.profile;
              rustInfo = makeRustInfo {
                inherit version profile;
              };
            in
              pkgs.mkShell {
                name = rustInfo.name;

                RUST_SRC_PATH = rustInfo.path;

                buildInputs = rustInfo.drvs;
              };
          };
      };
    };
  in
    self
    // {
      homeManagerModules.default = import ./nix/hm-module.nix self;
      homeManagerModules.cargo-warp = self.homeManagerModules.default;
    };
}
