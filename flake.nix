{
  description = ''
    WIP GUI for NixOS

    View documentation (like `man configuration.nix`) and your config (like `nixos-option` or a REPL).
  '';

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        nixos-druid = pkgs.rustPlatform.buildRustPackage rec {
          pname = "nixos-druid";
          version = "0.1.0";

          src = ./.;
          cargoSha256 = "sha256-tfJnq5jE+QlnthOwVz7munIsIoq8Z+qQrs3AJeqltCs=";

          nativeBuildInputs = with pkgs; [ pkgconfig ];
          buildInputs = with pkgs; [ gtk3 ];
          cargoBuildFlags = [ "--bin" "nixos-option-browser" "--bin" "nixos-config-browser" ];

          # Tests fail at the moment because they test src/bin/node.rs alone as well
          doCheck = false;

          meta = with pkgs.lib; {
            description = "WIP GUI for NixOS";
            homepage = "https://github.com/syberant/nixos-druid";
            # TODO: Pick license
            # license = licenses.;
            maintainers = with maintainers; [ syberant ];
          };
        };
      in rec {
        packages.nixos-druid = nixos-druid;

        apps.nixos-option-browser = flake-utils.lib.mkApp {
          drv = nixos-druid;
          exePath = "/bin/nixos-option-browser";
        };
        apps.nixos-config-browser = flake-utils.lib.mkApp {
          drv = nixos-druid;
          exePath = "/bin/nixos-config-browser";
        };

        packages.default = nixos-druid;
        apps.default = apps.nixos-option-browser;
      });
}
