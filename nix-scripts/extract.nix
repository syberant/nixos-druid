# Evaluate this file with:
# nix-instantiate --eval extract.nix --strict --json

# Idea: Generate JSON schema for a webapp?
# https://github.com/json-editor/json-editor

# Known bugs:
# - Internal attributes within submodules are not filtered out (e.g. users.users.<name>._module.check)

{ utilities ? import ./utilities.nix }:

with builtins;

let
  inherit (import <nixpkgs> { }) lib;
  nixosOptions = (import <nixpkgs/nixos> { configuration = { }; }).options;

  inherit (utilities { inherit lib; }) catchJson;
in with lib;

let
  fixTypes = shallow: antiInfiniteRecursion: type:
    let
      fixTypes' =
        fixTypes shallow ([ type.description ] ++ antiInfiniteRecursion);
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
      options = if shallow then { } else recurseAttrs' (type.getSubOptions [ ]);
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
      type = fixTypes (if opt ? visible then
        opt.visible == "shallow"
      else
        trace opt.loc false) antiInfiniteRecursion opt.type;
    } else
      recurseAttrs antiInfiniteRecursion opt;

  fixMissingFields = let
    recurse = n: v:
      if isOption v then
        let
          opt = v // {
            internal = v.internal or false;
            # Possible values: true, false, "shallow"
            visible = v.visible or true;
            readOnly = v.readOnly or false;
            type = mapAttrs recurse v.type;
          };
        in opt
      else if isAttrs v then
        mapAttrs recurse v
      else
        v;
  in recurse "";

  # If `v` is a visible option, keep all fields and recurse into type.nestedTypes
  # If `v` is a non-visible option, delete it
  # Otherwise recurse
  removeInternal = let
    pred = name: v:
      if isOption v then
        (if isString v.visible then v.visible == "shallow" else v.visible)
        && !v.internal
      else
        true;
    recurse = n: v:
      if !isAttrs v then
        v
      else if isOption v then
        v // { type = mapAttrs recurse v.type; }
      else
        mapAttrs recurse (filterAttrs pred v);
  in recurse "";

  recurseAttrs = antiInfiniteRecursion: opt:
    mapAttrs (fixAttrs antiInfiniteRecursion) opt;

  getOptionsInfo = recurseAttrs [ ];
in pipe nixosOptions [
  fixMissingFields
  removeInternal
  getOptionsInfo
]
# in { inherit nixosOptions fixMissingFields removeInternal getOptionsInfo; }
