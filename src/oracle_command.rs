use crate::cli::OracleArgs;
use clap::Parser;

pub async fn run(args: OracleArgs) -> anyhow::Result<()> {
    let arguments = std::iter::once(std::ffi::OsString::from("recreate-oracle"))
        .chain(args.args)
        .collect::<Vec<_>>();
    let cli = recreate_oracle::cli::Cli::try_parse_from(arguments)?;
    recreate_oracle::engine::run(cli).await
}

pub fn embed(artifact: &std::path::Path, output: &std::path::Path) -> anyhow::Result<()> {
    let verified = recreate_oracle::artifact::read(artifact)?;
    let destination = output
        .join("react")
        .join("public")
        .join("recreate-oracle.json");
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow::anyhow!("oracle destination has no parent"))?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(destination, serde_json::to_vec_pretty(&verified)?)?;
    Ok(())
}
