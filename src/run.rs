use std::io::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

// TODO: Implement caching?
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

pub fn get_options() -> super::parse::NixValue {
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
