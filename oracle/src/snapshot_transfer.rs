use crate::{cdp::Cdp, probe};

const CHUNK_BYTES: usize = 256 * 1024;

pub async fn capture(cdp: &mut Cdp) -> anyhow::Result<serde_json::Value> {
    let length =
        cdp.evaluate(&format!(
            "globalThis.__recreateOracleSnapshot=JSON.stringify({});\
             globalThis.__recreateOracleSnapshot.length",
            probe::SNAPSHOT
        ))
        .await?
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("oracle snapshot length is unavailable"))? as usize;
    let mut encoded = String::with_capacity(length);
    for start in (0..length).step_by(CHUNK_BYTES) {
        let end = (start + CHUNK_BYTES).min(length);
        let chunk = cdp
            .evaluate(&format!(
                "globalThis.__recreateOracleSnapshot.slice({start},{end})"
            ))
            .await?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("oracle snapshot chunk is unavailable"))?
            .to_owned();
        encoded.push_str(&chunk);
    }
    let _ = cdp
        .evaluate("delete globalThis.__recreateOracleSnapshot")
        .await;
    anyhow::ensure!(
        encoded.len() == length,
        "oracle snapshot transfer was truncated"
    );
    Ok(serde_json::from_str(&encoded)?)
}
