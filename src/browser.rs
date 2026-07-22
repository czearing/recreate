use crate::{
    cdp::Cdp,
    cli::{CaptureArgs, OpenArgs},
};
use anyhow::{Context, Result, bail};
use serde_json::json;
use std::{path::PathBuf, process::Command, time::Duration};

pub use recreate_browser::Target;

pub async fn open(args: OpenArgs) -> Result<()> {
    ensure_endpoint(&args.cdp_url).await?;
    let target = create(&args.cdp_url, &args.url).await?;
    activate(&args.cdp_url, &target.id).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "cdp_url": args.cdp_url,
            "target": target.id,
            "url": target.url
        }))?
    );
    Ok(())
}

pub async fn target(args: &CaptureArgs) -> Result<(Target, Cdp)> {
    ensure_endpoint(&args.cdp_url).await?;
    let target = if args.reuse {
        let id = args
            .target
            .as_deref()
            .context("--reuse requires --target")?;
        list(&args.cdp_url)
            .await?
            .into_iter()
            .find(|target| target.id == id)
            .with_context(|| format!("open target not found: {id}"))?
    } else {
        let url = args.url.as_deref().context("capture requires a URL")?;
        create(&args.cdp_url, url).await?
    };
    if args.reuse {
        activate(&args.cdp_url, &target.id).await?;
    }
    let cdp = Cdp::connect(&target.websocket_url).await?;
    Ok((target, cdp))
}

pub async fn list(endpoint: &str) -> Result<Vec<Target>> {
    recreate_browser::list(endpoint).await
}

async fn create(endpoint: &str, url: &str) -> Result<Target> {
    recreate_browser::create(endpoint, url).await
}

async fn activate(endpoint: &str, id: &str) -> Result<()> {
    recreate_browser::activate(endpoint, id).await
}

async fn ensure_endpoint(endpoint: &str) -> Result<()> {
    if reqwest::get(format!("{endpoint}/json/version"))
        .await
        .is_ok_and(|response| response.status().is_success())
    {
        return Ok(());
    }
    launch(endpoint)?;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if reqwest::get(format!("{endpoint}/json/version"))
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return Ok(());
        }
    }
    bail!("browser debugging endpoint did not start: {endpoint}")
}

fn launch(endpoint: &str) -> Result<()> {
    let port = endpoint
        .rsplit_once(':')
        .and_then(|(_, value)| value.parse::<u16>().ok())
        .context("CDP endpoint must include a port")?;
    let executable =
        recreate_browser::find_executable().context("Chrome, Edge, or Chromium not found")?;
    let profile = dirs_home()?
        .join(".recreate")
        .join(format!("browser-profile-{port}"));
    std::fs::create_dir_all(&profile)?;
    Command::new(executable)
        .args([
            format!("--remote-debugging-port={port}"),
            format!("--user-data-dir={}", profile.display()),
            "--no-first-run".into(),
            "--no-default-browser-check".into(),
            "--force-device-scale-factor=1".into(),
            "--new-window".into(),
            "about:blank".into(),
        ])
        .spawn()
        .context("start browser")?;
    Ok(())
}

fn dirs_home() -> Result<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .context("home directory unavailable")
}

pub async fn set_viewport(cdp: &mut Cdp, width: u32, height: u32) -> Result<()> {
    cdp.send(
        "Emulation.setDeviceMetricsOverride",
        json!({
            "width": width,
            "height": height,
            "deviceScaleFactor": 1,
            "mobile": width < 600
        }),
    )
    .await?;
    Ok(())
}
