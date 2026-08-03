use crate::{browser, updater};
use anyhow::{Context, Result};
use std::ffi::OsString;

pub async fn run(mut args: Vec<OsString>) -> Result<()> {
    attach_remembered_source(&mut args).await?;
    let binary = updater::ensure_backtest().await?;
    let status = tokio::process::Command::new(binary)
        .args(args)
        .status()
        .await?;
    std::process::exit(status.code().unwrap_or(1));
}

async fn attach_remembered_source(args: &mut Vec<OsString>) -> Result<()> {
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
        attach_prepared_tabs(args).await?;
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
/// prepared, so both sides must still be open and neither is reloaded.
async fn attach_prepared_tabs(args: &mut Vec<OsString>) -> Result<()> {
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
        let session =
            browser::find_open_session(&url).with_context(|| prepare_instruction(&url))?;
        let live = browser::list(&session.cdp_url)
            .await
            .is_ok_and(|targets| targets.iter().any(|target| target.id == session.target));
        anyhow::ensure!(live, "{}", prepare_instruction(&url));
        args.extend([
            OsString::from(cdp_option),
            OsString::from(session.cdp_url),
            OsString::from(target_option),
            OsString::from(session.target),
        ]);
    }
    Ok(())
}

fn prepare_instruction(url: &str) -> String {
    format!(
        "{url} is not open; run `recreate open {url}`, get the page to the state you want to compare, then run this command again"
    )
}

fn option_value(args: &[OsString], option: &str) -> Option<String> {
    args.windows(2)
        .find(|values| values[0] == option)
        .map(|values| values[1].to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn explicit_source_attachment_is_preserved() {
        let mut args = ["prepare", "source", "--cdp-url", "http://localhost:1"]
            .map(OsString::from)
            .to_vec();
        attach_remembered_source(&mut args).await.unwrap();
        assert_eq!(args.len(), 4);
    }

    #[tokio::test]
    async fn run_help_does_not_require_a_remembered_source() {
        let mut args = ["run", "--help"].map(OsString::from).to_vec();
        attach_remembered_source(&mut args).await.unwrap();
        assert_eq!(args, ["run", "--help"].map(OsString::from));
    }

    #[tokio::test]
    async fn run_fails_closed_when_a_page_was_never_opened() {
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
            .await
            .expect_err("comparison must not proceed against an unprepared page")
            .to_string();
        assert!(error.contains("recreate open"), "{error}");
    }

    #[tokio::test]
    async fn run_fails_closed_when_a_remembered_tab_is_no_longer_live() {
        let temporary = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var("USERPROFILE", temporary.path()) };
        let recreate = temporary.path().join(".recreate");
        std::fs::create_dir_all(&recreate).unwrap();
        std::fs::write(
            recreate.join("open-tabs.json"),
            r#"[{"cdp_url":"http://127.0.0.1:1","target":"DEAD","url":"http://localhost:8090/","rendered_url":"http://localhost:8090/"}]"#,
        )
        .unwrap();
        let mut args = [
            "run",
            "--source",
            "http://localhost:8090/",
            "--recreation",
            "http://localhost:8090/",
        ]
        .map(OsString::from)
        .to_vec();
        let error = attach_remembered_source(&mut args)
            .await
            .expect_err("a closed browser must not be compared")
            .to_string();
        assert!(error.contains("recreate open"), "{error}");
    }
}
