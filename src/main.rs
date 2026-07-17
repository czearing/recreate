mod asset_script;
mod attribute_sequence_script;
mod browser;
mod capture;
mod capture_startup;
mod cdp;
mod cli;
mod compare;
mod compare_node;
#[cfg(test)]
mod compare_tests;
mod generate;
mod interaction_state;
mod interactions;
mod interactions_input;
mod lifecycle_script;
mod model;
mod page_script;
mod probe;
#[cfg(test)]
mod release_gate_tests;
mod skill;
mod state_style_script;
mod style_contract;
mod updater;
mod validate;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

#[tokio::main]
async fn main() -> Result<()> {
    if updater::refresh().await? {
        return Ok(());
    }
    let cli = Cli::parse();
    match cli.command {
        Command::Capture(args) => capture::run(args).await,
        Command::Generate(args) => generate::from_file(&args.spec, &args.out).await,
        Command::Install(args) => skill::install(args),
        Command::Open(args) => browser::open(args).await,
        Command::Skill => {
            print!("{}", skill::workflow());
            Ok(())
        }
        Command::Verify(args) => compare::run(args).await,
    }
}
