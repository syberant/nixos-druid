{ lib }:

with builtins;
with lib;

rec {
  # Extract the text from sets of the form `{ _type = "literalExpression"; text = <something>; }`
  extractLiteralExpression = expr:
    if ((expr._type or null) == "literalExpression") then expr.text else expr;


  # Visibility related functions
  isVisibleNameValue = name: v:
    if name == "_module" then
      false
    else if isOption v then
      isVisibleOption v
    else
      true;
  isVisibleOption = { visible ? true, internal ? false, ... }@opt:
    (if isString visible then visible == "shallow" else visible) && !internal;


  # Guard methods, they take a function for easy composability
  guardTryEval = f: v:
    let try = tryEval v;
    in if try.success then f try.value else { _error = true; };
  guardDerivation = f: v:
    if (isDerivation v) then {
      _derivation = true;
      # TODO: protect against errors here too?
      inherit (v) name meta;
    } else
      f v;
  guardFunction = f: v: if (isFunction v) then { _function = true; } else f v;
  guardOptionType = f: v: if (isOptionType v) then { _type = true; } else f v;

  # Recursively call on elements of product types with given function
  recurseWith = f: val:
    if (isAttrs val) then
      mapAttrs (k: v: f v) val
    else
      (if (isList val) then map f val else val);

  # Process config with recursive tryEval to root out `throw` errors
  catchErrors = guardTryEval (guardDerivation
    (guardFunction (guardOptionType (recurseWith catchErrors))));

  # Guard against values that don't make sense in JSON
  catchJson =
    guardDerivation (guardFunction (guardOptionType (recurseWith catchJson)));
  catchJsonLeaveOption =
    guardDerivation (guardFunction (recurseWith catchJson));
}
