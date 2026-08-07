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

    /// A sub-deadline carved out of this one, never later than it.
    ///
    /// A stage that must not consume the whole comparison takes a share of what
    /// is left rather than inventing a duration of its own: an independently
    /// chosen timeout composes additively with every other stage, so a chain of
    /// locally reasonable limits still outruns the budget the caller promised.
    pub fn slice(self, share: f64) -> Self {
        let remaining = self.expires.saturating_duration_since(Instant::now());
        Self {
            expires: Instant::now() + remaining.mul_f64(share.clamp(0.0, 1.0)),
        }
    }

    /// Whether another unit of work of the given estimated cost still fits.
    ///
    /// Asking before each unit is what turns a deadline into a bound on elapsed
    /// time; a check made only after the fact reports an overrun it already
    /// paid for.
    pub fn admits(self, estimate: Duration) -> bool {
        self.remaining()
            .is_ok_and(|remaining| remaining > estimate)
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

    /// A stage that must not consume the whole comparison takes a share of what
    /// is left, and can never extend the budget it was carved from.
    #[test]
    fn a_slice_never_outlives_the_deadline_it_came_from() {
        let deadline = Deadline::new(1_000);
        let whole = deadline.remaining().unwrap();
        let part = deadline.slice(0.5).remaining().unwrap();
        assert!(part < whole, "a slice is shorter than its parent");
        assert!(part <= whole.mul_f64(0.6), "and takes roughly its share");
        assert!(
            deadline.slice(2.0).remaining().unwrap() <= whole,
            "an out-of-range share cannot extend the parent"
        );
    }

    /// Asking before the work is what bounds elapsed time; asking after only
    /// reports an overrun already paid for.
    #[test]
    fn a_spent_slice_admits_no_further_work() {
        assert!(!Deadline::new(0).admits(Duration::from_millis(1)));
        let ample = Deadline::new(1_000);
        assert!(ample.admits(Duration::from_millis(10)));
        assert!(
            !ample.admits(Duration::from_millis(10_000)),
            "a unit costing more than the remaining budget is refused"
        );
    }
}
