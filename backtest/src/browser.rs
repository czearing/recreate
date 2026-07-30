use crate::deadline::Deadline;
use anyhow::Context;
use serde::Deserialize;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};

pub struct BrowserProcess {
    child: Option<Child>,
    pub endpoint: String,
    pub executable: PathBuf,
    pub profile: PathBuf,
}

impl BrowserProcess {
    pub fn persist(mut self) {
        self.child.take();
    }
}

impl Drop for BrowserProcess {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub id: String,
    pub web_socket_debugger_url: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Deserialize)]
struct Version {
    #[serde(rename = "Browser")]
    browser: String,
}

pub fn find(explicit: Option<&Path>) -> anyhow::Result<PathBuf> {
    if let Some(path) = explicit {
        anyhow::ensure!(
            path.is_file(),
            "browser executable not found: {}",
            path.display()
        );
        return Ok(path.to_path_buf());
    }
    let mut candidates = Vec::new();
    if let Some(value) = std::env::var_os("PROGRAMFILES") {
        candidates
            .push(PathBuf::from(value.clone()).join("Google\\Chrome\\Application\\chrome.exe"));
        candidates.push(PathBuf::from(value).join("Microsoft\\Edge\\Application\\msedge.exe"));
    }
    if let Some(value) = std::env::var_os("PROGRAMFILES(X86)") {
        candidates
            .push(PathBuf::from(value.clone()).join("Microsoft\\Edge\\Application\\msedge.exe"));
        candidates.push(PathBuf::from(value).join("Google\\Chrome\\Application\\chrome.exe"));
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .context("Chrome or Edge executable was not found")
}

pub async fn launch(
    executable: &Path,
    profile: &Path,
    headful: bool,
) -> anyhow::Result<BrowserProcess> {
    std::fs::create_dir_all(profile)?;
    let profile = profile.canonicalize()?;
    let port = free_port()?;
    let endpoint = format!("http://127.0.0.1:{port}");
    let mut command = Command::new(executable);
    command
        .arg(format!("--remote-debugging-port={port}"))
        .arg("--remote-debugging-address=127.0.0.1")
        .arg("--remote-allow-origins=*")
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-background-networking")
        .arg("--disable-component-update")
        .arg("--disable-default-apps")
        .arg("--disable-extensions")
        .arg("--disable-features=Translate,OptimizationHints,MediaRouter")
        .arg("--disable-popup-blocking")
        .arg("--allow-insecure-localhost")
        .arg("--host-resolver-rules=MAP localhost 127.0.0.1")
        .arg("--hide-scrollbars")
        .arg("--mute-audio")
        .arg("--window-size=1440,900")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if !headful {
        command.arg("--headless=new").arg("--disable-gpu");
    }
    #[cfg(windows)]
    command.creation_flags(0x0000_0008 | 0x0000_0200);
    command.arg("about:blank");
    let child = command
        .spawn()
        .with_context(|| format!("failed to launch {}", executable.display()))?;
    let mut process = BrowserProcess {
        child: Some(child),
        endpoint,
        executable: executable.to_path_buf(),
        profile,
    };
    let deadline = Deadline::new(10_000);
    loop {
        match version(&process.endpoint).await {
            Ok(_) => return Ok(process),
            Err(error) => {
                if let Some(status) = process
                    .child
                    .as_mut()
                    .context("browser process handle is missing")?
                    .try_wait()?
                {
                    anyhow::bail!("browser exited before exposing CDP ({status})");
                }
                if deadline.remaining().is_err() {
                    anyhow::bail!("browser did not expose CDP: {error}");
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}

pub async fn version(endpoint: &str) -> anyhow::Result<String> {
    let value: Version = reqwest::get(format!("{endpoint}/json/version"))
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(value.browser)
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
    let targets: Vec<Target> = reqwest::get(format!("{endpoint}/json/list"))
        .await?
        .error_for_status()?
        .json()
        .await?;
    targets
        .into_iter()
        .find(|target| target.id == id)
        .with_context(|| format!("browser target {id} is unavailable"))
}

fn free_port() -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}
