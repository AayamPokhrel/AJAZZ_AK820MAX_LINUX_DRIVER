{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.hardware.ajazz-ak820-max;
in {
  options.hardware.ajazz-ak820-max = {
    enable = mkEnableOption "Ajazz AK820 Max native Linux userspace driver and RGB music visualizer";

    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ./package.nix { };
      defaultText = literalExpression "pkgs.ajazz-ak820-max";
      description = "The ajazz-ak820-max package to use.";
    };

    daemon = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable the background user daemon (ak820d) for automatic music sync and effect management.";
      };
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    # Install udev rules so users in the 'input' group or active seat can control RGB
    services.udev.packages = [ cfg.package ];

    systemd.user.services.ak820d = mkIf cfg.daemon.enable {
      description = "Ajazz AK820 Max Userspace RGB Daemon & Music Visualizer";
      documentation = [ "https://github.com/ajazz-linux/ajazz-ak820-max" ];
      after = [ "pipewire.service" "pulseaudio.service" ];
      partOf = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" "default.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/ak820d";
        Restart = "on-failure";
        RestartSec = "3s";
      };
    };
  };
}
