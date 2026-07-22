use clap::Parser;
use recreate_oracle::{cli, engine};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    engine::run(cli::Cli::parse()).await
}
