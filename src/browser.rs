use crate::{
    cdp::Cdp,
    cli::{CaptureArgs, OpenArgs},
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Target {
    pub id: String,
    pub url: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub websocket_url: String,
}

pub async fn open(args: OpenArgs) -> Result<()> {
    ensure_endpoint(&args.cdp_url).await?;
    let target = create(&args.cdp_url, &args.url).await?;
    reqwest::get(format!("{}/json/activate/{}", args.cdp_url, target.id))
        .await?
        .error_for_status()?;
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
    let cdp = Cdp::connect(&target.websocket_url).await?;
    Ok((target, cdp))
}

pub async fn list(endpoint: &str) -> Result<Vec<Target>> {
    Ok(reqwest::get(format!("{endpoint}/json/list"))
        .await?
        .error_for_status()?
        .json()
        .await?)
}

async fn create(endpoint: &str, url: &str) -> Result<Target> {
    let encoded: String = url::form_urlencoded::byte_serialize(url.as_bytes()).collect();
    let client = reqwest::Client::new();
    let created: Target = client
        .put(format!("{endpoint}/json/new?{encoded}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(created)
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
    let executable = browser_executable().context("Chrome, Edge, or Chromium not found")?;
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

fn browser_executable() -> Option<PathBuf> {
    candidates().into_iter().find(|path| path.exists())
}

fn candidates() -> Vec<PathBuf> {
    if cfg!(windows) {
        let roots = [
            std::env::var_os("PROGRAMFILES(X86)"),
            std::env::var_os("PROGRAMFILES"),
            std::env::var_os("LOCALAPPDATA"),
        ];
        return roots
            .into_iter()
            .flatten()
            .flat_map(|root| {
                let root = Path::new(&root);
                [
                    root.join("Microsoft/Edge/Application/msedge.exe"),
                    root.join("Google/Chrome/Application/chrome.exe"),
                ]
            })
            .collect();
    }
    vec![
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into(),
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge".into(),
        "/usr/bin/google-chrome".into(),
        "/usr/bin/chromium".into(),
        "/usr/bin/microsoft-edge".into(),
    ]
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
