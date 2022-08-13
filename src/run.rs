use std::fs::File;
use std::io::{BufReader, Error, Read};
use std::path::Path;
use std::process::{Command, Output};

// TODO: Implement caching?
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

pub fn get_options() -> super::parse::NixValue {
    let cache_file = Path::new("/tmp/nixos.json");
    if let Ok(content) = load_json_file(&cache_file) {
        return serde_json::from_str(&content)
            .expect("Parsing json produced by cached `extract.nix` failed");
    }

    let file = Path::new("extract.nix");
    let command = run_nix_file(&file);

    if let Ok(output) = command {
        if output.status.success() {
            return serde_json::from_slice(&output.stdout)
                .expect("Parsing json produced by `extract.nix` failed");
        } else {
            match output.status {
                other => eprintln!(
                    "Exited with status: {}\nError output: {}",
                    other,
                    String::from_utf8(output.stderr)
                        .expect("Command outputted nonvalid UTF-8 text")
                ),
            }
        }
    }
    unimplemented!()
}

pub fn get_config() -> super::parse::NixGuardedValue {
    let cache_file = Path::new("/tmp/nixosConfig.json");
    if let Ok(content) = load_json_file(&cache_file) {
        return serde_json::from_str(&content)
            .expect("Parsing json produced by cached `extractConfig.nix` failed");
    }

    let file = Path::new("extractConfig.nix");
    let command = run_nix_file(&file);

    if let Ok(output) = command {
        if output.status.success() {
            return serde_json::from_slice(&output.stdout)
                .expect("Parsing json produced by `extractConfig.nix` failed");
        } else {
            match output.status {
                other => eprintln!(
                    "Exited with status: {}\nError output: {}",
                    other,
                    String::from_utf8(output.stderr)
                        .expect("Command outputted nonvalid UTF-8 text")
                ),
            }
        }
    }
    unimplemented!()
}
