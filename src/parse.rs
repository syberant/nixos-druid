use serde::Deserialize;
use serde_json::Value;
use std::boxed::Box;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize, Debug, Clone)]
pub struct NixGuardedOptionType {
    _type: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NixDerivation {
    _derivation: bool,
    pub name: String,
    // pub meta: Value,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NixFunction {
    _function: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NixTryEvalError {
    _error: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NixLiteralExpression {
    // Needs to be "literalExpression"
    _type: String,
    pub text: String,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum NixGuardedValue {
    LiteralExpression(NixLiteralExpression),
    Function(NixFunction),
    Error(NixTryEvalError),
    Derivation(NixDerivation),
    OptionType(NixGuardedOptionType),

    // Recursive variants
    Attrs(HashMap<String, Box<NixGuardedValue>>),
    List(Vec<Box<NixGuardedValue>>),

    // Basic
    String(String),
    Number(i64),
    Float(f64),
    Bool(bool),
    Null(()),

    // Maybe turn all of the basic and recursive variants into an "other"
    // value and keep them as a serialized JSON string?
    // Need to handle nested derivations, etc. though
}

impl std::fmt::Display for NixGuardedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Simply use the pretty-print output of `Debug`
        write!(f, "{}", format!("{self:#?}"))
    }
}

impl std::fmt::Debug for NixGuardedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use NixGuardedValue::*;

        match self {
            OptionType(_) => write!(f, "<option type>"),
            Function(_) => write!(f, "<function>"),
            Derivation(d) => write!(f, "<build of {}>", d.name),
            Error(_) => write!(f, "<error>"),
            LiteralExpression(e) => write!(f, "```{}```", e.text),

            String(s) => write!(f, "\"{}\"", s),
            Number(n) => write!(f, "{}", n),
            Float(n) => write!(f, "{}", n),
            Bool(b) => write!(f, "{}", b),
            Null(_) => write!(f, "null"),


            Attrs(children) => {
                f.debug_map().entries(children.iter()).finish()
            },
            List(children) => {
                f.debug_list().entries(children.iter()).finish()
            },

            // other => write!(f, "{:#?}", other),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct NixSubmodule {
    _submodule: bool,
    pub options: NixSet,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct NixType {
    _type: bool,
    pub description: String,
    functorName: String,
    pub name: String,
    pub nestedTypes: HashMap<String,NixTypeValue>,

    #[serde(default)]
    functorPayload: Vec<Value>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct NixInfiniteRecursion {
    _infiniteRecursion: bool,
    #[serde(default)]
    _standard: bool,
    // description: String,
    // functorName: String,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum NixTypeValue {
    Type(NixType),
    Submodule(NixSubmodule),
    InfiniteRecursion(NixInfiniteRecursion),
}

impl NixTypeValue {
    pub fn get_submodule(&self) -> Option<&NixSubmodule> {
        match self {
            NixTypeValue::Submodule(s) => Some(s),
            _ => None,
        }
    }
}

impl std::fmt::Display for NixTypeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use NixTypeValue::*;

        match self {
            Type(t) => write!(f, "{}", t.description),
            Submodule(_) => write!(f, "submodule"),
            InfiniteRecursion(_) => write!(f, "infinite recursion error"),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct NixOption {
    _option: bool,
    pub description: String,
    pub r#type: NixTypeValue,
    pub default: Option<NixGuardedValue>,
    pub example: Option<NixGuardedValue>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum NixValue {
    Option(NixOption),
    Set(NixSet),
}

pub type NixSet = HashMap<String, Box<NixValue>>;

pub fn get_root() -> NixValue {
    let file = File::open("/tmp/nixos.json").unwrap();
    return serde_json::from_reader(BufReader::new(file)).unwrap();
}
