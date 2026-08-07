use clap::{Args, Parser, Subcommand};
use std::{ffi::OsString, path::PathBuf};

#[derive(Parser)]
#[command(name = "recreate", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(disable_help_flag = true)]
    Backtest {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    Capture(CaptureArgs),
    Generate(GenerateArgs),
    Install(InstallArgs),
    Open(OpenArgs),
    Verify(VerifyArgs),
}

#[derive(Args, Clone)]
pub struct OpenArgs {
    /// Source page to open in Recreate's visible browser.
    pub url: String,
    /// Advanced override for the browser debugging endpoint.
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
    /// Source URL, or a local directory to serve and capture. Omit when using --reuse.
    pub url: Option<String>,
    /// Capture the page most recently opened by `recreate open`.
    #[arg(long)]
    pub reuse: bool,
    /// Reload after instrumentation to record startup behavior.
    #[arg(long)]
    pub reload: bool,
    /// Write capture evidence without generating the React project.
    #[arg(long)]
    pub spec_only: bool,
    /// Advanced override for a specific browser tab.
    #[arg(long)]
    pub target: Option<String>,
    /// Advanced override for the browser debugging endpoint.
    #[arg(long, default_value = "http://127.0.0.1:9222")]
    pub cdp_url: String,
    /// Directory for the generated recreation and capture evidence.
    #[arg(long, default_value = "recreate-output")]
    pub out: PathBuf,
    /// Comma-separated viewport sizes to capture.
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
    fn forwards_backtest_arguments_unchanged() {
        let cli = Cli::try_parse_from([
            "recreate",
            "backtest",
            "compare",
            "--source",
            "source.json",
            "--focus",
            "toolbar",
        ])
        .unwrap();
        let Command::Backtest { args } = cli.command else {
            panic!("expected backtest");
        };
        assert_eq!(
            args,
            ["compare", "--source", "source.json", "--focus", "toolbar"].map(OsString::from)
        );
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
    fn parses_fast_spec_capture() {
        let cli = Cli::try_parse_from([
            "recreate",
            "capture",
            "--spec-only",
            "--viewports",
            "1440x900",
        ])
        .unwrap();
        let Command::Capture(args) = cli.command else {
            panic!("expected capture");
        };
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
