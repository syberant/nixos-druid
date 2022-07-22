use druid::Data;
use serde::Deserialize;
use serde_json::Value;
use std::boxed::Box;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

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
    name: String,
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
