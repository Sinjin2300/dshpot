{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    {
      self,
      flake-utils,
      naersk,
      nixpkgs,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        application_name = "dshpot";
        pkgs = (import nixpkgs) {
          inherit system;
        };

        naersk' = pkgs.callPackage naersk { };

        buildInputs = with pkgs; [ ];
        nativeBuildInputs = with pkgs; [ ];

        entrypoint = pkgs.writeTextFile {
          name = "entrypoint";
          text = builtins.readFile ./entrypoint.sh;
          destination = "/entrypoint.sh";
          executable = true;
        };
      in
      rec {
        defaultPackage = packages.${application_name};
        packages = {
          ${application_name} = naersk'.buildPackage {
            src = ./.;
            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs;
          };

          container = pkgs.dockerTools.buildImage {
            name = "${application_name}";

            copyToRoot = pkgs.buildEnv {
              name = "root";
              paths = [
                pkgs.bash
                pkgs.coreutils
                packages.${application_name}
                entrypoint
              ];
            };

            config = {
              Entrypoint = [
                "${pkgs.bash}/bin/bash"
                "/entrypoint.sh"
              ];
              ExposedPorts = {
                "2222/tcp" = { };
              };
              Volumes = {
                "/data" = { };
              };
              Env = [
                "DATA_DIR=/data"
                "BIND_PORT=2222"
                "BIND_IP=0.0.0.0"
              ];
            };
          };
        };

        devShell = pkgs.mkShell {
          nativeBuildInputs =
            with pkgs;
            [
              cmake
              rustc
            ]
            ++ buildInputs
            ++ nativeBuildInputs;
        };
      }
    );
}
