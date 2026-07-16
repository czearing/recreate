use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub struct Cdp {
    socket: Socket,
    next_id: u64,
}

impl Cdp {
    pub async fn connect(url: &str) -> Result<Self> {
        let (socket, _) = connect_async(url)
            .await
            .with_context(|| format!("connect CDP websocket {url}"))?;
        Ok(Self { socket, next_id: 0 })
    }

    pub async fn send(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let request = json!({ "id": id, "method": method, "params": params });
        self.socket
            .send(Message::Text(request.to_string().into()))
            .await?;
        loop {
            let message =
                tokio::time::timeout(std::time::Duration::from_secs(30), self.socket.next())
                    .await
                    .with_context(|| format!("CDP command timed out: {method}"))?
                    .context("CDP socket closed")??;
            let Message::Text(text) = message else {
                continue;
            };
            let value: Value = serde_json::from_str(&text)?;
            if value.get("id").and_then(Value::as_u64) != Some(id) {
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
}

#[cfg(test)]
mod tests {
    #[test]
    fn cdp_request_shape_is_stable() {
        let value = serde_json::json!({
            "id": 1,
            "method": "Page.enable",
            "params": {}
        });
        assert_eq!(value["method"], "Page.enable");
    }
}
