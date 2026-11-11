{
  description = "Reproducible Rust build";

  inputs = {
    # Update with `nix flake update` to pin to the latest toolchain version
    # in nixpkgs-unstable (updates flake.lock)
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };
  outputs =
    { self, nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;

      # Generate outputs for systems listed in https://github.com/NixOS/nixpkgs/blob/master/lib/systems/flake-systems.nix
      eachSystem = lib.genAttrs lib.systems.flakeExposed;

      # Defines a function to build the dyndns package for the current system architecture
      mkDynDns =
        {
          rustPlatform,
          nixVersions,
          ...
        }:
        rustPlatform.buildRustPackage {
          pname = "dyndns";
          version = "0.6.0";
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = false;
          };

          nativeBuildInputs = [ (nixVersions.latest or nixVersions.unstable) ];

          meta = {
            description = "A simple DynDNS server whcih allows you to update DNS records with your dynamic IP address that is assigned by your Internet Service Provider (ISP).";
            homepage = "https://github.com/bbastin/dyndns";
            license = with lib.licenses; [
              agpl3Only
            ];
            mainProgram = "dyndns";
          };
        };
    in
    {
      formatter = eachSystem (system: nixpkgs.legacyPackages.${system}.nixfmt-tree);
      # Development shell for pinned rust toolchain
      devShells.default = eachSystem (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          rust-toolchain = pkgs.symlinkJoin {
            name = "rust-toolchain";
            paths = [
              pkgs.rustc
              pkgs.cargo
              pkgs.rustPlatform.rustcSrc
            ];
          };
        in
        pkgs.mkShell {
          buildInputs = [
            rust-toolchain
            pkgs.nixfmt
          ];
        }
      );

      # package output for direct package consumption
      # `nix build` assembles the nix package
      packages = eachSystem (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.callPackage mkDynDns { };
          dyndns = pkgs.callPackage mkDynDns { };
        }
      );

      # nixosModule to enable dyndns as a systemd service for nixos systems
      nixosModules.dyndns =
        {
          config,
          utils,
          pkgs,
          ...
        }:
        let
          cfg = config.services.dyndns;
        in
        {
          options = {
            services.dyndns = {
              enable = lib.mkEnableOption "DynDNS";

              openFirewall = lib.mkOption {
                type = lib.types.bool;
                default = false;
                description = ''
                  Open ports in the firewall for the dyndns api
                '';
              };
              settings = lib.mkOption {
                type = lib.types.submodule {
                  options = {
                    # configures ROCKET_PORT as documented in https://rocket.rs/guide/v0.5/configuration/#environment-variables
                    port = lib.mkOption {
                      type = lib.types.port;
                      description = "Port Number";
                      default = 8000;
                    };
                    # configures ROCKET_ADDRESS as documented in https://rocket.rs/guide/v0.5/configuration/
                    address = lib.mkOption {
                      type = lib.types.str;
                      default = "127.0.0.1";
                      example = "127.0.0.1";
                      description = ''
                        The IP address dyndns listens on, 127.0.0.1 by default                            
                      '';
                    };                    
                  };
                };
              };
              user = lib.mkOption {
                type = lib.types.str;
                default = "dyndns";
                description = ''
                  User account under which DynDNS runs.";

                  ::: {.note}
                  If left as the default value this user will automatically be  created on system activation, otherwise you are responsible for
                  ensuring the user exists before the DynDNS service starts.
                  :::
                '';
              };

              group = lib.mkOption {
                type = lib.types.str;
                default = "dyndns";
                description = ''
                  Group account under which DynDNS runs.

                  ::: {.note}
                  If left as the default value this group will automatically be created
                  on system activation, otherwise you are responsible for
                  ensuring the group exists before the DynDNS service starts.
                  :::
                '';
              };

              package = lib.mkPackageOption pkgs "dyndns" { };
            };
          };
          config = lib.mkIf cfg.enable {
            systemd.services.dyndns = {
              description = "DynDNS server for cloud hosted zones";
              after = [ "network.target" ];

              environment = {
                ROCKET_PORT = "${toString (cfg.settings.port)}";
                ROCKET_ADDRESS = "${toString (cfg.settings.address)}";
                DYNDNS_CONFIG_FILE_PATH = "%d/config.json";
              };
              serviceConfig = {
                Type = "simple";
                DynamicUser = true;
                ExecStart = utils.escapeSystemdExecArgs [
                  (lib.getExe cfg.package)
                ];
                # NOTE - this could be configurable to load the config.json from a user defined path
                LoadCredential="config.json:/etc/dyndns/config.json";
                BindReadOnlyPaths = [
                          builtins.storeDir                          
                          "/etc/" # dns, ssl certificates,...                                                
                ];
                
                Restart = "on-failure";
                RestartSec="5s";
                RuntimeDirectory="dyndns";
                RootDirectory="/run/dyndns";
                RootImage="";
                MountAPIVFS=true;

                # Hardening
                CapabilityBoundingSet = "";
                DeviceAllow = [];
                LockPersonality = true;
                MemoryDenyWriteExecute=true;
                NoNewPrivileges = true;
                ProtectHome = true;
                ProtectClock = true;
                ProtectKernelLogs = true;
                PrivateTmp = true;
                PrivateDevices = true;
                PrivateUsers = true;
                ProtectKernelTunables = true;
                ProtectKernelModules = true;
                ProtectControlGroups = true;
                ProtectProc="noaccess";
                ProcSubset="pid";
                RestrictSUIDSGID = true;
                RemoveIPC = true;
                ProtectHostname = true;
                ProtectSystem = "strict";
                RestrictAddressFamilies = [
                  "AF_INET"
                  "AF_INET6"
                ];
                RestrictNamespaces = true;
                RestrictRealtime = true;

                SystemCallArchitectures = "native";
                SystemCallFilter = [
                  "@system-service"
                  "~@privileged"
                  "~@resources"
                ];

                UMask="0066";
              };
            };

            networking.firewall = lib.mkIf cfg.openFirewall {
              allowedTCPPorts = [ cfg.settings.port ];
            };

            users.users = lib.mkIf (cfg.user == "dyndns") {
              dyndns = {
                group = cfg.group; 
                isSystemUser = true;
                isNormalUser = false;
              };
            };

            users.groups = lib.mkIf (cfg.group == "dyndns") {
              dyndns = { };
            };
          };
        };
    };
}
