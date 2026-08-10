mod asset_script;
mod attribute_sequence_script;
mod backtest;
mod behavior;
mod blocking_overlay;
mod browser;
#[cfg(test)]
mod browser_session_tests;
mod capture;
mod capture_settle;
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
mod generate;
mod interaction_rebase;
mod interaction_script;
mod interaction_state;
mod interaction_surface;
mod interactions;
mod interactions_input;
mod lifecycle_scheduled_script;
mod lifecycle_script;
mod lifecycle_settle_script;
mod model;
mod page_script;
mod probe;
#[cfg(test)]
mod release_gate_tests;
mod rule_activation_script;
#[cfg(test)]
#[path = "sequence_termination_tests.rs"]
mod sequence_termination_tests;
mod serve;
mod skill;
mod state_style_script;
mod style_baseline;
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
        Command::Backtest { args } => backtest::run(args).await,
        Command::Capture(args) => capture::run(args).await,
        Command::Generate(args) => generate::from_file(&args.spec, &args.out).await,
        Command::Install(args) => skill::install(args).await,
        Command::Open(args) => browser::open(args).await,
        Command::Verify(args) => compare::run(args).await,
    }
}

#[cfg(test)]
mod node_eval;
