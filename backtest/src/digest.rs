use sha2::{Digest, Sha256};

pub fn bytes(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}

pub fn json<T: serde::Serialize>(value: &T) -> anyhow::Result<String> {
    Ok(bytes(&serde_json::to_vec(value)?))
}

