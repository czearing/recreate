use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "recreate", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Capture(CaptureArgs),
    Fidelity(FidelityArgs),
    Generate(GenerateArgs),
    Install(InstallArgs),
    Open(OpenArgs),
    Skill,
    Verify(VerifyArgs),
}

#[derive(Args, Clone)]
pub struct FidelityArgs {
    #[arg(long)]
    pub source_target: String,
    #[arg(long)]
    pub candidate_target: String,
    #[arg(long)]
    pub label: String,
    #[arg(long, default_value_t = 1440)]
    pub width: u32,
    #[arg(long, default_value_t = 900)]
    pub height: u32,
    #[arg(long, default_value = "320,390,480,600,768,960,1200,1440,1920")]
    pub widths: String,
    #[arg(long, default_value = "http://127.0.0.1:9223")]
    pub cdp_url: String,
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Args, Clone)]
pub struct OpenArgs {
    pub url: String,
    #[arg(long, default_value = "http://127.0.0.1:9223")]
    pub cdp_url: String,
}

#[derive(Args, Clone)]
pub struct GenerateArgs {
    #[arg(long)]
    pub spec: PathBuf,
    #[arg(long)]
    pub out: PathBuf,
}

#[derive(Args, Clone)]
pub struct VerifyArgs {
    #[arg(long)]
    pub spec: PathBuf,
    #[arg(long)]
    pub url: String,
    #[arg(long, default_value = "http://127.0.0.1:9222")]
    pub cdp_url: String,
    #[arg(long)]
    pub interaction: Option<usize>,
}

#[derive(Args, Clone)]
pub struct CaptureArgs {
    pub url: Option<String>,
    #[arg(long)]
    pub reuse: bool,
    #[arg(long)]
    pub reload: bool,
    #[arg(long)]
    pub baseline_only: bool,
    #[arg(long)]
    pub spec_only: bool,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long, default_value = "http://127.0.0.1:9222")]
    pub cdp_url: String,
    #[arg(long, default_value = "recreate-output")]
    pub out: PathBuf,
    #[arg(long, default_value = "1920x1080,1440x900,768x1024,390x844,320x568")]
    pub viewports: String,
}

#[derive(Args, Clone)]
pub struct InstallArgs {
    #[arg(long)]
    pub copilot: bool,
    #[arg(long)]
    pub claude: bool,
    #[arg(long)]
    pub all: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_page_capture() {
        let cli = Cli::try_parse_from([
            "recreate",
            "capture",
            "https://example.com",
            "--viewports",
            "1200x800,390x844",
        ])
        .unwrap();
        let Command::Capture(args) = cli.command else {
            panic!("expected capture");
        };
        assert_eq!(args.url.as_deref(), Some("https://example.com"));
        assert_eq!(args.viewports, "1200x800,390x844");
    }

    #[test]
    fn defaults_to_five_responsive_layouts() {
        let cli = Cli::try_parse_from(["recreate", "capture", "https://example.com"]).unwrap();
        let Command::Capture(args) = cli.command else {
            panic!("expected capture");
        };
        assert_eq!(
            args.viewports,
            "1920x1080,1440x900,768x1024,390x844,320x568"
        );
    }

    #[test]
    fn parses_fast_baseline_spec_capture() {
        let cli = Cli::try_parse_from([
            "recreate",
            "capture",
            "--baseline-only",
            "--spec-only",
            "--viewports",
            "1440x900",
        ])
        .unwrap();
        let Command::Capture(args) = cli.command else {
            panic!("expected capture");
        };
        assert!(args.baseline_only);
        assert!(args.spec_only);
    }

    #[test]
    fn opens_a_visible_authentication_target() {
        let cli = Cli::try_parse_from(["recreate", "open", "https://example.com"]).unwrap();
        let Command::Open(args) = cli.command else {
            panic!("expected open");
        };
        assert_eq!(args.url, "https://example.com");
        assert_eq!(args.cdp_url, "http://127.0.0.1:9223");
    }
}
