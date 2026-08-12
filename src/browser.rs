use crate::{
    cdp::Cdp,
    cli::{CaptureArgs, OpenArgs},
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

pub use recreate_browser::Target;

/// Which browser tab an operation must reclaim when it finishes.
///
/// The browser outlives this process, so a tab is not freed when the run ends:
/// it stays open until someone closes it. Every call site therefore has to
/// answer "is this tab mine to close?", and the answer is the same one every
/// time — a tab we created is ours, a tab the operator prepared with
/// `recreate open` and named through `--reuse` is not. Answering it here, once,
/// is what stops the answer being forgotten at the next call site.
pub struct Tab {
    endpoint: String,
    id: String,
    owned: bool,
}

impl Tab {
    pub fn new(endpoint: &str, id: &str, owned: bool) -> Self {
        Self {
            endpoint: endpoint.into(),
            id: id.into(),
            owned,
        }
    }

    /// The tab that closing will reclaim, if any. Keeping the decision separate
    /// from the act of closing lets the decision be proven without a browser.
    pub fn expiring(&self) -> Option<&str> {
        self.owned.then_some(self.id.as_str())
    }

    pub async fn close(self) -> Result<()> {
        match self.expiring() {
            Some(id) => recreate_browser::close(&self.endpoint, id).await,
            None => Ok(()),
        }
    }
}

/// A browser tab held for one operation, together with its protocol connection.
pub struct Session {
    pub target: Target,
    pub cdp: Cdp,
    tab: Tab,
}

impl Session {
    pub async fn close(self) -> Result<()> {
        self.tab.close().await
    }
}

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

pub async fn target(args: &CaptureArgs) -> Result<Session> {
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
    let cdp = Cdp::connect(
        &target.websocket_url,
        crate::capture_settle::TRANSPORT_DEADLINE,
    )
    .await?;
    let tab = Tab::new(endpoint, &target.id, !args.reuse);
    Ok(Session { target, cdp, tab })
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
    tabs.retain(|tab| !replaces(session, tab));
    tabs.push(session.clone());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec(&tabs)?)?;
    Ok(())
}

/// A newly opened tab supersedes an earlier one that shows the same page or
/// occupies the same browser target.
fn replaces(session: &OpenSession, tab: &OpenSession) -> bool {
    same_url(&tab.url, &session.url)
        || (tab.cdp_url == session.cdp_url && tab.target == session.target)
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

/// The command line the capture browser is launched with.
///
/// A capture must observe the page, and only the page. A browser carries software of its own
/// into every document it opens: extensions run content scripts in the page's world, so their
/// edits are recorded as the page's markup and their poll loops are recorded as the page's
/// outstanding work. Neither belongs to the site under test, and neither is reproducible on
/// another machine, so both are pure contamination of the recreation.
///
/// A private profile is not enough. An enterprise-managed browser force-installs extensions
/// by policy into every profile including a brand new one, so the isolation the profile
/// appears to give is not real on exactly the machines an agent tool runs on. Measured here,
/// a managed remote-desktop extension added `data-rdwebrtc-ext-url` to the emitted markup of
/// a scene that authored no attributes at all, and its poll loop held the recorder waiting
/// for 5.5s of every capture.
///
/// Nothing distinguishes an extension's edit from the page's own once it has been captured,
/// because both are written through the same interface. So the exclusion has to happen at
/// the boundary, before the first document, rather than as a filter downstream.
pub fn launch_args(port: u16, profile: &Path) -> Vec<String> {
    vec![
        format!("--remote-debugging-port={port}"),
        format!("--user-data-dir={}", profile.display()),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        "--disable-extensions".into(),
        "--disable-component-extensions-with-background-pages".into(),
        // A development server usually serves an untrusted certificate. Rendering the
        // browser's own privacy interstitial instead of the page under test compares
        // nothing, so trust what the operator asked us to open.
        "--ignore-certificate-errors".into(),
        "--force-device-scale-factor=1".into(),
        "--new-window".into(),
        "about:blank".into(),
    ]
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
        .args(launch_args(port, &profile))
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
