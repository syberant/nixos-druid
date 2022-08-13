use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::{BufReader, Error, Read};
use std::path::Path;
use std::process::{Command, Output};

#[derive(Debug)]
pub enum LoadJsonError {
    /// Errors where the command could not be properly called
    FailedCommand(std::io::Error),
    /// Errors where the resulting JSON output could not be parsed
    ParseRelated(serde_json::Error),
    /// Errors returned by `nix-instantiate` on a nonzero status code
    FailedEval(String),
}

fn run_nix_file(file: &Path) -> Result<Output, Error> {
    Command::new("nix-instantiate")
        .args([
            "--json",
            "--strict",
            "--eval",
            file.to_str().expect("UTF-8 validation of file path failed"),
        ])
        .output()
}

fn load_json_file(name: &Path) -> std::io::Result<String> {
    let file = File::open(name)?;
    let mut buf_reader = BufReader::new(file);
    let mut content = String::new();
    buf_reader.read_to_string(&mut content)?;
    Ok(content)
}

fn load_from_cache_or_eval<V: DeserializeOwned>(
    file: &Path,
    cache_file: &Path,
) -> Result<V, LoadJsonError> {
    if let Ok(content) = load_json_file(cache_file) {
        if let Ok(parsed) = serde_json::from_str(&content) {
            return Ok(parsed);
        } else {
            eprintln!(
                "Parsing json produced by cached `{}` failed, ignoring cache",
                file.display()
            );
        }
    }

    let command = run_nix_file(file);

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
pub fn get_available_nixos_configurations() -> Vec<String> {
    let result = Command::new("nix-instantiate")
        .args([
            "--eval",
            "--strict",
            "--json",
            "-E",
            "with builtins; attrNames (getFlake \"/etc/nixos\").nixosConfigurations",
        ])
        .output();

    serde_json::from_slice(&result.unwrap().stdout)
        .expect("Parsing json containing available nixosConfigurations failed.")
}

pub fn get_options() -> Result<super::parse::NixValue, LoadJsonError> {
    let cache_file = Path::new("/tmp/nixos.json");
    let file = Path::new("extract.nix");

    load_from_cache_or_eval(file, cache_file)
}

pub fn get_config() -> Result<super::parse::NixGuardedValue, LoadJsonError> {
    let cache_file = Path::new("/tmp/nixosConfig.json");
    let file = Path::new("extractConfig.nix");

    load_from_cache_or_eval(file, cache_file)
}
