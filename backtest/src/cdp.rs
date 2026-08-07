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
        let id = self.send(method, params, deadline).await?;
        self.receive(id, method, deadline).await
    }

    async fn send(
        &mut self,
        method: &str,
        params: Value,
        deadline: Deadline,
    ) -> anyhow::Result<u64> {
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
        Ok(id)
    }

    async fn receive(
        &mut self,
        id: u64,
        method: &str,
        deadline: Deadline,
    ) -> anyhow::Result<Value> {
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

    /// Applies a command and reads an expression in one round trip.
    ///
    /// Commands on one session are executed in the order they are sent, so the
    /// expression still observes the command's effect; waiting for the command
    /// to acknowledge first would only double the number of serialized network
    /// waits, which is the entire cost of sampling many widths.
    pub async fn call_then_evaluate(
        &mut self,
        method: &str,
        params: Value,
        expression: &str,
        deadline: Deadline,
    ) -> anyhow::Result<Value> {
        let command = self.send(method, params, deadline).await?;
        let evaluation = self
            .send(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true
                }),
                deadline,
            )
            .await?;
        self.receive(command, method, deadline).await?;
        let result = self
            .receive(evaluation, "Runtime.evaluate", deadline)
            .await?;
        if let Some(details) = result.get("exceptionDetails") {
            anyhow::bail!("JavaScript evaluation failed: {details}");
        }
        Ok(result
            .pointer("/result/value")
            .cloned()
            .unwrap_or(Value::Null))
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
