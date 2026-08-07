mod blackbox;
mod browser;
mod capture;
mod cdp;
mod cli;
mod compare;
mod deadline;
mod digest;
mod fixture;
mod instrumentation;
mod model;
mod report;
mod server;
mod sweep;

use clap::Parser;

#[tokio::main]
async fn main() {
    if let Err(error) = cli::Cli::parse().run().await {
        eprintln!("recreate-backtest: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod architecture_tests {
    use std::{fs, path::Path};

    #[test]
    fn package_is_a_standalone_workspace_without_recreate_imports() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(manifest.lines().any(|line| line.trim() == "[workspace]"));
        assert!(!manifest.contains("path ="));

        let repository_manifest = fs::read_to_string(root.join("..").join("Cargo.toml")).unwrap();
        let workspace_members = repository_manifest
            .lines()
            .find(|line| line.trim_start().starts_with("members ="))
            .unwrap();
        assert!(!workspace_members.contains("backtest"));

        let lock = fs::read_to_string(root.join("Cargo.lock")).unwrap();
        assert!(!lock.contains("name = \"recreate\""));
        assert!(!lock.contains("name = \"recreate-browser\""));

        let recreate_namespace = ["recreate", "::"].concat();
        let browser_crate_needle = ["recreate", "_browser"].concat();
        let mut pending = vec![root.join("src")];
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                    let source = fs::read_to_string(path).unwrap();
                    assert!(!source.contains(&recreate_namespace));
                    assert!(!source.contains(&browser_crate_needle));
                }
            }
        }
    }
}
