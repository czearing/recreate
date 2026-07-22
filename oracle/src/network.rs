use crate::{cdp::Cdp, digest};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;

pub async fn manifest(cdp: &mut Cdp) -> anyhow::Result<Value> {
    let mut entries = Vec::new();
    let frame_tree = cdp.send("Page.getFrameTree", serde_json::json!({})).await?;
    let loader_id = &frame_tree["frameTree"]["frame"]["loaderId"];
    for event in cdp.take_events_named("Network.responseReceived") {
        let kind = event["params"]["type"].as_str().unwrap_or_default();
        if !matches!(kind, "Fetch" | "XHR") || event["params"]["loaderId"] != *loader_id {
            continue;
        }
        let request_id = event["params"]["requestId"].clone();
        let body = cdp
            .send(
                "Network.getResponseBody",
                serde_json::json!({"requestId": request_id}),
            )
            .await;
        let mut entry = serde_json::json!({
            "method": "GET",
            "path": normalized_path(
                event["params"]["response"]["url"].as_str().unwrap_or_default()
            ),
            "status": event["params"]["response"]["status"],
            "mime": event["params"]["response"]["mimeType"],
            "from_disk_cache": event["params"]["response"]["fromDiskCache"],
            "from_service_worker": event["params"]["response"]["fromServiceWorker"]
        });
        match body {
            Ok(body) => {
                entry["body_sha256"] = digest::bytes(&decode_body(&body)?).into();
            }
            Err(error) if unavailable_body(&error) => {
                entry["body_unavailable"] = true.into();
            }
            Err(error) => return Err(error),
        }
        entries.push(entry);
    }

    fn unavailable_body(error: &anyhow::Error) -> bool {
        let message = error.to_string();
        message.contains("No data found for resource")
            || message.contains("No resource with given identifier")
    }
    Ok(Value::Array(entries))
}

fn decode_body(body: &Value) -> anyhow::Result<Vec<u8>> {
    if body["base64Encoded"] == true {
        return Ok(STANDARD.decode(body["body"].as_str().unwrap_or_default())?);
    }
    Ok(body["body"]
        .as_str()
        .unwrap_or_default()
        .as_bytes()
        .to_vec())
}

fn normalized_path(raw: &str) -> String {
    url::Url::parse(raw)
        .map(|url| {
            let mut value = url.path().to_string();
            if let Some(query) = url.query() {
                value.push('?');
                value.push_str(query);
            }
            value
        })
        .unwrap_or_else(|_| raw.to_string())
}
