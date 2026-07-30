use crate::{
    cdp::Cdp,
    cli::{CaptureArgs, OpenArgs},
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::PathBuf, process::Command, time::Duration};

pub use recreate_browser::Target;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct OpenSession {
    pub cdp_url: String,
    pub target: String,
    pub url: String,
    #[serde(default)]
    pub rendered_url: String,
}

pub async fn open(args: OpenArgs) -> Result<()> {
    ensure_endpoint(&args.cdp_url).await?;
    let target = create(&args.cdp_url, &args.url).await?;
    activate(&args.cdp_url, &target.id).await?;
    save_open_session(&OpenSession {
        cdp_url: args.cdp_url,
        target: target.id,
        url: args.url,
        rendered_url: target.url.clone(),
    })?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "opened",
            "url": target.url,
        }))?
    );
    Ok(())
}

pub async fn target(args: &CaptureArgs) -> Result<(Target, Cdp)> {
    let remembered = (args.reuse && args.target.is_none())
        .then(last_open_session)
        .transpose()?;
    let endpoint = remembered
        .as_ref()
        .map_or(args.cdp_url.as_str(), |session| session.cdp_url.as_str());
    ensure_endpoint(endpoint).await?;
    let target = if args.reuse {
        let id = remembered
            .as_ref()
            .map(|session| session.target.as_str())
            .or(args.target.as_deref())
            .context("run `recreate open <url>` before `recreate capture --reuse`")?;
        list(endpoint)
            .await?
            .into_iter()
            .find(|target| target.id == id)
            .with_context(|| format!("remembered browser tab is no longer open: {id}"))?
    } else {
        let url = args.url.as_deref().context("capture requires a URL")?;
        create(endpoint, url).await?
    };
    if args.reuse {
        activate(endpoint, &target.id).await?;
    }
    let cdp = Cdp::connect(&target.websocket_url).await?;
    Ok((target, cdp))
}

pub(crate) fn last_open_session() -> Result<OpenSession> {
    let path = open_session_path()?;
    let bytes = fs::read(&path).with_context(|| {
        format!(
            "no remembered browser tab; run `recreate open <url>` first ({})",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).context("remembered browser tab is invalid")
}

fn save_open_session(session: &OpenSession) -> Result<()> {
    let path = open_session_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec(session)?)?;
    remember_open_tab(session)
}

/// Records every tab opened by `recreate open` so several prepared pages can be
/// compared against each other without one overwriting another.
fn remember_open_tab(session: &OpenSession) -> Result<()> {
    let path = open_tabs_path()?;
    let mut tabs: Vec<OpenSession> = fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    tabs.retain(|tab| {
        !same_url(&tab.url, &session.url)
            && !(tab.cdp_url == session.cdp_url && tab.target == session.target)
    });
    tabs.push(session.clone());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec(&tabs)?)?;
    Ok(())
}

pub(crate) fn find_open_session(url: &str) -> Option<OpenSession> {
    let tabs: Vec<OpenSession> = fs::read(open_tabs_path().ok()?)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    tabs.into_iter()
        .rev()
        .find(|tab| same_url(&tab.url, url) || same_url(&tab.rendered_url, url))
}

fn same_url(left: &str, right: &str) -> bool {
    left.trim_end_matches('/') == right.trim_end_matches('/')
}

fn open_tabs_path() -> Result<PathBuf> {
    Ok(dirs_home()?.join(".recreate").join("open-tabs.json"))
}

fn open_session_path() -> Result<PathBuf> {
    Ok(dirs_home()?.join(".recreate").join("last-open.json"))
}

pub async fn list(endpoint: &str) -> Result<Vec<Target>> {
    recreate_browser::list(endpoint).await
}

#[cfg(test)]
pub async fn close(endpoint: &str, id: &str) -> Result<()> {
    recreate_browser::close(endpoint, id).await
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
