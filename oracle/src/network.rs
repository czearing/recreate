use crate::{cdp::Cdp, digest};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Fixture {
    responses: BTreeMap<String, Response>,
    misses: std::collections::BTreeSet<String>,
}

impl Fixture {
    pub fn is_empty(&self) -> bool {
        self.responses.is_empty()
    }
}

struct Response {
    status: u16,
    headers: Vec<Value>,
    body: String,
}

pub async fn capture_fixture(cdp: &mut Cdp) -> anyhow::Result<Fixture> {
    let mut fixture = Fixture::default();
    update_fixture(cdp, &mut fixture).await?;
    Ok(fixture)
}

pub async fn update_fixture(cdp: &mut Cdp, fixture: &mut Fixture) -> anyhow::Result<()> {
    let requests = cdp.take_events_named("Network.requestWillBeSent");
    let responses = cdp.take_events_named("Network.responseReceived");
    let request_by_id = requests
        .iter()
        .filter_map(|event| {
            Some((
                event["params"]["requestId"].as_str()?.to_owned(),
                request_key(&event["params"]["request"]),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    for event in &responses {
        let kind = event["params"]["type"].as_str().unwrap_or_default();
        let mime = event["params"]["response"]["mimeType"]
            .as_str()
            .unwrap_or_default();
        if !matches!(kind, "Fetch" | "XHR" | "Other") && !mime.contains("json") {
            continue;
        }
        let request_id = event["params"]["requestId"].as_str().unwrap_or_default();
        let Some(key) = request_by_id.get(request_id) else {
            continue;
        };
        if fixture.responses.contains_key(key) {
            continue;
        }
        let body = cdp
            .send(
                "Network.getResponseBody",
                serde_json::json!({"requestId": request_id}),
            )
            .await;
        let body = match body {
            Ok(body) => body,
            Err(error) if unavailable_body(&error) => continue,
            Err(error) => return Err(error),
        };
        let bytes = decode_body(&body)?;
        let headers = event["params"]["response"]["headers"]
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(name, _)| replay_header(name))
            .map(|(name, value)| {
                serde_json::json!({"name":name,"value":value.as_str().unwrap_or_default()})
            })
            .collect();
        fixture.responses.insert(
            key.clone(),
            Response {
                status: event["params"]["response"]["status"]
                    .as_u64()
                    .unwrap_or(200) as u16,
                headers,
                body: STANDARD.encode(bytes),
            },
        );
    }
    cdp.put_events_named("Network.requestWillBeSent", requests);
    cdp.put_events_named("Network.responseReceived", responses);
    Ok(())
}

pub async fn enable_fixture(cdp: &mut Cdp) -> anyhow::Result<()> {
    cdp.send(
        "Fetch.enable",
        serde_json::json!({"patterns":[
            {"resourceType":"XHR"},
            {"resourceType":"Fetch"},
            {"resourceType":"Other"},
            {"resourceType":"XHR","requestStage":"Response"},
            {"resourceType":"Fetch","requestStage":"Response"},
            {"resourceType":"Other","requestStage":"Response"}
        ]}),
    )
    .await?;
    Ok(())
}

pub async fn enable_learning(cdp: &mut Cdp) -> anyhow::Result<()> {
    cdp.send(
        "Fetch.enable",
        serde_json::json!({"patterns":[
            {"resourceType":"XHR","requestStage":"Response"},
            {"resourceType":"Fetch","requestStage":"Response"},
            {"resourceType":"Other","requestStage":"Response"}
        ]}),
    )
    .await?;
    Ok(())
}

pub async fn fulfill_pending(cdp: &mut Cdp, fixture: &mut Fixture) -> anyhow::Result<()> {
    loop {
        let paused = cdp.take_events_named("Fetch.requestPaused");
        if paused.is_empty() {
            return Ok(());
        }
        for event in paused {
            let request_id = &event["params"]["requestId"];
            let key = request_key(&event["params"]["request"]);
            if !event["params"]["responseStatusCode"].is_null() {
                learn_response(cdp, fixture, &event, &key).await?;
                continue;
            }
            let result = if let Some(response) = fixture.responses.get(&key) {
                cdp.send(
                    "Fetch.fulfillRequest",
                    serde_json::json!({
                        "requestId":request_id,
                        "responseCode":response.status,
                        "responseHeaders":response.headers,
                        "body":response.body
                    }),
                )
                .await
            } else {
                let miss = format!(
                    "{} {}",
                    event["params"]["request"]["method"]
                        .as_str()
                        .unwrap_or_default(),
                    event["params"]["request"]["url"]
                        .as_str()
                        .unwrap_or_default()
                );
                if std::env::var_os("RECREATE_TIMING").is_some()
                    && fixture.misses.insert(miss.clone())
                {
                    eprintln!("oracle_fixture_miss={miss}");
                }
                cdp.send(
                    "Fetch.continueRequest",
                    serde_json::json!({"requestId":request_id}),
                )
                .await
            };
            if let Err(error) = result
                && !error.to_string().contains("Invalid InterceptionId")
            {
                return Err(error);
            }
        }

        async fn learn_response(
            cdp: &mut Cdp,
            fixture: &mut Fixture,
            event: &Value,
            key: &str,
        ) -> anyhow::Result<()> {
            if !fixture.responses.contains_key(key)
                && let Ok(body) = cdp
                    .send(
                        "Fetch.getResponseBody",
                        serde_json::json!({"requestId":event["params"]["requestId"]}),
                    )
                    .await
            {
                fixture.responses.insert(
                    key.into(),
                    Response {
                        status: event["params"]["responseStatusCode"]
                            .as_u64()
                            .unwrap_or(200) as u16,
                        headers: event["params"]["responseHeaders"]
                            .as_array()
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|header| header["name"].as_str().is_none_or(replay_header))
                            .collect(),
                        body: if body["base64Encoded"] == true {
                            body["body"].as_str().unwrap_or_default().into()
                        } else {
                            STANDARD.encode(body["body"].as_str().unwrap_or_default().as_bytes())
                        },
                    },
                );
            }
            let result = cdp
                .send(
                    "Fetch.continueResponse",
                    serde_json::json!({"requestId":event["params"]["requestId"]}),
                )
                .await;
            if let Err(error) = result
                && !error.to_string().contains("Invalid InterceptionId")
            {
                return Err(error);
            }
            Ok(())
        }
    }
}

fn request_key(request: &Value) -> String {
    format!(
        "{}\n{}\n{}",
        request["method"].as_str().unwrap_or("GET"),
        request["url"].as_str().unwrap_or_default(),
        request["postData"].as_str().unwrap_or_default()
    )
}

fn replay_header(name: &str) -> bool {
    !matches!(
        name.to_ascii_lowercase().as_str(),
        "content-encoding" | "content-length" | "transfer-encoding"
    )
}

pub async fn manifest(cdp: &mut Cdp) -> anyhow::Result<Value> {
    let mut entries = Vec::new();
    let events = cdp.take_events_named("Network.responseReceived");
    if events.is_empty() {
        return Ok(Value::Array(entries));
    }
    let frame_tree = cdp.send("Page.getFrameTree", serde_json::json!({})).await?;
    let loader_id = &frame_tree["frameTree"]["frame"]["loaderId"];
    for event in events {
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

    Ok(Value::Array(entries))
}

fn unavailable_body(error: &anyhow::Error) -> bool {
    let message = error.to_string();
    message.contains("No data found for resource")
        || message.contains("No resource with given identifier")
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

#[cfg(test)]
mod tests {
    use super::replay_header;

    #[test]
    fn decoded_fixture_bodies_drop_transport_headers() {
        assert!(!replay_header("Content-Encoding"));
        assert!(!replay_header("content-length"));
        assert!(replay_header("content-type"));
    }
}
