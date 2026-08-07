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

fn pid_file(profile: &Path) -> PathBuf {
    profile.with_file_name(format!(
        "{}.pid",
        profile
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("browser")
    ))
}

/// A browser left behind by an interrupted run keeps ownership of its profile,
/// so every later launch hands off to it and exits without ever exposing CDP.
/// Recording the process makes that leak recoverable instead of permanent.
pub fn terminate_recorded(profile: &Path) {
    let file = pid_file(profile);
    let Ok(text) = std::fs::read_to_string(&file) else {
        return;
    };
    if let Ok(pid) = text.trim().parse::<u32>() {
        terminate(pid);
    }
    let _ = std::fs::remove_file(file);
}

#[cfg(windows)]
fn terminate(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(windows))]
fn terminate(pid: u32) {
    let _ = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
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
    if let Some(value) = std::env::var_os(BROWSER_VARIABLE) {
        let path = PathBuf::from(value);
        anyhow::ensure!(
            path.is_file(),
            "{BROWSER_VARIABLE} does not point at an executable: {}",
            path.display()
        );
        return Ok(path);
    }
    candidates()
        .into_iter()
        .find(|path| path.is_file())
        .context("Chrome or Edge executable was not found")
}

const BROWSER_VARIABLE: &str = "RECREATE_BACKTEST_BROWSER";

#[cfg(windows)]
fn candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for variable in ["PROGRAMFILES", "PROGRAMFILES(X86)"] {
        let Some(value) = std::env::var_os(variable) else {
            continue;
        };
        let root = PathBuf::from(value);
        candidates.push(root.join("Google\\Chrome\\Application\\chrome.exe"));
        candidates.push(root.join("Microsoft\\Edge\\Application\\msedge.exe"));
    }
    candidates
}

#[cfg(target_os = "macos")]
fn candidates() -> Vec<PathBuf> {
    [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

#[cfg(all(unix, not(target_os = "macos")))]
fn candidates() -> Vec<PathBuf> {
    [
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/microsoft-edge",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

pub async fn launch(
    executable: &Path,
    profile: &Path,
    headful: bool,
) -> anyhow::Result<BrowserProcess> {
    std::fs::create_dir_all(profile)?;
    terminate_recorded(profile);
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
        .arg("--ignore-certificate-errors")
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
    command.creation_flags(0x0800_0000 | 0x0000_0200);
    command.arg("about:blank");
    let child = command
        .spawn()
        .with_context(|| format!("failed to launch {}", executable.display()))?;
    let _ = std::fs::write(pid_file(&profile), child.id().to_string());
    let mut process = BrowserProcess {
        child: Some(child),
        endpoint,
        executable: executable.to_path_buf(),
        profile,
    };
    let deadline = Deadline::new(10_000);
    let mut launcher_exit = None;
    loop {
        match version(&process.endpoint).await {
            Ok(_) => return Ok(process),
            Err(error) => {
                // A Windows browser launcher hands off to a detached process and
                // exits before CDP is listening, so its exit is only fatal once
                // the deadline has also passed.
                if launcher_exit.is_none()
                    && let Some(status) = process
                        .child
                        .as_mut()
                        .context("browser process handle is missing")?
                        .try_wait()?
                {
                    launcher_exit = Some(status);
                }
                if deadline.remaining().is_err() {
                    match launcher_exit {
                        Some(status) => {
                            anyhow::bail!("browser exited before exposing CDP ({status})")
                        }
                        None => anyhow::bail!("browser did not expose CDP: {error}"),
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_explicit_browser_path_that_is_not_a_file() {
        let error = find(Some(Path::new("does-not-exist"))).unwrap_err();
        assert!(error.to_string().contains("browser executable not found"));
    }

    #[test]
    fn offers_platform_specific_candidates() {
        let candidates = candidates();
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|path| path.is_absolute()));
    }
}
