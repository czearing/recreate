mod asset_script;
mod attribute_sequence_script;
mod behavior;
mod browser;
mod capture;
mod capture_startup;
#[cfg(test)]
mod capture_startup_tests;
mod cdp;
mod cli;
mod compare;
mod compare_animation;
mod compare_capture;
mod compare_css_value;
mod compare_dom;
mod compare_node;
#[cfg(test)]
mod compare_tests;
mod fidelity;
mod fidelity_responsive;
mod fidelity_responsive_script;
#[cfg(test)]
mod fidelity_responsive_tests;
mod fidelity_script;
mod generate;
mod interaction_rebase;
mod interaction_script;
mod interaction_state;
mod interaction_surface;
mod interactions;
mod interactions_input;
mod lifecycle_script;
mod model;
mod oracle_command;
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
        Command::Fidelity(args) => fidelity::run(args).await,
        Command::Generate(args) => {
            generate::from_file(&args.spec, &args.out).await?;
            if let Some(artifact) = args.oracle {
                oracle_command::embed(&artifact, &args.out)?;
            }
            Ok(())
        }
        Command::Install(args) => skill::install(args),
        Command::Open(args) => browser::open(args).await,
        Command::Oracle(args) => oracle_command::run(args).await,
        Command::Skill => {
            print!("{}", skill::workflow());
            Ok(())
        }
        Command::Verify(args) => compare::run(args).await,
    }
}
