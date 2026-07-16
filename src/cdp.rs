use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub struct Cdp {
    socket: Socket,
    next_id: u64,
    command_timeout: std::time::Duration,
    #[cfg(test)]
    events: Vec<Value>,
}

impl Cdp {
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with_timeout(url, std::time::Duration::from_secs(30)).await
    }

    async fn connect_with_timeout(url: &str, command_timeout: std::time::Duration) -> Result<Self> {
        let (socket, _) = connect_async(url)
            .await
            .with_context(|| format!("connect CDP websocket {url}"))?;
        Ok(Self {
            socket,
            next_id: 0,
            command_timeout,
            #[cfg(test)]
            events: Vec::new(),
        })
    }

    pub async fn send(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let request = json!({ "id": id, "method": method, "params": params });
        self.socket
            .send(Message::Text(request.to_string().into()))
            .await?;
        let deadline = tokio::time::Instant::now() + self.command_timeout;
        loop {
            let message = tokio::time::timeout_at(deadline, self.socket.next())
                .await
                .with_context(|| format!("CDP command timed out: {method}"))?
                .context("CDP socket closed")??;
            let Message::Text(text) = message else {
                continue;
            };
            let value: Value = serde_json::from_str(&text)?;
            if value.get("id").and_then(Value::as_u64) != Some(id) {
                #[cfg(test)]
                if value.get("method").is_some() {
                    self.events.push(value);
                }
                continue;
            }
            if let Some(error) = value.get("error") {
                bail!("CDP {method} failed: {error}");
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    pub async fn enable(&mut self, domains: &[&str]) -> Result<()> {
        for domain in domains {
            self.send(&format!("{domain}.enable"), json!({})).await?;
        }
        Ok(())
    }

    pub async fn evaluate(&mut self, expression: &str) -> Result<Value> {
        let result = self
            .send(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true
                }),
            )
            .await?;
        if let Some(exception) = result.get("exceptionDetails") {
            bail!("browser evaluation failed: {exception}");
        }
        Ok(result["result"]["value"].clone())
    }

    #[cfg(test)]
    pub fn take_events(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio_tungstenite::{accept_async, tungstenite::Message};

    #[test]
    fn cdp_request_shape_is_stable() {
        let value = serde_json::json!({
            "id": 1,
            "method": "Page.enable",
            "params": {}
        });
        assert_eq!(value["method"], "Page.enable");
    }

    #[tokio::test]
    async fn recovers_after_timeout_and_keeps_interleaved_events() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let first = request_id(socket.next().await.unwrap().unwrap());
            let second = request_id(socket.next().await.unwrap().unwrap());
            socket
                .send(Message::Text(
                    serde_json::json!({"method":"Runtime.consoleAPICalled"})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
            socket
                .send(Message::Text(
                    serde_json::json!({"id":first,"result":{"stale":true}})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
            socket
                .send(Message::Text(
                    serde_json::json!({"id":second,"result":{"ok":true}})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
        });
        let url = format!("ws://{address}");
        let mut cdp = Cdp::connect_with_timeout(&url, std::time::Duration::from_millis(10))
            .await
            .unwrap();
        assert!(cdp.send("Page.reload", json!({})).await.is_err());
        let result = cdp.send("Runtime.enable", json!({})).await.unwrap();
        assert_eq!(result["ok"], true);
        assert_eq!(cdp.take_events().len(), 1);
    }

    fn request_id(message: Message) -> u64 {
        let Message::Text(text) = message else {
            panic!("expected text request");
        };
        serde_json::from_str::<Value>(&text).unwrap()["id"]
            .as_u64()
            .unwrap()
    }
}
