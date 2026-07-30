use crate::deadline::Deadline;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

pub struct Cdp {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_id: u64,
}

impl Cdp {
    pub async fn connect(url: &str, timeout: Duration) -> anyhow::Result<Self> {
        let (socket, _) = tokio::time::timeout(timeout, connect_async(url)).await??;
        Ok(Self { socket, next_id: 1 })
    }

    pub async fn call(
        &mut self,
        method: &str,
        params: Value,
        deadline: Deadline,
    ) -> anyhow::Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({ "id": id, "method": method, "params": params });
        deadline
            .run("CDP send", async {
                self.socket
                    .send(Message::Text(request.to_string().into()))
                    .await?;
                Ok(())
            })
            .await?;
        loop {
            let message = deadline
                .run("CDP response", async {
                    self.socket
                        .next()
                        .await
                        .ok_or_else(|| anyhow::anyhow!("CDP connection closed"))?
                        .map_err(Into::into)
                })
                .await?;
            let Message::Text(text) = message else {
                continue;
            };
            let value: Value = serde_json::from_str(&text)?;
            if value.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = value.get("error") {
                anyhow::bail!("CDP {method} failed: {error}");
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    pub async fn evaluate(
        &mut self,
        expression: &str,
        deadline: Deadline,
    ) -> anyhow::Result<Value> {
        let result = self
            .call(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true
                }),
                deadline,
            )
            .await?;
        if let Some(details) = result.get("exceptionDetails") {
            anyhow::bail!("JavaScript evaluation failed: {details}");
        }
        Ok(result
            .pointer("/result/value")
            .cloned()
            .unwrap_or(Value::Null))
    }
}
