use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Record(RecordArgs),
    Compare(CompareArgs),
    Qualify(QualifyArgs),
    Benchmark(BenchmarkArgs),
}

#[derive(Args)]
pub struct BrowserArgs {
    #[arg(long)]
    pub browser: Option<PathBuf>,
    #[arg(long)]
    pub cdp_url: Option<String>,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long, default_value_t = 1280)]
    pub height: u32,
}

#[derive(Args)]
pub struct RecordArgs {
    pub source: String,
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long, value_delimiter = ',', default_value = "320,390,768,1280,1440")]
    pub widths: Vec<u32>,
    #[command(flatten)]
    pub browser: BrowserArgs,
}

#[derive(Args)]
pub struct CompareArgs {
    pub artifact: PathBuf,
    pub candidate: String,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[command(flatten)]
    pub browser: BrowserArgs,
}

#[derive(Args)]
pub struct QualifyArgs {
    #[arg(long)]
    pub fixtures: PathBuf,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub holdouts: Option<PathBuf>,
    #[command(flatten)]
    pub browser: BrowserArgs,
}

#[derive(Args)]
pub struct BenchmarkArgs {
    pub artifact: PathBuf,
    pub candidate: String,
    #[arg(long, default_value_t = 20)]
    pub iterations: usize,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[command(flatten)]
    pub browser: BrowserArgs,
}
