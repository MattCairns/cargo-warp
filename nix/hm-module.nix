self: {
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib) mkEnableOption mkOption mkIf types;

  cfg = config.programs.cargo-warp;

  hostSettingsModule = types.submodule {
    options = {
      cross = mkOption {
        type = types.nullOr types.bool;
        default = null;
        description = "Whether to use cross instead of cargo for building.";
      };

      release = mkOption {
        type = types.nullOr types.bool;
        default = null;
        description = "Whether to build in release mode.";
      };

      package = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Package name to build in a workspace.";
      };

      target = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Rust target triple (e.g. aarch64-unknown-linux-gnu).";
      };
    };
  };

  # Remove null values from an attrset so they don't appear in the TOML.
  filterNulls = attrs:
    lib.filterAttrs (_: v: v != null) attrs;

  # Build the TOML config string from the settings attrset.
  generateConfig = settings: let
    defaultsSection =
      if (filterNulls settings.defaults) != {}
      then formatSection "defaults" (filterNulls settings.defaults)
      else "";

    hostsSections = lib.concatStringsSep "\n" (
      lib.mapAttrsToList (
        name: hostSettings: let
          filtered = filterNulls hostSettings;
        in
          if filtered != {}
          then formatSection "hosts.${formatKey name}" filtered
          else ""
      )
      settings.hosts
    );
  in
    lib.concatStringsSep "\n" (lib.filter (s: s != "") [defaultsSection hostsSections]);

  formatKey = key:
    if lib.hasInfix " " key || lib.hasInfix "*" key || lib.hasInfix "?" key || lib.hasInfix "." key
    then "\"${key}\""
    else key;

  formatSection = name: attrs: let
    lines = lib.mapAttrsToList (k: v: "${k} = ${formatValue v}") attrs;
  in
    "[${name}]\n" + lib.concatStringsSep "\n" lines + "\n";

  formatValue = v:
    if builtins.isBool v
    then
      (
        if v
        then "true"
        else "false"
      )
    else if builtins.isString v
    then "\"${v}\""
    else builtins.toString v;

  hasAnySettings =
    (filterNulls cfg.settings.defaults)
    != {}
    || cfg.settings.hosts != {};
in {
  options.programs.cargo-warp = {
    enable = mkEnableOption "cargo-warp, a cargo subcommand to build and transfer binaries to remote hosts";

    package = lib.mkPackageOption self.packages.${pkgs.stdenv.hostPlatform.system} "cargo-warp" {
      default = "default";
    };

    settings = {
      defaults = mkOption {
        type = hostSettingsModule;
        default = {};
        description = ''
          Default build settings applied to all destinations.
          These can be overridden by host-specific settings or CLI flags.
        '';
      };

      hosts = mkOption {
        type = types.attrsOf hostSettingsModule;
        default = {};
        example = lib.literalExpression ''
          {
            "mypc" = {
              cross = true;
              target = "aarch64-unknown-linux-gnu";
            };
            "*.lab.local" = {
              release = true;
            };
          }
        '';
        description = ''
          Per-host build settings. Keys can be exact hostnames, SSH aliases,
          or wildcard patterns (using * and ?).

          Settings are resolved with the following precedence:
          CLI flags > host-specific config > defaults.
        '';
      };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [cfg.package];

    xdg.configFile."cargo-warp/config.toml" = mkIf hasAnySettings {
      text = generateConfig cfg.settings;
    };
  };
}
