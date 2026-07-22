use crate::qualification_server::Server;
use serde::Deserialize;
use std::{fs, path::Path};

pub struct Case {
    pub url: String,
    pub expected_domain: String,
    pub hidden: bool,
}

#[derive(Deserialize)]
struct MutationSpec {
    file: String,
    expected_domain: String,
}

pub fn load(
    public: &Path,
    holdouts: Option<&Path>,
    server: &Server,
    network_urls: (&str, &str),
) -> anyhow::Result<Vec<Case>> {
    let mut cases = load_root(public, false, server, network_urls)?;
    if let Some(root) = holdouts {
        cases.extend(load_root(root, true, server, network_urls)?);
    }
    anyhow::ensure!(!cases.is_empty(), "qualification requires mutation cases");
    Ok(cases)
}

fn load_root(
    root: &Path,
    hidden: bool,
    server: &Server,
    network_urls: (&str, &str),
) -> anyhow::Result<Vec<Case>> {
    let specs: Vec<MutationSpec> = serde_json::from_slice(&fs::read(root.join("mutations.json"))?)?;
    specs
        .into_iter()
        .map(|spec| {
            let html = String::from_utf8(fs::read(root.join("mutants").join(spec.file))?)?;
            Ok(Case {
                url: server.page(rewrite_network(&html, network_urls)),
                expected_domain: spec.expected_domain,
                hidden,
            })
        })
        .collect()
}

pub fn rewrite_network(html: &str, network_urls: (&str, &str)) -> String {
    html.replace("<script>", "<script type=\"module\">")
        .replace(
            "data:application/json,%7B%22ok%22%3Atrue%7D",
            network_urls.0,
        )
        .replace(
            "data:application/json,%7B%22ok%22%3Afalse%7D",
            network_urls.1,
        )
        .replace("fetch(\"http://", "await fetch(\"http://")
}

pub fn page_url(
    server: &Server,
    path: &Path,
    network_urls: (&str, &str),
) -> anyhow::Result<String> {
    let html = String::from_utf8(fs::read(path)?)?;
    Ok(server.page(rewrite_network(&html, network_urls)))
}
