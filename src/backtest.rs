use crate::{browser, updater};
use anyhow::{Context, Result};
use std::ffi::OsString;

pub async fn run(mut args: Vec<OsString>) -> Result<()> {
    attach_remembered_source(&mut args)?;
    let binary = updater::ensure_backtest().await?;
    let status = tokio::process::Command::new(binary)
        .args(args)
        .status()
        .await?;
    std::process::exit(status.code().unwrap_or(1));
}

fn attach_remembered_source(args: &mut Vec<OsString>) -> Result<()> {
    let is_source_prepare = args.first().is_some_and(|value| value == "prepare")
        && args.get(1).is_some_and(|value| value == "source");
    let is_run = args.first().is_some_and(|value| value == "run");
    let wants_help = args.iter().any(|value| value == "--help" || value == "-h");
    let has_attachment = args
        .iter()
        .any(|value| value == "--cdp-url" || value == "--target" || value == "--source-cdp-url");
    if (!is_source_prepare && !is_run) || has_attachment || wants_help {
        return Ok(());
    }
    if is_run {
        attach_prepared_tabs(args)?;
    } else {
        let session = browser::last_open_session()?;
        args.extend([
            OsString::from("--cdp-url"),
            OsString::from(session.cdp_url),
            OsString::from("--target"),
            OsString::from(session.target),
        ]);
    }
    Ok(())
}

/// A comparison is only meaningful against the pages the developer actually
/// prepared, so both sides must already be open and neither is reloaded.
fn attach_prepared_tabs(args: &mut Vec<OsString>) -> Result<()> {
    for (url_option, cdp_option, target_option) in [
        ("--source", "--source-cdp-url", "--source-target"),
        (
            "--recreation",
            "--recreation-cdp-url",
            "--recreation-target",
        ),
    ] {
        let url = option_value(args, url_option)
            .with_context(|| format!("backtest run requires {url_option} <url>"))?;
        let session = browser::find_open_session(&url).with_context(|| {
            format!(
                "{url} is not open; run `recreate open {url}`, sign in and get the page to the state you want to compare, then run this command again"
            )
        })?;
        args.extend([
            OsString::from(cdp_option),
            OsString::from(session.cdp_url),
            OsString::from(target_option),
            OsString::from(session.target),
        ]);
    }
    Ok(())
}

fn option_value(args: &[OsString], option: &str) -> Option<String> {
    args.windows(2)
        .find(|values| values[0] == option)
        .map(|values| values[1].to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_source_attachment_is_preserved() {
        let mut args = ["prepare", "source", "--cdp-url", "http://localhost:1"]
            .map(OsString::from)
            .to_vec();
        attach_remembered_source(&mut args).unwrap();
        assert_eq!(args.len(), 4);
    }

    #[test]
    fn run_help_does_not_require_a_remembered_source() {
        let mut args = ["run", "--help"].map(OsString::from).to_vec();
        attach_remembered_source(&mut args).unwrap();
        assert_eq!(args, ["run", "--help"].map(OsString::from));
    }

    #[test]
    fn run_fails_closed_when_a_page_was_never_opened() {
        let mut args = [
            "run",
            "--source",
            "https://example.invalid/never-opened",
            "--recreation",
            "http://localhost:65535",
        ]
        .map(OsString::from)
        .to_vec();
        let error = attach_remembered_source(&mut args)
            .expect_err("comparison must not proceed against an unprepared page")
            .to_string();
        assert!(error.contains("recreate open"), "{error}");
    }
}
