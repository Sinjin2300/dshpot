{
  lib,
  config,
  ...
}:
let
  cfg = config.services.dshpot;
in
{
  options.services.dshpot = {
    enable = lib.mkEnableOption "dshpot SSH honeypot";

    package = lib.mkOption {
      type = lib.types.package;
      description = "The dshpot package to use.";
    };

    honeypotPort = lib.mkOption {
      type = lib.types.port;
      default = 2222;
      description = "Port the honeypot listens on.";
    };

    honeypotIp = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0";
      description = "IP address the honeypot binds to.";
    };

    openFirewall = lib.mkEnableOption "open the firewall for the honeypot port";

    metricsType = lib.mkOption {
      type = lib.types.enum [
        "none"
        "file"
      ];
      default = "file";
      description = "Metric backend to use. Currently only 'file' is implemented.";
    };

    logLevel = lib.mkOption {
      type = lib.types.enum [
        "trace"
        "debug"
        "info"
        "warn"
        "error"
      ];
      default = "warn";
      description = "Log level to use.";
    };

    disableLogTimestamps = lib.mkOption {
      default = true;
      example = false;
      description = "Whether to disable timestamps in logs";
      type = lib.types.bool;
    };
  };

  config = lib.mkIf cfg.enable {
    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [ cfg.honeypotPort ];

    systemd.services.dshpot = {
      description = "dshpot SSH honeypot";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      serviceConfig = {
        ExecStartPre = "${lib.getExe cfg.package} init";
        ExecStart = "${lib.getExe cfg.package} serve";
        Restart = "on-failure";
        RestartSec = "5s";

        DynamicUser = true;
        StateDirectory = "dshpot";
        RuntimeDirectory = "dshpot";

        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        CapabilityBoundingSet = "";
      };

      environment = {
        BIND_PORT = toString cfg.honeypotPort;
        BIND_IP = cfg.honeypotIp;
        DATA_DIR = "/var/lib/dshpot";
        LOG_LEVEL = cfg.logLevel;
        METRICS_EXPORTER = cfg.metricsType;
        DISABLE_LOG_TIMESTAMP = if cfg.disableLogTimestamps then "true" else "false";
      };
    };
  };
}
