mod browser;
mod capture;
mod cdp;
mod cli;
mod compare;
#[cfg(test)]
mod compare_tests;
mod generate;
mod interactions;
mod lifecycle_script;
mod model;
mod page_script;
mod probe;
mod skill;
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
        Command::Skill => {
            print!("{}", skill::workflow());
            Ok(())
        }
        Command::Verify(args) => compare::run(args).await,
    }
}
