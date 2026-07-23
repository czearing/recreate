use crate::{browser_target, cdp::Cdp, digest, model::Environment, process_job::ProcessJob};
use anyhow::Context;
use serde_json::json;
use std::{net::TcpListener, path::PathBuf, process::Stdio, time::Duration};
use tokio::{process::Child, time::sleep};

pub struct Browser {
    child: Option<Child>,
    pub cdp: Cdp,
    endpoint: String,
    profile: Option<PathBuf>,
    target_id: String,
    target_owned: bool,
    _job: Option<ProcessJob>,
}

impl Browser {
    pub async fn launch(executable: Option<PathBuf>) -> anyhow::Result<Self> {
        let executable = executable
            .or_else(|| std::env::var_os("RECREATE_BROWSER").map(PathBuf::from))
            .or_else(recreate_browser::find_executable)
            .context("Chromium browser not found")?;
        let port = TcpListener::bind(("127.0.0.1", 0))?.local_addr()?.port();
        let profile = tempfile::tempdir()?.keep();
        let endpoint = format!("http://127.0.0.1:{port}");
        let mut child = tokio::process::Command::new(executable)
            .args([
                "--headless=new",
                "--disable-gpu",
                "--hide-scrollbars",
                "--disable-background-mode",
                "--disable-features=msEdgeStartupBoost",
                "--force-color-profile=srgb",
                "--lang=en-US",
                "--no-first-run",
                "--no-default-browser-check",
                "--noerrdialogs",
                &format!("--remote-debugging-port={port}"),
                &format!("--user-data-dir={}", profile.display()),
                "about:blank",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let job = ProcessJob::attach(child.id().context("browser process id unavailable")?)?;
        wait_ready(&endpoint, &mut child).await?;
        let target = recreate_browser::create(&endpoint, "about:blank").await?;
        let mut cdp = Cdp::connect(&target.websocket_url).await?;
        cdp.set_timeout(Duration::from_secs(120));
        Ok(Self {
            child: Some(child),
            cdp,
            endpoint,
            profile: Some(profile),
            target_id: target.id,
            target_owned: true,
            _job: Some(job),
        })
    }

    pub async fn attach(endpoint: String, target: Option<&str>) -> anyhow::Result<Self> {
        let target = recreate_browser::find_target(&endpoint, target).await?;
        let mut cdp = Cdp::connect(&target.websocket_url).await?;
        cdp.set_timeout(Duration::from_secs(120));
        Ok(Self {
            child: None,
            cdp,
            endpoint,
            profile: None,
            target_id: target.id,
            target_owned: false,
            _job: None,
        })
    }

    pub async fn environment(&mut self) -> anyhow::Result<Environment> {
        let version = self.cdp.send("Browser.getVersion", json!({})).await?;
        Ok(Environment {
            schema: 1,
            browser_product: version["product"].as_str().unwrap_or_default().into(),
            browser_revision: version["revision"].as_str().unwrap_or_default().into(),
            protocol_version: version["protocolVersion"]
                .as_str()
                .unwrap_or_default()
                .into(),
            command_line_digest: digest::bytes(
                b"headless=new;disable-gpu;hide-scrollbars;disable-background-mode;\
disable-features=msEdgeStartupBoost;force-color-profile=srgb;lang=en-US;\
no-first-run;no-default-browser-check;noerrdialogs",
            ),
            operating_system: std::env::consts::OS.into(),
            architecture: std::env::consts::ARCH.into(),
            locale: "en-US".into(),
            timezone: "UTC".into(),
            color_scheme: "light".into(),
            reduced_motion: false,
            device_scale_factor_milli: 1000,
        })
    }

    pub async fn prepare(&mut self) -> anyhow::Result<()> {
        for domain in [
            "Page",
            "Runtime",
            "Network",
            "DOM",
            "Animation",
            "Accessibility",
        ] {
            self.cdp
                .send(&format!("{domain}.enable"), json!({}))
                .await?;
        }
        self.cdp
            .send(
                "Emulation.setTimezoneOverride",
                json!({"timezoneId": "UTC"}),
            )
            .await?;
        self.cdp
            .send("Emulation.setLocaleOverride", json!({"locale": "en-US"}))
            .await?;
        Ok(())
    }

    pub async fn open(&mut self, url: &str) -> anyhow::Result<()> {
        let target = recreate_browser::create(&self.endpoint, url).await?;
        self.cdp = Cdp::connect(&target.websocket_url).await?;
        self.cdp.set_timeout(Duration::from_secs(120));
        let old_target = std::mem::replace(&mut self.target_id, target.id);
        if self.target_owned {
            let _ = recreate_browser::close(&self.endpoint, &old_target).await;
        }
        self.target_owned = true;
        self.prepare().await
    }
    pub async fn open_instrumented(&mut self, url: &str, script: &str) -> anyhow::Result<()> {
        self.open("about:blank").await?;
        self.cdp
            .send(
                "Page.addScriptToEvaluateOnNewDocument",
                json!({"source": script}),
            )
            .await?;
        self.cdp.send("Page.navigate", json!({"url": url})).await?;
        Ok(())
    }
    pub async fn open_or_reuse(&mut self, url: &str) -> anyhow::Result<()> {
        let current = self
            .cdp
            .evaluate("location.href")
            .await
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned));
        if current.as_deref() == Some(url) {
            return self.prepare().await;
        }
        self.open(url).await
    }

    pub async fn target_count(&self) -> anyhow::Result<usize> {
        browser_target::count(&self.endpoint).await
    }

    pub async fn close(mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = self.cdp.send("Browser.close", json!({})).await;
            for _ in 0..20 {
                if child.try_wait().ok().flatten().is_some() {
                    break;
                }
                sleep(Duration::from_millis(25)).await;
            }
            let _ = child.kill().await;
            let _ = child.wait().await;
        } else if self.target_owned {
            let _ = recreate_browser::close(&self.endpoint, &self.target_id).await;
        }
        if let Some(profile) = self.profile {
            let _ = std::fs::remove_dir_all(profile);
        }
    }
}

async fn wait_ready(endpoint: &str, child: &mut Child) -> anyhow::Result<()> {
    for _ in 0..100 {
        if child.try_wait()?.is_some() {
            anyhow::bail!("Chromium exited before CDP became ready");
        }
        if reqwest::get(format!("{endpoint}/json/version"))
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return Ok(());
        }
        sleep(Duration::from_millis(25)).await;
    }
    anyhow::bail!("timed out waiting for Chromium")
}
