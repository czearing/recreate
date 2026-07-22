use crate::{artifact, cli::BenchmarkArgs, compare, engine};
use serde_json::json;
use std::time::Instant;

pub async fn run(args: BenchmarkArgs) -> anyhow::Result<()> {
    anyhow::ensure!(args.iterations > 0, "iterations must be greater than zero");
    let expected = artifact::read(&args.artifact)?;
    let mut browser = crate::browser_factory::start(&args.browser).await?;
    browser.prepare().await?;
    for _ in 0..3 {
        let actual = engine::collect(&mut browser, &args.candidate, &expected.scenarios).await?;
        anyhow::ensure!(
            compare::artifacts(&expected, &actual, Default::default()).certified,
            "benchmark warmup candidate failed certification"
        );
    }
    let baseline_targets = browser.target_count().await?;
    let mut samples = Vec::with_capacity(args.iterations);
    for _ in 0..args.iterations {
        let started = Instant::now();
        let actual = engine::collect(&mut browser, &args.candidate, &expected.scenarios).await?;
        let report = compare::artifacts(&expected, &actual, started.elapsed());
        anyhow::ensure!(
            report.certified,
            "benchmark candidate failed certification: {:?}",
            report.first_difference
        );
        samples.push(started.elapsed().as_millis());
    }
    let final_targets = browser.target_count().await?;
    anyhow::ensure!(
        final_targets <= baseline_targets + 1,
        "browser target count grew from {baseline_targets} to {final_targets}"
    );
    browser.close().await;
    samples.sort_unstable();
    let p95 = percentile(&samples, 95);
    let p99 = percentile(&samples, 99);
    let evidence = json!({
        "iterations": samples.len(),
        "warmups": 3,
        "p50_ms": percentile(&samples, 50),
        "p95_ms": p95,
        "p99_ms": p99,
        "samples_ms": samples
        ,"baseline_targets": baseline_targets
        ,"final_targets": final_targets
    });
    let encoded = serde_json::to_vec_pretty(&evidence)?;
    if let Some(path) = args.out {
        std::fs::write(path, &encoded)?;
    }
    println!("{}", String::from_utf8(encoded)?);
    anyhow::ensure!(p95 <= 2_000, "warm p95 exceeded 2000ms: {p95}ms");
    anyhow::ensure!(p99 <= 2_500, "warm p99 exceeded 2500ms: {p99}ms");
    Ok(())
}

fn percentile(samples: &[u128], percentile: usize) -> u128 {
    let rank = (samples.len() * percentile).div_ceil(100);
    samples[rank.saturating_sub(1).min(samples.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_is_bounded() {
        assert_eq!(percentile(&[1, 2, 3, 4], 50), 2);
        assert_eq!(percentile(&[1, 2, 3, 4], 99), 4);
    }
}
