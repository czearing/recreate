use anyhow::Context;
use std::future::Future;
use std::time::{Duration, Instant};

pub const COMPARISON_DEADLINE_MS: u64 = 4400;
pub const WATCHDOG_MS: u64 = 4800;
const _: () = assert!(WATCHDOG_MS - COMPARISON_DEADLINE_MS >= 250);

#[derive(Clone, Copy, Debug)]
pub struct Deadline {
    expires: Instant,
}

impl Deadline {
    pub fn new(milliseconds: u64) -> Self {
        Self {
            expires: Instant::now() + Duration::from_millis(milliseconds),
        }
    }

    pub fn remaining(self) -> anyhow::Result<Duration> {
        self.expires
            .checked_duration_since(Instant::now())
            .filter(|duration| !duration.is_zero())
            .context("comparison deadline expired")
    }

    pub async fn run<T>(
        self,
        label: &str,
        future: impl Future<Output = anyhow::Result<T>>,
    ) -> anyhow::Result<T> {
        tokio::time::timeout(self.remaining()?, future)
            .await
            .with_context(|| format!("{label} exceeded comparison deadline"))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn internal_deadline_expires_with_report_headroom() {
        let deadline = Deadline::new(5);
        let error = deadline
            .run("slow probe", async {
                std::future::pending::<()>().await;
                Ok(())
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("slow probe"));
    }
}
