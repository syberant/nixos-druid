# Evaluate this file with:
# nix-instantiate --eval extract.nix --strict --json

# Idea: Generate JSON schema for a webapp?
# https://github.com/json-editor/json-editor

with builtins;

let
  inherit (import <nixpkgs> { }) lib;
  options = (import <nixpkgs/nixos> { configuration = { }; }).options;
  inherit (import ./utilities.nix { inherit lib; }) catchJson;
in with lib;

let
  fixTypes = antiInfiniteRecursion: type:
    let
      fixTypes' = fixTypes ([ type.description ] ++ antiInfiniteRecursion);
      recurseAttrs' = recurseAttrs antiInfiniteRecursion;

      nestedTypes = mapAttrs (name: fixTypes') type.nestedTypes;
      isInfinite = nested:
        any (x: x ? "_infiniteRecursion") (attrValues nested);
      mkType = {
        _type = true;
        inherit (type) name description;
        functorName = type.functor.name;
      };
    in if count (x: x == type.description) antiInfiniteRecursion >= 3 then
    # TODO: Handle infinite recursion types properly
    # Currently counting how many times this description was seen before
    # This method is prone to false positives
      mkType // {
        _infiniteRecursion = true;
      }

      # Use this line to take a look at these infinite recursion types
      # trace type.description (mkType // { _infiniteRecursion = true; })
    else if type.name == "submodule" then {
      _submodule = true;
      options = recurseAttrs' (type.getSubOptions [ ]);
    } else if elem type.description [
      "JSON value"
      "Yaml value"
      "YAML value"
      "TOML value"
    ] then
    # TODO: Special processing for these common infinite recursion types?
      mkType // {
        _infiniteRecursion = true;
        _standard = true;
      }
    else if isInfinite nestedTypes then
      mkType // { _infiniteRecursion = true; }
    else if type.name == "enum" then
      mkType // {
        # Extract the 'payload' of the enum
        # i.e. the allowed values
        functorPayload = type.functor.payload;
        inherit nestedTypes;
      }
    else
      mkType // { inherit nestedTypes; };

  fixAttrs = antiInfiniteRecursion: name: opt:
    if isOption opt then {
      _option = true;
      description = opt.description or "";
      example = opt.example or null;
      default = opt.defaultText or (catchJson (opt.default or null));
      type = fixTypes antiInfiniteRecursion opt.type;
    } else
      recurseAttrs antiInfiniteRecursion opt;

  removeInternal = filterAttrs (name: value: name != "_module");
  recurseAttrs = antiInfiniteRecursion: opt:
    mapAttrs (fixAttrs antiInfiniteRecursion) (removeInternal opt);

  getOptionsInfo = recurseAttrs [ ];
in getOptionsInfo options
