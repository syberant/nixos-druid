with builtins;

let
  # Load flake
  osConf = (getFlake "/etc/nixos").nixosConfigurations;
  computer = osConf.nixos-macbook;

  # Get necessary components
  lib = computer.pkgs.lib;
  options = computer.options;
  config = lib.recursiveUpdate computer.config {
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
