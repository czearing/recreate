use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::{Message, protocol::WebSocketConfig},
};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct Cdp {
    socket: Socket,
    next_id: u64,
    events: BTreeMap<String, Vec<Value>>,
    timeout: std::time::Duration,
}

impl Cdp {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        Self::connect_with_timeout(url, std::time::Duration::from_secs(30)).await
    }

    async fn connect_with_timeout(url: &str, timeout: std::time::Duration) -> anyhow::Result<Self> {
        let config = WebSocketConfig::default()
            .max_message_size(Some(64 * 1024 * 1024))
            .max_frame_size(Some(64 * 1024 * 1024));
        let (socket, _) = connect_async_with_config(url, Some(config), false)
            .await
            .with_context(|| format!("connect CDP websocket {url}"))?;
        Ok(Self {
            socket,
            next_id: 0,
            events: BTreeMap::new(),
            timeout,
        })
    }

    pub async fn send(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        self.socket
            .send(Message::Text(
                json!({"id": id, "method": method, "params": params})
                    .to_string()
                    .into(),
            ))
            .await?;
        loop {
            let message = tokio::time::timeout(self.timeout, self.socket.next())
                .await
                .with_context(|| format!("CDP command timed out: {method}"))?
                .context("CDP disconnected")??;
            let Message::Text(text) = message else {
                continue;
            };
            let value: Value = serde_json::from_str(&text)?;
            if value["id"].as_u64() == Some(id) {
                if !value["error"].is_null() {
                    anyhow::bail!("{method}: {}", value["error"]);
                }
                return Ok(value["result"].clone());
            }
            self.store_event(value);
        }
    }

    pub async fn enable(&mut self, domains: &[&str]) -> anyhow::Result<()> {
        for domain in domains {
            self.send(&format!("{domain}.enable"), json!({})).await?;
        }
        Ok(())
    }

    pub async fn evaluate(&mut self, expression: &str) -> anyhow::Result<Value> {
        let result = self
            .send(
                "Runtime.evaluate",
                json!({"expression":expression,"returnByValue":true,"awaitPromise":true}),
            )
            .await?;
        anyhow::ensure!(
            result["exceptionDetails"].is_null(),
            "browser evaluation failed: {}",
            result["exceptionDetails"]
        );
        Ok(result["result"]["value"].clone())
    }

    pub fn take_events(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.events)
            .into_values()
            .flatten()
            .collect()
    }

    pub fn take_events_named(&mut self, name: &str) -> Vec<Value> {
        self.events.remove(name).unwrap_or_default()
    }

    pub fn error_count(&mut self) -> usize {
        let exceptions = self.take_events_named("Runtime.exceptionThrown").len();
        let console = self
            .take_events_named("Runtime.consoleAPICalled")
            .iter()
            .filter(|event| event["params"]["type"] == "error")
            .count();
        let network = self
            .take_events_named("Network.loadingFailed")
            .iter()
            .filter(|event| event["params"]["canceled"] != true)
            .count();
        exceptions + console + network
    }

    pub async fn wait_event(&mut self, name: &str) -> anyhow::Result<Value> {
        if let Some(value) = self.events.get_mut(name).and_then(|events| events.pop()) {
            return Ok(value);
        }
        loop {
            let message = tokio::time::timeout(self.timeout, self.socket.next())
                .await
                .with_context(|| format!("CDP event timed out: {name}"))?
                .context("CDP disconnected")??;
            let Message::Text(text) = message else {
                continue;
            };
            let value: Value = serde_json::from_str(&text)?;
            if value["method"] == name {
                return Ok(value);
            }
            self.store_event(value);
        }
    }

    fn store_event(&mut self, value: Value) {
        if let Some(name) = value["method"].as_str() {
            self.events.entry(name.into()).or_default().push(value);
        }
    }
}
