# Evaluate this file with:
# nix-instantiate --eval extract.nix --strict --json --argstr foo bar

# Idea: Generate JSON schema for a webapp?
# https://github.com/json-editor/json-editor

{ utilities ? import ./utilities.nix }:

with builtins;

let
  inherit (import <nixpkgs> { }) lib;
  nixosOptions = (import <nixpkgs/nixos> { configuration = { }; }).options;

  inherit (utilities { inherit lib; }) catchJson isVisibleNameValue;
in with lib;

let
  # Properly export the type, arguments are as follows:
  # - shallow: boolean, if set nested suboptions (in submodule(s)) will not be exported
  # - antiInfiniteRecursion: int, counter to prevent infinite recursion present in some types
  # - type: NixOS type, actual type to export
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

  # Export an option with the fields properly set.
  # In case this is not an option recurse into the nested options.
  fixAttrs = antiInfiniteRecursion: name: opt:
    if isOption opt then {
      _option = true;
      description = opt.description or "";
      example = opt.example or null;
      default = opt.defaultText or (catchJson (opt.default or null));
      type =
        fixTypes (if opt ? visible then opt.visible == "shallow" else false)
        antiInfiniteRecursion opt.type;
    } else
      recurseAttrs antiInfiniteRecursion opt;

  # Recursively visit all options, removes non-visible options
  recurseAttrs = antiInfiniteRecursion: opt:
    let
      visibleOptions = filterAttrs isVisibleNameValue opt;
      fixAttrs' = fixAttrs antiInfiniteRecursion;
    in mapAttrs fixAttrs' visibleOptions;

  # Start recursion with empty `antiInfiniteRecursion`
  getOptionsInfo = recurseAttrs [ ];
in getOptionsInfo nixosOptions
