//! The single owner of "can the tool read this page at all?".
//!
//! This is deliberately not `FIXTURES`. That list asserts parity against committed fixtures,
//! and parity is only a meaningful claim about a page that holds still. A production page
//! drifts between loads, so a parity assertion pointed at one would fail for reasons nobody
//! introduced, and a gate that fails for reasons nobody introduced is switched off within a
//! week — taking its true findings with it. The two lists want two questions, not one list
//! with exceptions.
//!
//! So the question here is binary and drift-proof: did a spec come back, and does it
//! describe anything. That is the claim the corpus could never make. Ninety-odd
//! hand-authored scenes assign styles; a component framework declares class fields, and a
//! class field named `style` shadows the accessor the baseline probe reached through, which
//! ended every capture of a framework-rendered page with no artifact at all. Green was never
//! evidence, because the construct was idiomatic in exactly the code no scene contained.
//!
//! Both sides of that boundary are kept here on purpose. A set drawn only from the pages
//! that already worked cannot tell "the tool cannot read this page" apart from "the tool
//! read it and lost detail".

use crate::{browser, capture, cli::CaptureArgs};
use std::path::PathBuf;

/// One live page from each side of the boundary the corpus does not span: `TARGETS[0]` is
/// rendered by a component shell, `TARGETS[1]` is authored as a document.
const TARGETS: &[&str] = &["https://www.youtube.com/", "https://www.youtube.com/about/"];

/// The viewport is incidental — reachability does not depend on width, and one is enough to
/// answer whether a spec comes back at all.
const VIEWPORT: (u32, u32) = (1920, 1080);

#[tokio::test]
#[ignore = "reads live pages over the network; run explicitly as the release gate"]
async fn every_target_yields_a_spec_that_describes_something() {
    let browser_path =
        super::support::browser_path().expect("Chromium browser is required for reachability");
    let port = super::support::free_port();
    let profile = tempfile::tempdir().unwrap();
    let mut child = super::support::launch_browser(&browser_path, profile.path(), port);
    super::support::wait_for_browser(port).await;
    let mut failures = Vec::new();
    for target in TARGETS {
        match reach(target, port).await {
            Ok(0) => failures.push(format!("{target}: spec describes no state")),
            Ok(_) => (),
            Err(error) => failures.push(format!("{target}: {error:#}")),
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    assert!(failures.is_empty(), "unreadable targets: {failures:#?}");
}

/// How many states a capture of `url` produced. An error is the tool failing to read the
/// page, which is the outcome this gate exists to catch.
async fn reach(url: &str, port: u16) -> anyhow::Result<usize> {
    let args = CaptureArgs {
        url: Some(url.into()),
        reuse: false,
        reload: false,
        spec_only: true,
        interactions: false,
        target: None,
        cdp_url: format!("http://127.0.0.1:{port}"),
        out: PathBuf::new(),
        viewports: String::new(),
    };
    let mut session = browser::target(&args).await?;
    let cdp = &mut session.cdp;
    cdp.enable(&["Page", "Runtime", "Network", "DOM", "CSS"])
        .await?;
    let (width, height) = VIEWPORT;
    let state = capture::capture_state(cdp, super::support::viewport(width, height), true).await;
    session.close().await?;
    Ok(state?.nodes.len().min(1))
}
