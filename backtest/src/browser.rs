use crate::{cdp::Cdp, cli::Side, model::Viewport};
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Target {
    pub id: String,
    #[serde(default)]
    pub url: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub websocket_url: String,
    #[serde(default, rename = "type")]
    pub kind: String,
}

pub struct BrowserProcess {
    child: Option<Child>,
}

impl Drop for BrowserProcess {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn endpoint(side: Side) -> &'static str {
    match side {
        Side::Source => "http://127.0.0.1:9331",
        Side::Candidate => "http://127.0.0.1:9332",
    }
}

pub async fn ensure(
    endpoint: &str,
    side: &str,
    visible: bool,
    profile_override: Option<&Path>,
) -> anyhow::Result<BrowserProcess> {
    if ready(endpoint).await {
        return Ok(BrowserProcess { child: None });
    }
    let executable = find_executable().context("Chrome, Edge, or Chromium not found")?;
    let port = endpoint
        .rsplit_once(':')
        .and_then(|(_, value)| value.parse::<u16>().ok())
        .context("CDP endpoint must include a port")?;
    let profile = match profile_override {
        Some(value) => value.to_path_buf(),
        None => home()?
            .join(".recreate-backtest")
            .join("profiles")
            .join(side),
    };
    std::fs::create_dir_all(&profile)?;
    let mut command = Command::new(executable);
    command
        .args([
            format!("--remote-debugging-port={port}"),
            format!("--user-data-dir={}", profile.display()),
            "--no-first-run".into(),
            "--no-default-browser-check".into(),
            "--force-device-scale-factor=1".into(),
            "--disable-background-networking".into(),
            "--disable-component-update".into(),
            "--disable-sync".into(),
            "--new-window".into(),
            "about:blank".into(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if !visible {
        command.arg("--headless=new");
    }
    let child = command.spawn().context("launch browser")?;
    for _ in 0..80 {
        if ready(endpoint).await {
            return Ok(BrowserProcess { child: Some(child) });
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    bail!("browser endpoint did not start: {endpoint}")
}

pub async fn ready(endpoint: &str) -> bool {
    reqwest::get(format!("{endpoint}/json/version"))
        .await
        .is_ok_and(|response| response.status().is_success())
}

pub async fn version(endpoint: &str) -> anyhow::Result<String> {
    let value: serde_json::Value = reqwest::get(format!("{endpoint}/json/version"))
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(value["Browser"].as_str().unwrap_or_default().into())
}

pub async fn list(endpoint: &str) -> anyhow::Result<Vec<Target>> {
    Ok(reqwest::get(format!("{endpoint}/json/list"))
        .await?
        .error_for_status()?
        .json()
        .await?)
}

pub async fn create(endpoint: &str, url: &str) -> anyhow::Result<Target> {
    let encoded: String = url::form_urlencoded::byte_serialize(url.as_bytes()).collect();
    Ok(reqwest::Client::new()
        .put(format!("{endpoint}/json/new?{encoded}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

pub async fn target(endpoint: &str, id: &str) -> anyhow::Result<Target> {
    list(endpoint)
        .await?
        .into_iter()
        .find(|value| value.id == id)
        .with_context(|| format!("target not found: {id}"))
}

pub async fn activate(endpoint: &str, id: &str) -> anyhow::Result<()> {
    reqwest::get(format!("{endpoint}/json/activate/{id}"))
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn close(endpoint: &str, id: &str) -> anyhow::Result<()> {
    reqwest::get(format!("{endpoint}/json/close/{id}"))
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn connect(
    endpoint: &str,
    id: &str,
    timeout: Duration,
) -> anyhow::Result<(Target, Cdp)> {
    let target = target(endpoint, id).await?;
    let cdp = Cdp::connect(&target.websocket_url, timeout).await?;
    Ok((target, cdp))
}

pub async fn set_viewport(cdp: &mut Cdp, viewport: &Viewport) -> anyhow::Result<()> {
    cdp.send(
        "Emulation.setDeviceMetricsOverride",
        json!({
            "width":viewport.width,
            "height":viewport.height,
            "deviceScaleFactor":1,
            "mobile":viewport.width < 600
        }),
    )
    .await?;
    Ok(())
}

fn find_executable() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("RECREATE_BACKTEST_BROWSER") {
        let path = PathBuf::from(value);
        if path.exists() {
            return Some(path);
        }
    }
    candidates().into_iter().find(|path| path.exists())
}

fn candidates() -> Vec<PathBuf> {
    if cfg!(windows) {
        return [
            std::env::var_os("PROGRAMFILES(X86)"),
            std::env::var_os("PROGRAMFILES"),
            std::env::var_os("LOCALAPPDATA"),
        ]
        .into_iter()
        .flatten()
        .flat_map(|root| {
            let root = Path::new(&root);
            [
                root.join("Microsoft\\Edge\\Application\\msedge.exe"),
                root.join("Google\\Chrome\\Application\\chrome.exe"),
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

fn home() -> anyhow::Result<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .context("home directory unavailable")
}

