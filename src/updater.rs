use anyhow::{Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};

const RELEASE_API: &str =
    "https://api.github.com/repos/czearing/recreate/releases/tags/recreate-main";

#[derive(Deserialize)]
struct Release {
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

pub async fn refresh() -> Result<bool> {
    if std::env::var_os("RECREATE_NO_UPDATE").is_some() || !installed_binary()? {
        return Ok(false);
    }
    if recently_checked()? {
        return Ok(false);
    }
    let client = reqwest::Client::new();
    let response = client
        .get(RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "recreate")
        .send()
        .await?;
    mark_checked()?;
    if !response.status().is_success() {
        return Ok(false);
    }
    let release: Release = response.json().await?;
    refresh_backtest(&client, &release).await?;
    let name = asset_name();
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == name)
        .with_context(|| format!("release asset missing: {name}"))?;
    let current = std::env::current_exe()?;
    let current_digest = sha256(&fs::read(&current)?);
    if asset
        .digest
        .as_deref()
        .is_some_and(|digest| digest == current_digest)
    {
        return Ok(false);
    }
    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    if sha256(&bytes) == current_digest {
        return Ok(false);
    }
    let temporary = temporary_path(&current);
    fs::write(&temporary, &bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o755))?;
    }
    self_replace::self_replace(&temporary)?;
    let status = Command::new(&current)
        .args(std::env::args_os().skip(1))
        .env("RECREATE_NO_UPDATE", "1")
        .status()?;
    std::process::exit(status.code().unwrap_or(1));
}

pub async fn ensure_backtest() -> Result<PathBuf> {
    let path = backtest_path()?;
    if path.is_file() {
        return Ok(path);
    }
    let client = reqwest::Client::new();
    let release: Release = client
        .get(RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "recreate")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    install_asset(&client, &release, backtest_asset_name(), &path).await?;
    Ok(path)
}

async fn refresh_backtest(client: &reqwest::Client, release: &Release) -> Result<()> {
    if std::env::var_os(BACKTEST_VARIABLE).is_some() {
        return Ok(());
    }
    let path = backtest_path()?;
    let name = backtest_asset_name();
    let Some(asset) = release.assets.iter().find(|asset| asset.name == name) else {
        return if path.is_file() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("release asset missing: {name}"))
        };
    };
    if path.is_file() {
        let digest = sha256(&fs::read(&path)?);
        if asset
            .digest
            .as_deref()
            .is_some_and(|expected| expected == digest)
        {
            return Ok(());
        }
    }
    install_asset(client, release, name, &path).await
}

async fn install_asset(
    client: &reqwest::Client,
    release: &Release,
    name: &str,
    path: &Path,
) -> Result<()> {
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == name)
        .with_context(|| format!("release asset missing: {name}"))?;
    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let digest = sha256(&bytes);
    if let Some(expected) = &asset.digest {
        anyhow::ensure!(*expected == digest, "release digest mismatch: {name}");
    }
    let temporary = path.with_extension("download");
    fs::write(&temporary, &bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o755))?;
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

fn backtest_path() -> Result<PathBuf> {
    if let Some(value) = std::env::var_os(BACKTEST_VARIABLE) {
        let path = PathBuf::from(value);
        anyhow::ensure!(
            path.is_file(),
            "{BACKTEST_VARIABLE} does not point at an executable: {}",
            path.display()
        );
        return Ok(path);
    }
    let name = if cfg!(windows) {
        "recreate-backtest.exe"
    } else {
        "recreate-backtest"
    };
    Ok(std::env::current_exe()?
        .parent()
        .context("installed binary directory unavailable")?
        .join(name))
}

/// Lets a locally built comparison companion be tested through the normal command.
const BACKTEST_VARIABLE: &str = "RECREATE_BACKTEST_BIN";

fn installed_binary() -> Result<bool> {
    let path = std::env::current_exe()?;
    Ok(path
        .parent()
        .and_then(|parent| parent.parent())
        .and_then(|parent| parent.file_name())
        .is_some_and(|name| name == ".recreate"))
}

fn recently_checked() -> Result<bool> {
    let path = check_path()?;
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(false);
    };
    Ok(SystemTime::now()
        .duration_since(metadata.modified()?)
        .unwrap_or_default()
        < Duration::from_secs(300))
}

fn mark_checked() -> Result<()> {
    let path = check_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, b"checked")?;
    Ok(())
}

fn check_path() -> Result<PathBuf> {
    Ok(std::env::current_exe()?
        .parent()
        .context("installed binary directory unavailable")?
        .join(".update-check"))
}

fn temporary_path(current: &std::path::Path) -> PathBuf {
    current.with_extension("update")
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

fn asset_name() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "recreate-windows-x86_64.exe",
        ("linux", "x86_64") => "recreate-linux-x86_64",
        ("macos", "aarch64") => "recreate-macos-aarch64",
        _ => "recreate-unsupported",
    }
}

fn backtest_asset_name() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "recreate-backtest-windows-x86_64.exe",
        ("linux", "x86_64") => "recreate-backtest-linux-x86_64",
        ("macos", "aarch64") => "recreate-backtest-macos-aarch64",
        _ => "recreate-backtest-unsupported",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_platform_has_an_asset_name() {
        assert_ne!(asset_name(), "recreate-unsupported");
        assert_ne!(backtest_asset_name(), "recreate-backtest-unsupported");
    }

    #[test]
    fn hashes_release_assets() {
        assert_eq!(
            sha256(b"recreate"),
            "sha256:9efa66815ecaa75d90584029681ca68eae876b0f76ef2d226d3616f130145061"
        );
    }
}
