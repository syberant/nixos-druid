use druid::Data;
use serde::Deserialize;
use serde_json::Value;
use std::boxed::Box;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize, Debug, Clone)]
struct NixSubmodule {
    _submodule: bool,
    options: NixSet,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
struct NixType {
    _type: bool,
    description: String,
    functorName: String,
    name: String,
    nestedTypes: Value,

    #[serde(default)]
    functorPayload: Vec<Value>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
struct NixInfiniteRecursion {
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
enum NixTypeValue {
    Type(NixType),
    Submodule(NixSubmodule),
    InfiniteRecursion(NixInfiniteRecursion),
}

#[derive(Deserialize, Debug, Clone)]
pub struct NixOption {
    _option: bool,
    description: String,
    r#type: NixTypeValue,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum NixValue {
    Option(NixOption),
    Set(NixSet),
}

type NixSet = HashMap<String, Box<NixValue>>;

pub fn get_root() -> NixValue {
    let file = File::open("/tmp/nixos.json").unwrap();
    return serde_json::from_reader(BufReader::new(file)).unwrap();
}
