# nix/modules/home-manager.nix — auto-generated from lava-schema.caixa.lisp
{ config, lib, pkgs, ... }:
let cfg = config.programs.lava-schema; in {
  options.programs.lava-schema = {
    enable = lib.mkEnableOption "lava-schema";
    package = lib.mkOption { type = lib.types.package; default = pkgs.lava-schema or null; };
  };
  config = lib.mkIf cfg.enable { home.packages = [ cfg.package ]; };
}
