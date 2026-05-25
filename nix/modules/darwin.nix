# nix/modules/darwin.nix — auto-generated from lava-schema.caixa.lisp
{ config, lib, pkgs, ... }:
let cfg = config.services.lava-schema; in {
  options.services.lava-schema = {
    enable = lib.mkEnableOption "lava-schema";
    package = lib.mkOption { type = lib.types.package; default = pkgs.lava-schema or null; };
  };
  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
  };
}
