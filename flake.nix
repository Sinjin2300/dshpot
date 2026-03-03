{
  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
      ];

      perSystem =
        {
          pkgs,
          lib,
          self',
          system,
          ...
        }:
        let
          application_name = "dshpot";
          craneLib = inputs.crane.mkLib pkgs;

          buildInputs = [ ];
          nativeBuildInputs = with pkgs; [
            cmake
            pkg-config
          ];

          entrypoint = pkgs.writeTextFile {
            name = "entrypoint";
            text = builtins.readFile ./entrypoint.sh;
            destination = "/entrypoint.sh";
            executable = true;
          };

          commonArgs = {
            src =
              let
                sqlFilter = path: _type: builtins.match ".*\.sql$" path != null;
                sqlOrCargo = path: type: (craneLib.filterCargoSources path type) || (sqlFilter path type);
              in
              lib.cleanSourceWith {
                src = craneLib.path ./.;
                filter = sqlOrCargo;
              };
            strictDeps = true;
            inherit buildInputs nativeBuildInputs;
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        {
          packages.${application_name} = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              meta.mainProgram = application_name;
            }
          );

          packages.default = self'.packages.${application_name};

          packages.container = pkgs.dockerTools.buildImage {
            name = application_name;
            copyToRoot = pkgs.buildEnv {
              name = "root";
              paths = [
                pkgs.bash
                pkgs.coreutils
                self'.packages.${application_name}
                entrypoint
              ];
            };
            config = {
              Entrypoint = [
                "${pkgs.bash}/bin/bash"
                "/entrypoint.sh"
              ];
              ExposedPorts."2222/tcp" = { };
              Volumes."/data" = { };
              Env = [
                "DATA_DIR=/data"
                "BIND_PORT=2222"
                "BIND_IP=0.0.0.0"
              ];
            };
          };

          checks = pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
            cargo-clippy = craneLib.cargoClippy (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

            cargo-test = craneLib.cargoTest (commonArgs // { inherit cargoArtifacts; });

            dshpot-nixos = pkgs.testers.runNixOSTest {
              name = "dshpot-nixos";
              nodes.machine =
                { ... }:
                {
                  imports = [ ./module.nix ];
                  services.dshpot = {
                    enable = true;
                    openFirewall = true;
                    package = self'.packages.${application_name};
                  };
                };
              testScript = ''
                machine.start()
                machine.wait_for_unit("dshpot.service")
                machine.wait_for_open_port(2222)
                machine.succeed("systemctl is-active dshpot.service")
                machine.succeed("test -d /var/lib/dshpot")
              '';
            };
          };

          devShells.default = pkgs.mkShell {
            inputsFrom = [ self'.packages.${application_name} ];
            nativeBuildInputs = with pkgs; [
              rustc
              cargo
              rust-analyzer
              clippy
            ];
          };
        };

      flake = {
        nixosModules.default =
          { lib, pkgs, ... }:
          {
            imports = [ ./module.nix ];
            config.services.dshpot.package =
              lib.mkDefault
                inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.dshpot;
          };
      };
    };
}
