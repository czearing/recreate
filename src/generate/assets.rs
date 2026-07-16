use crate::model::{BrowserCookie, Specification};
use anyhow::Result;
use base64::Engine;
use reqwest::header::COOKIE;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
use url::Url;

pub async fn download(
    specification: &Specification,
    root: &Path,
    cookies: &[BrowserCookie],
) -> Result<BTreeMap<String, String>> {
    let directory = root.join("public").join("assets");
    fs::create_dir_all(&directory)?;
    let client = reqwest::Client::new();
    let mut map = BTreeMap::new();
    for (url, data) in states(specification).flat_map(|state| &state.asset_data) {
        if map.contains_key(url) {
            continue;
        }
        if let Some((metadata, encoded)) = data.split_once(',') {
            let bytes = base64::engine::general_purpose::STANDARD.decode(encoded)?;
            let hash = hex::encode(Sha256::digest(&bytes));
            let extension = data_extension(metadata);
            let filename = format!("{}.{}", &hash[..20], extension);
            fs::write(directory.join(&filename), bytes)?;
            map.insert(url.clone(), format!("/assets/{filename}"));
        }
    }
    for url in states(specification).flat_map(|state| &state.asset_urls) {
        if map.contains_key(url) || url.starts_with("data:") || url.starts_with("blob:") {
            continue;
        }

        let Ok(parsed) = Url::parse(url) else {
            continue;
        };
        let cookie = cookie_header(&parsed, cookies);
        let mut request = client.get(url);
        if !cookie.is_empty() {
            request = request.header(COOKIE, cookie);
        }
        let Ok(response) = request
            .send()
            .await
            .and_then(|value| value.error_for_status())
        else {
            continue;
        };
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let bytes = response.bytes().await?;
        if !usable_asset(&parsed, &content_type, &bytes) {
            continue;
        }
        let hash = hex::encode(Sha256::digest(&bytes));
        let extension = extension(&parsed, &content_type);
        let filename = format!("{}.{}", &hash[..20], extension);
        fs::write(directory.join(&filename), bytes)?;
        map.insert(url.clone(), format!("/assets/{filename}"));
    }

    Ok(map)
}

fn usable_asset(url: &Url, content_type: &str, bytes: &[u8]) -> bool {
    let content_type = content_type.split(';').next().unwrap_or_default();
    if matches!(content_type, "text/html" | "application/xhtml+xml") {
        return false;
    }
    if url.path().ends_with(".svg") {
        return content_type == "image/svg+xml"
            || std::str::from_utf8(bytes)
                .is_ok_and(|value| value.trim_start().starts_with("<svg"));
    }
    true
}

fn states(specification: &Specification) -> impl Iterator<Item = &crate::model::PageState> {
    specification.states.iter().chain(
        specification
            .interactions
            .iter()
            .flat_map(|interaction| interaction.states.iter()),
    )
}

fn data_extension(metadata: &str) -> &'static str {
    if metadata.contains("image/png") {
        "png"
    } else if metadata.contains("image/jpeg") {
        "jpg"
    } else if metadata.contains("image/webp") {
        "webp"
    } else if metadata.contains("image/svg+xml") {
        "svg"
    } else {
        "bin"
    }
}

fn cookie_header(url: &Url, cookies: &[BrowserCookie]) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes_cookies_to_matching_domains() {
        let url = Url::parse("https://app.example.com/image.png").unwrap();
        let cookies = vec![BrowserCookie {
            name: "session".into(),
            value: "value".into(),
            domain: ".example.com".into(),
            path: "/".into(),
            secure: true,
        }];
        assert_eq!(cookie_header(&url, &cookies), "session=value");
    }

    #[test]
    fn includes_assets_introduced_by_interactions() {
        let state = serde_json::json!({
            "url":"https://example.test",
            "title":"Fixture",
            "viewport":{"width":800,"height":600},
            "nodes":[],
            "animations":[],
            "state_styles":[],
            "css_rules":[],
            "asset_urls":["https://example.test/dialog.svg"],
            "asset_data":{}
        });
        let specification: Specification = serde_json::from_value(serde_json::json!({
            "schema_version":1,
            "requested_url":"https://example.test",
            "captured_url":"https://example.test",
            "states":[state.clone()],
            "interactions":[{
                "trigger_path":"html>body>button",
                "trigger_tag":"button",
                "trigger_label":"Open",
                "states":[state]
            }]
        }))
        .unwrap();
        assert_eq!(states(&specification).count(), 2);
    }

    #[test]
    fn rejects_login_html_disguised_as_svg() {
        let url = Url::parse("https://example.test/image.svg").unwrap();
        assert!(!usable_asset(&url, "text/html; charset=utf-8", b"<html>"));
        assert!(usable_asset(
            &url,
            "image/svg+xml",
            br#"<svg xmlns="http://www.w3.org/2000/svg"/>"#
        ));
    }
}
