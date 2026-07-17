use crate::model::BrowserCookie;
use anyhow::Result;
use futures_util::{StreamExt, stream};
use reqwest::header::COOKIE;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
use url::Url;

pub async fn download(
    urls: impl Iterator<Item = String>,
    directory: &Path,
    cookies: &[BrowserCookie],
) -> Result<BTreeMap<String, String>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let urls = urls.collect::<BTreeSet<_>>();
    let results = stream::iter(urls.into_iter().map(|url| {
        let client = client.clone();
        async move {
            let parsed = Url::parse(&url).ok()?;
            let cookie = cookie_header(&parsed, cookies);
            let mut request = client.get(&url);
            if !cookie.is_empty() {
                request = request.header(COOKIE, cookie);
            }
            let response = request.send().await.ok()?.error_for_status().ok()?;
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let bytes = response.bytes().await.ok()?;
            usable_asset(&parsed, &content_type, &bytes).then_some((
                url,
                parsed,
                content_type,
                bytes,
            ))
        }
    }))
    .buffer_unordered(32)
    .filter_map(|value| async move { value })
    .collect::<Vec<_>>()
    .await;
    let mut map = BTreeMap::new();
    for (url, parsed, content_type, bytes) in results {
        let hash = hex::encode(Sha256::digest(&bytes));
        let filename = format!("{}.{}", &hash[..20], extension(&parsed, &content_type));
        fs::write(directory.join(&filename), bytes)?;
        map.insert(url, format!("/assets/{filename}"));
    }
    Ok(map)
}

pub(super) fn usable_asset(url: &Url, content_type: &str, bytes: &[u8]) -> bool {
    let content_type = content_type.split(';').next().unwrap_or_default();
    if matches!(content_type, "text/html" | "application/xhtml+xml") {
        return false;
    }
    !url.path().ends_with(".svg")
        || content_type == "image/svg+xml"
        || std::str::from_utf8(bytes).is_ok_and(|value| value.trim_start().starts_with("<svg"))
}

pub(super) fn cookie_header(url: &Url, cookies: &[BrowserCookie]) -> String {
    let host = url.host_str().unwrap_or_default();
    cookies
        .iter()
        .filter(|cookie| {
            (host == cookie.domain.trim_start_matches('.')
                || host.ends_with(&format!(".{}", cookie.domain.trim_start_matches('.'))))
                && url.path().starts_with(&cookie.path)
        })
        .filter(|cookie| !cookie.secure || url.scheme() == "https")
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>()
        .join("; ")
}

fn extension(url: &Url, content_type: &str) -> &'static str {
    match content_type.split(';').next().unwrap_or_default() {
        "image/svg+xml" => "svg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/avif" => "avif",
        "video/mp4" => "mp4",
        "font/woff2" => "woff2",
        "font/woff" => "woff",
        "font/ttf" => "ttf",
        _ if url.path().ends_with(".svg") => "svg",
        _ if url.path().ends_with(".png") => "png",
        _ if url.path().ends_with(".webp") => "webp",
        _ if url.path().ends_with(".woff2") => "woff2",
        _ if url.path().ends_with(".woff") => "woff",
        _ if url.path().ends_with(".ttf") => "ttf",
        _ => "bin",
    }
}
