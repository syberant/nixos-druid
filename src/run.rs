use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::{BufReader, Error, Read};
use std::path::Path;
use std::process::{Command, Output};

const EXTRACT_NIX: &'static str = include_str!("../nix-scripts/extract.nix");
const UTILITIES_NIX: &'static str = include_str!("../nix-scripts/utilities.nix");
const EXTRACT_CONFIG_NIX: &'static str = include_str!("../nix-scripts/extractConfig.nix");

#[derive(Debug)]
pub enum LoadJsonError {
    /// Errors where the command could not be properly called
    FailedCommand(std::io::Error),
    /// Errors where the resulting JSON output could not be parsed
    ParseRelated(serde_json::Error),
    /// Errors returned by `nix-instantiate` on a nonzero status code
    FailedEval(String),
}

pub fn run_nix_file(file: &Path) -> Result<Output, Error> {
    Command::new("nix-instantiate")
        .args([
            "--json",
            "--strict",
            "--eval",
            file.to_str().expect("UTF-8 validation of file path failed"),
        ])
        .output()
}

pub fn run_nix_str(nix_code: &str) -> Result<Output, Error> {
    Command::new("nix-instantiate")
        .args(["--json", "--strict", "--eval", "-E", nix_code])
        .output()
}

pub fn run_nix_str_with_args<A, B, I>(nix_code: &str, args: I) -> Result<Output, Error>
where
    A: ToString,
    B: ToString,
    I: IntoIterator<Item = (bool, A, B)>,
{
    let formatted_args = args
        .into_iter()
        .map(|(string_arg, name, value)| {
            [
                if string_arg { "--argstr" } else { "--arg" }.to_string(),
                name.to_string(),
                value.to_string(),
            ]
        })
        .flatten();

    Command::new("nix-instantiate")
        .args(["--json", "--strict", "--eval", "-E", nix_code])
        .args(formatted_args)
        .output()
}

fn load_json_file(name: &Path) -> std::io::Result<String> {
    let file = File::open(name)?;
    let mut buf_reader = BufReader::new(file);
    let mut content = String::new();
    buf_reader.read_to_string(&mut content)?;
    Ok(content)
}

fn load_from_cache_or_eval<V, A, B, I>(
    nix_code: &str,
    args: I,
    cache_file: &Path,
) -> Result<V, LoadJsonError>
where
    V: DeserializeOwned,
    A: ToString,
    B: ToString,
    I: IntoIterator<Item = (bool, A, B)>,
{
    if let Ok(content) = load_json_file(cache_file) {
        if let Ok(parsed) = serde_json::from_str(&content) {
            return Ok(parsed);
        } else {
            eprintln!(
                "Parsing json produced by cached `{}` failed, ignoring cache",
                cache_file.display()
            );
        }
    }

    let command = run_nix_str_with_args(nix_code, args);
    match command {
        Ok(output) => {
            if output.status.success() {
                serde_json::from_slice(&output.stdout).map_err(|e| LoadJsonError::ParseRelated(e))
            } else {
                Err(LoadJsonError::FailedEval(format!(
                    "Exited with status: {}\nError output: {}",
                    output.status,
                    String::from_utf8(output.stderr).expect("Command outputted invalid UTF-8 text")
                )))
            }
        }
        Err(e) => Err(LoadJsonError::FailedCommand(e)),
    }
}

/// Returns a list with all the attribute names of <flake>.nixosConfigurations
pub fn get_available_nixos_configurations(flake_path: &str) -> Option<Vec<String>> {
    let cmd = format!("with builtins; attrNames (getFlake \"{flake_path}\").nixosConfigurations");
    let result = run_nix_str(&cmd).ok()?;

    serde_json::from_slice(&result.stdout).ok()
}

pub fn get_options() -> Result<super::parse::NixValue, LoadJsonError> {
    let cache_file = Path::new("/tmp/nixos.json");

    let args = [(false, "utilities", UTILITIES_NIX)];
    load_from_cache_or_eval(EXTRACT_NIX, args, cache_file)
}

pub fn get_config(
    flake: &str,
    hostname: &str,
) -> Result<super::parse::NixGuardedValue, LoadJsonError> {
    let cache_file = Path::new("/tmp/nixosConfig.json");

    let args = [
        (false, "utilities", UTILITIES_NIX),
        (true, "flakePath", flake),
        (true, "hostname", hostname),
    ];
    load_from_cache_or_eval(EXTRACT_CONFIG_NIX, args, cache_file)
}
