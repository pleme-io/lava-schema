# nix/modules/nixos.nix — auto-generated from lava-schema.caixa.lisp
# description: "Typed schema protocol for lava architectures. GraphQL-equivalent for infrastructure: every architecture declares a strict interface (typed inputs + typed outputs) other architectures consume by typed query. Loose escape hatches where strictness doesn't apply. Authored in tatara-lisp ((deflava-interface ...)). Powers cross-architecture compile-time composition."
{ config, lib, pkgs, ... }:
let
  cfg = config.services.lava-schema;
in {
  options.services.lava-schema = {
    enable = lib.mkEnableOption "lava-schema";
    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.lava-schema or null;
    };
  };
  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
  };
}
