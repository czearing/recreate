use crate::model::{BrowserCookie, Specification};
use anyhow::Result;
use base64::Engine;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

pub async fn download(
    specification: &Specification,
    root: &Path,
    cookies: &[BrowserCookie],
) -> Result<BTreeMap<String, String>> {
    let directory = root.join("public").join("assets");
    fs::create_dir_all(&directory)?;
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
    let remote = states(specification)
        .flat_map(|state| &state.asset_urls)
        .filter(|url| !map.contains_key(*url))
        .filter(|url| !url.starts_with("data:") && !url.starts_with("blob:"))
        .cloned();
    map.extend(super::assets_remote::download(remote, &directory, cookies).await?);
    Ok(map)
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

#[cfg(test)]
mod tests {
    use super::super::assets_remote::{cookie_header, usable_asset};
    use super::*;
    use url::Url;

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
