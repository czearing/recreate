use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Target {
    pub id: String,
    #[serde(default)]
    pub url: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub websocket_url: String,
    #[serde(default, rename = "type")]
    pub kind: String,
}

pub async fn list(endpoint: &str) -> anyhow::Result<Vec<Target>> {
    Ok(reqwest::get(format!("{endpoint}/json/list"))
        .await?
        .error_for_status()?
        .json()
        .await?)
}

pub async fn find_target(endpoint: &str, requested: Option<&str>) -> anyhow::Result<Target> {
    list(endpoint)
        .await?
        .into_iter()
        .find(|target| {
            requested.map_or(target.kind == "page" || target.kind.is_empty(), |id| {
                target.id == id
            })
        })
        .ok_or_else(|| anyhow::anyhow!("browser target was not found"))
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

pub async fn activate(endpoint: &str, id: &str) -> anyhow::Result<()> {
    reqwest::get(format!("{endpoint}/json/activate/{id}"))
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn close(endpoint: &str, id: &str) -> anyhow::Result<()> {
    reqwest::get(format!("{endpoint}/json/close/{id}"))
        .await?
        .error_for_status()?;
    Ok(())
}
