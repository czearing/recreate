use anyhow::{Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command};

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

fn installed_binary() -> Result<bool> {
    let path = std::env::current_exe()?;
    Ok(path
        .parent()
        .and_then(|parent| parent.parent())
        .and_then(|parent| parent.file_name())
        .is_some_and(|name| name == ".recreate"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_platform_has_an_asset_name() {
        assert_ne!(asset_name(), "recreate-unsupported");
    }

    #[test]
    fn hashes_release_assets() {
        assert_eq!(
            sha256(b"recreate"),
            "sha256:9efa66815ecaa75d90584029681ca68eae876b0f76ef2d226d3616f130145061"
        );
    }
}
