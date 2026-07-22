use anyhow::Context;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn json<T: Serialize>(value: &T) -> anyhow::Result<String> {
    let encoded = serde_json::to_vec(value).context("serialize digest input")?;
    Ok(bytes(&encoded))
}

pub fn bytes(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_stable() {
        assert_eq!(bytes(b"oracle"), bytes(b"oracle"));
        assert_ne!(bytes(b"oracle"), bytes(b"candidate"));
    }
}
