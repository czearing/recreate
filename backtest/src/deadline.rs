use anyhow::Context;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub struct Deadline {
    started: Instant,
    limit: Duration,
}

impl Deadline {
    pub fn new(limit_ms: u64) -> Self {
        Self {
            started: Instant::now(),
            limit: Duration::from_millis(limit_ms),
        }
    }

    pub fn elapsed_ms(self) -> u128 {
        self.started.elapsed().as_millis()
    }

    pub fn remaining(self) -> Duration {
        self.limit.saturating_sub(self.started.elapsed())
    }

    pub async fn run<T>(
        self,
        label: &str,
        future: impl std::future::Future<Output = anyhow::Result<T>>,
    ) -> anyhow::Result<T> {
        tokio::time::timeout(self.remaining(), future)
            .await
            .with_context(|| format!("{label} exceeded {}ms", self.limit.as_millis()))?
    }
}

