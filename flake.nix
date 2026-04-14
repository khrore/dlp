{
  inputs = {
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      crane,
      fenix,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ fenix.overlays.default ];
        };
        lib = pkgs.lib;
        projectRoot = ./.;
        src = lib.cleanSource projectRoot;

        rustToolchain = pkgs.fenix.combine [
          (pkgs.fenix.complete.withComponents [
            "cargo"
            "clippy"
            "rust-src"
            "rustc"
            "rustfmt"
          ])
          pkgs.fenix.targets.wasm32-unknown-unknown.latest.rust-std
        ];

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        wasmBindgenCli = pkgs.wasm-bindgen-cli_0_2_114;

        cargoToml = lib.fileset.fileFilter (file: file.name == "Cargo.toml") projectRoot;
        cargoLock = lib.fileset.fileFilter (file: file.name == "Cargo.lock") projectRoot;
        cargoConfig = lib.fileset.fileFilter (file: file.name == "config.toml") projectRoot;
        rustSources = lib.fileset.fileFilter (
          file:
          let
            ext = lib.toLower (builtins.baseNameOf file.name);
          in
          lib.hasSuffix ".rs" ext || lib.hasSuffix ".toml" ext
        ) projectRoot;
        uiAssets = lib.fileset.fileFilter (
          file:
          let
            lowerName = lib.toLower file.name;
          in
          lowerName == "index.html"
          || lib.hasSuffix ".css" lowerName
          || lib.hasSuffix ".scss" lowerName
          || lib.hasSuffix ".sass" lowerName
          || lib.hasSuffix ".gif" lowerName
          || lib.hasSuffix ".ico" lowerName
          || lib.hasSuffix ".jpeg" lowerName
          || lib.hasSuffix ".jpg" lowerName
          || lib.hasSuffix ".json" lowerName
          || lib.hasSuffix ".png" lowerName
          || lib.hasSuffix ".svg" lowerName
          || lib.hasSuffix ".txt" lowerName
          || lib.hasSuffix ".webp" lowerName
          || lib.hasSuffix ".woff" lowerName
          || lib.hasSuffix ".woff2" lowerName
        ) projectRoot;

        uiAssetDirs = lib.fileset.unions [
          (lib.fileset.maybeMissing ./crates/ui/assets)
          (lib.fileset.maybeMissing ./crates/ui/public)
          (lib.fileset.maybeMissing ./crates/ui/static)
        ];
        appConfigCrate = lib.fileset.unions [
          ./crates/app-config/Cargo.toml
          ./crates/app-config/src
        ];

        workspaceSrc = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            cargoToml
            cargoLock
            cargoConfig
            rustSources
            appConfigCrate
          ];
        };

        uiSrc = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            cargoToml
            cargoLock
            cargoConfig
            rustSources
            appConfigCrate
            uiAssets
            uiAssetDirs
          ];
        };

        commonArgs = {
          inherit src;
          pname = "dlp-workspace";
          strictDeps = true;
          version = "0.1.0";
        };

        nativeRuntimeInputs = [
          rustToolchain
          pkgs.stdenv.cc
        ] ++ lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];

        darwinNativeEnv = lib.optionalString pkgs.stdenv.isDarwin ''
          export MACOSX_DEPLOYMENT_TARGET=14.0
          export LIBRARY_PATH="${pkgs.libiconv}/lib''${LIBRARY_PATH:+:$LIBRARY_PATH}"
        '';

        nativeCommonArgs = commonArgs // {
          src = workspaceSrc;
          cargoExtraArgs = "--workspace --exclude ui";
        };

        nativeCargoArtifacts = craneLib.buildDepsOnly nativeCommonArgs;

        dlp = craneLib.buildPackage (
          nativeCommonArgs
          // {
            cargoArtifacts = nativeCargoArtifacts;
            cargoExtraArgs = "--package dlp";
            pname = "dlp";
          }
        );

        controlPlane = craneLib.buildPackage (
          nativeCommonArgs
          // {
            cargoArtifacts = nativeCargoArtifacts;
            cargoExtraArgs = "--package control-plane";
            pname = "control-plane";
          }
        );

        uiCommonArgs = commonArgs // {
          src = uiSrc;
          pname = "ui";
          cargoExtraArgs = "--package ui";
          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
        };

        uiCargoArtifacts = craneLib.buildDepsOnly (
          uiCommonArgs
          // {
            doCheck = false;
          }
        );

        ui = craneLib.buildTrunkPackage (
          uiCommonArgs
          // {
            "wasm-bindgen-cli" = wasmBindgenCli;
            cargoArtifacts = uiCargoArtifacts;
            trunkIndexPath = "index.html";
            preBuild = ''
              cd crates/ui
            '';
            postBuild = ''
              mv dist ../..
              cd ../..
            '';
          }
        );

        uiDev = pkgs.writeShellApplication {
          name = "ui-dev";
          runtimeInputs = [
            rustToolchain
            pkgs.trunk
            wasmBindgenCli
          ];
          text = ''
            if [ -d "$PWD/crates/ui" ]; then
              cd "$PWD/crates/ui"
            else
              echo "ui-dev must be run from the repository root" >&2
              exit 1
            fi
            unset NO_COLOR
            exec trunk serve index.html "$@"
          '';
        };

        fmt = pkgs.writeShellApplication {
          name = "fmt";
          runtimeInputs = nativeRuntimeInputs;
          text = ''
            ${darwinNativeEnv}
            if [ ! -f "$PWD/Cargo.toml" ]; then
              echo "fmt must be run from the repository root" >&2
              exit 1
            fi
            exec cargo fmt --all --check "$@"
          '';
        };

        clippy = pkgs.writeShellApplication {
          name = "clippy";
          runtimeInputs = nativeRuntimeInputs;
          text = ''
            ${darwinNativeEnv}
            if [ ! -f "$PWD/Cargo.toml" ]; then
              echo "clippy must be run from the repository root" >&2
              exit 1
            fi
            exec cargo clippy --workspace --all-targets "$@"
          '';
        };

        test = pkgs.writeShellApplication {
          name = "test";
          runtimeInputs = nativeRuntimeInputs;
          text = ''
            ${darwinNativeEnv}
            if [ ! -f "$PWD/Cargo.toml" ]; then
              echo "test must be run from the repository root" >&2
              exit 1
            fi
            exec cargo test --workspace "$@"
          '';
        };

        build = pkgs.writeShellApplication {
          name = "build";
          runtimeInputs = nativeRuntimeInputs;
          text = ''
            ${darwinNativeEnv}
            if [ ! -f "$PWD/Cargo.toml" ]; then
              echo "build must be run from the repository root" >&2
              exit 1
            fi

            cargo build --workspace --exclude ui "$@"
            cargo build -p ui --target wasm32-unknown-unknown "$@"
          '';
        };

        check = pkgs.writeShellApplication {
          name = "check";
          runtimeInputs = [ pkgs.nix ];
          text = ''
            if [ ! -f "$PWD/flake.nix" ]; then
              echo "check must be run from the repository root" >&2
              exit 1
            fi
            exec nix flake check "$PWD" "$@"
          '';
        };
      in
      {
        apps = {
          build = flake-utils.lib.mkApp {
            drv = build;
          };
          check = flake-utils.lib.mkApp {
            drv = check;
          };
          clippy = flake-utils.lib.mkApp {
            drv = clippy;
          };
          dlp = flake-utils.lib.mkApp {
            drv = dlp;
          };
          control-plane = flake-utils.lib.mkApp {
            drv = controlPlane;
          };
          fmt = flake-utils.lib.mkApp {
            drv = fmt;
          };
          test = flake-utils.lib.mkApp {
            drv = test;
          };
          ui-dev = flake-utils.lib.mkApp {
            drv = uiDev;
          };
          default = self.apps.${system}.dlp;
        };

        packages = {
          inherit dlp ui;
          control-plane = controlPlane;
          default = dlp;
        };

        checks = {
          inherit dlp ui;
          control-plane = controlPlane;
          cargo-fmt = craneLib.cargoFmt {
            inherit src;
            pname = "dlp-workspace";
            version = "0.1.0";
          };
          cargo-clippy = craneLib.cargoClippy (
            nativeCommonArgs
            // {
              cargoArtifacts = nativeCargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            }
          );
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = [
            rustToolchain
            pkgs.rust-analyzer-nightly
            pkgs.trunk
            wasmBindgenCli
          ];
        };
      }
    );
}
