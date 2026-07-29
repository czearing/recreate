use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "recreate-backtest", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Prepare(PrepareArgs),
    Record(RecordArgs),
    Compare(CompareArgs),
    Pipeline(PipelineArgs),
    Benchmark(BenchmarkArgs),
    Qualify(QualifyArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Side {
    Source,
    Candidate,
}

impl Side {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Candidate => "candidate",
        }
    }
}

#[derive(Args)]
pub struct PrepareArgs {
    #[arg(value_enum)]
    pub side: Side,
    pub url: String,
    #[arg(long)]
    pub session: PathBuf,
    #[arg(long)]
    pub ready_selector: Option<String>,
    #[arg(long)]
    pub non_interactive: bool,
    #[arg(long, default_value_t = 1440)]
    pub width: u32,
    #[arg(long, default_value_t = 900)]
    pub height: u32,
    #[arg(long)]
    pub cdp_url: Option<String>,
}

#[derive(Args)]
pub struct RecordArgs {
    #[arg(long)]
    pub session: PathBuf,
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long, default_value = "1440x900,390x844")]
    pub viewports: String,
}

#[derive(Args, Clone)]
pub struct CompareArgs {
    pub artifact: PathBuf,
    #[arg(long)]
    pub candidate_session: PathBuf,
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long, default_value_t = 4800)]
    pub budget_ms: u64,
}

#[derive(Args)]
pub struct PipelineArgs {
    #[arg(long)]
    pub recreate_bin: PathBuf,
    #[arg(long)]
    pub source_url: String,
    #[arg(long)]
    pub candidate_url: String,
    #[arg(long)]
    pub work_dir: PathBuf,
    #[arg(long)]
    pub recreate_args: Vec<String>,
}

#[derive(Args)]
pub struct BenchmarkArgs {
    pub artifact: PathBuf,
    #[arg(long)]
    pub candidate_session: PathBuf,
    #[arg(long, default_value_t = 20)]
    pub iterations: usize,
    #[arg(long, default_value_t = 4800)]
    pub budget_ms: u64,
}

#[derive(Args)]
pub struct QualifyArgs {
    #[arg(long, default_value = "fixtures")]
    pub fixtures: PathBuf,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long, default_value_t = 20)]
    pub repeat: usize,
}

