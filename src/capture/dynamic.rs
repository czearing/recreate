use anyhow::Result;
use std::time::{Duration, Instant};

/// The longest a page is watched for recurring attribute behaviour.
const WINDOW_MS: u64 = 12_000;
/// The shortest observation, so a sequence that starts late is still seen.
const FLOOR_MS: u64 = 4_000;
/// How long a page must record nothing new before it counts as settled.
const QUIET_MS: u64 = 2_000;
const POLL_MS: u64 = 250;

/// Reports the number of recorded changes and how many change groups have not
/// yet proven a repeating cycle, using the same grouping rules the sequence
/// capture applies afterwards.
const READING: &str = r#"(() => {
  const events = window.__recreateAttributeMutations || [];
  const groups = new Map();
  for (const event of events) {
    const key = `${event.target}|${event.attribute}`;
    const values = groups.get(key) || [];
    if (values.at(-1) !== event.value) values.push(event.value);
    groups.set(key, values);
  }
  let pending = 0;
  for (const values of groups.values()) {
    let cycle = values.length;
    for (let size = 1; size <= Math.floor(values.length / 2); size++) {
      if (values.every((value, index) => value === values[index % size])) {
        cycle = size;
        break;
      }
    }
    if (values.length < 3 || cycle === values.length) pending++;
  }
  return `${events.length}:${pending}`;
})()"#;

/// Watches the page for the recurring attribute changes that become sequences.
///
/// A page that is still changing, or whose changes have not yet repeated, is
/// watched for the whole window. A page that has recorded nothing new and has
/// no unfinished sequence stops early, so a static page no longer pays the
/// full window.
pub(super) async fn observe(cdp: &mut crate::cdp::Cdp) -> Result<()> {
    let started = Instant::now();
    let mut last_change = Instant::now();
    let mut previous = String::new();
    while started.elapsed() < Duration::from_millis(WINDOW_MS) {
        tokio::time::sleep(Duration::from_millis(POLL_MS)).await;
        let reading = cdp
            .evaluate(READING)
            .await?
            .as_str()
            .unwrap_or_default()
            .to_string();
        if reading != previous {
            previous = reading;
            last_change = Instant::now();
            continue;
        }
        if settled(started.elapsed(), last_change.elapsed(), &previous) {
            break;
        }
    }
    Ok(())
}

fn settled(elapsed: Duration, quiet: Duration, reading: &str) -> bool {
    elapsed >= Duration::from_millis(FLOOR_MS)
        && quiet >= Duration::from_millis(QUIET_MS)
        && reading.split(':').nth(1) == Some("0")
}

#[cfg(test)]
mod tests {
    use super::{FLOOR_MS, READING, settled};
    use std::time::Duration;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn a_settled_page_stops_after_the_floor() {
        assert!(settled(ms(FLOOR_MS), ms(2_000), "0:0"));
        assert!(settled(ms(5_000), ms(3_000), "48:0"));
    }

    #[test]
    fn a_page_is_never_cut_short_of_the_floor() {
        assert!(!settled(ms(FLOOR_MS - 1), ms(3_000), "0:0"));
    }

    #[test]
    fn an_unfinished_sequence_keeps_the_full_window() {
        assert!(!settled(ms(11_000), ms(9_000), "12:1"));
    }

    #[test]
    fn a_still_changing_page_keeps_the_full_window() {
        assert!(!settled(ms(11_000), ms(500), "40:0"));
    }

    #[test]
    fn the_reading_groups_changes_the_way_sequence_capture_does() {
        assert!(READING.contains("__recreateAttributeMutations"));
        assert!(READING.contains("values.at(-1) !== event.value"));
        assert!(READING.contains("value === values[index % size]"));
        assert!(READING.contains("values.length < 3 || cycle === values.length"));
    }
}
