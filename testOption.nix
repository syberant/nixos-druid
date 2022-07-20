# Evaluate this file with:
# nix-instantiate --eval 'json/testOption.nix' --argstr location '[ "location", "latitude" ]' --argstr valueString '4.0' --strict --json

# Check the validity of a declaration
# TODO: Also check submodules
# Doable by traversing types as well like in ./extract.nix
{ location, valueString }:

let
  inherit (import <nixpkgs> {}) pkgs lib;
  inherit (import <nixpkgs/nixos> { configuration = {}; }) options;
in with lib;
   with builtins;

let
  option = getAttrFromPath (fromJSON location) options;
  value = fromJSON valueString;

  recurseAttrs = opt: mapAttrs fixAttrs opt;
in if isOption option then
      option.type.check value
   else throw "Invalid location: Not an option"
