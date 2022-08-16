with builtins;

let
  # NOTE: As part of a horrible hack ./utilities.nix is inlined at runtime so that the executable works standalone.
  # utilities = import ./utilities.nix;
  # These variables are inlined as arguments as well.
  # flakePath = "/etc/nixos";
  # hostname = "nixos-desktop";

  # Load flake
  computer = (getFlake flakePath).nixosConfigurations.${hostname};

  # Get necessary components
  lib = computer.pkgs.lib;
  options = computer.options;
  config = lib.recursiveUpdate computer.config {
    assertions = null;
    home-manager = null;
    nixpkgs.pkgs = null;
    system.build.manual = null;
    virtualisation = null;
    boot = null;

    # Trying to do all services runs up to my memory limit (more than 12GB) and gets `nix-instantiate` killed, turn it off for now
    services = null;
    # services.etebase-server = null;
    # services.hercules-ci-agent = null;
    # services.gitlab = null;
  };

  inherit (utilities { inherit lib; }) catchErrors;
in catchErrors config
