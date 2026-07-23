use crate::{cdp::Cdp, probe};
use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD};

const CHUNK_BYTES: usize = 48 * 1024;

pub async fn capture(cdp: &mut Cdp) -> anyhow::Result<serde_json::Value> {
    let length =
        cdp.evaluate(&format!(
            "globalThis.__recreateOracleSnapshot=new TextEncoder().encode(JSON.stringify({}));\
             globalThis.__recreateOracleSnapshot.byteLength",
            probe::SNAPSHOT
        ))
        .await
        .context("creating byte-exact oracle snapshot")?
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("oracle snapshot length is unavailable"))? as usize;
    let mut encoded = Vec::with_capacity(length);
    for start in (0..length).step_by(CHUNK_BYTES) {
        let end = (start + CHUNK_BYTES).min(length);
        let chunk = cdp
            .evaluate(&format!(
                "btoa(String.fromCharCode.apply(null,\
                 globalThis.__recreateOracleSnapshot.slice({start},{end})))"
            ))
            .await
            .with_context(|| format!("reading oracle snapshot bytes {start}..{end}"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("oracle snapshot chunk is unavailable"))?
            .to_owned();
        encoded.extend(STANDARD.decode(chunk)?);
    }
    let _ = cdp
        .evaluate("delete globalThis.__recreateOracleSnapshot")
        .await;
    anyhow::ensure!(
        encoded.len() == length,
        "oracle snapshot transfer was truncated"
    );
    Ok(serde_json::from_slice(&encoded)?)
}
