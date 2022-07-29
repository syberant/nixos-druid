with builtins;

let
  # Load flake
  osConf = (getFlake "/etc/nixos").nixosConfigurations;
  lib = osConf.nixos-desktop.pkgs.lib;
  options = osConf.nixos-desktop.options;
  config = lib.recursiveUpdate osConf.nixos-desktop.config {
    assertions = null;
    home-manager = null;
    nixpkgs.pkgs = null;
    system.build.manual = null;

    # Trying to do all services runs up to my memory limit and gets `nix-instantiate` killed, turn it off for now
    services = null;
    # services.etebase-server = null;
    # services.hercules-ci-agent = null;
  };

  inherit (import ./utilities.nix { inherit lib; }) catchErrors;
in catchErrors config
