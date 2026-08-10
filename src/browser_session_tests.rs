use super::browser::Tab;
use std::{fs, path::PathBuf};

/// A tab this process opened must be closed when the work that needed it ends.
/// The browser outlives the process, so a tab left behind is never reclaimed:
/// it is not a leak that a restart clears, it is one that accumulates run after
/// run until every later capture pays for it.
#[test]
fn a_tab_this_process_opened_is_closed_when_the_session_ends() {
    assert_eq!(
        Tab::new("http://127.0.0.1:9222", "TAB-1", true).expiring(),
        Some("TAB-1")
    );
}

/// The operator's own tab, prepared with `recreate open` and named by
/// `--reuse`, is not ours to close. Capturing it must leave it open.
#[test]
fn a_tab_the_operator_opened_outlives_the_session() {
    assert_eq!(
        Tab::new("http://127.0.0.1:9222", "TAB-1", false).expiring(),
        None
    );
}

fn rust_sources() -> Vec<(PathBuf, String)> {
    let mut pending = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")];
    let mut sources = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("read source directory") {
            let path = entry.expect("source entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|value| value == "rs") {
                let text = fs::read_to_string(&path).expect("read source");
                sources.push((path, text));
            }
        }
    }
    sources
}

/// The rule "close the tab you opened" cannot be enforced by remembering to
/// write it down, because the two call sites that forgot were the two that ran
/// most. A file that opens a tab must dispose of it, either by closing it or by
/// handing the session to its caller, and this test fails the moment a new call
/// site does neither.
#[test]
fn every_opened_session_is_closed_by_the_code_that_opened_it() {
    let offenders: Vec<_> = rust_sources()
        .into_iter()
        .filter(|(_, text)| {
            text.contains("browser::target(")
                && !text.contains(".close()")
                && !text.contains("Result<browser::Session>")
        })
        .map(|(path, _)| path.display().to_string())
        .collect();
    assert!(
        offenders.is_empty(),
        "these open a browser tab and neither close it nor hand it on: {offenders:?}"
    );
}

/// Closing must be reached even when the work fails. A capture that errors
/// still opened a tab, and the browser keeps it either way.
#[test]
fn a_failing_capture_still_closes_its_tab() {
    let text = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("capture.rs"),
    )
    .expect("read capture.rs");
    let close = text
        .find(".close()")
        .expect("capture.rs closes its session");
    let propagate = text
        .find("let outcome =")
        .expect("capture.rs defers its result");
    assert!(
        propagate < close,
        "capture.rs must close the tab before propagating a failure"
    );
}

/// A capture must observe the page and nothing else. An extension's content script writes
/// through the same DOM interface the page does, so once its edits are captured no downstream
/// stage can tell them apart — a scene authoring no attributes at all emitted an extension's
/// `data-rdwebrtc-ext-url`, and the extension's poll loop was read as the page's own
/// outstanding work at a cost of 5.5s per capture.
///
/// A private profile does not prevent this: a managed browser force-installs by policy into
/// every profile, including a brand new one. So the exclusion belongs in the launch.
#[test]
fn the_capture_browser_runs_none_of_the_machine_s_own_extensions() {
    let args = super::browser::launch_args(9222, std::path::Path::new("profile"));
    for flag in [
        "--disable-extensions",
        "--disable-component-extensions-with-background-pages",
    ] {
        assert!(
            args.iter().any(|value| value == flag),
            "capture browser must launch with {flag}"
        );
    }
}

/// The capture browser must keep its own profile directory. Sharing the operator's would
/// carry their cookies, storage and session into the recreation, and would make the capture
/// contend with a browser they are using.
#[test]
fn the_capture_browser_keeps_its_own_profile() {
    let args = super::browser::launch_args(9222, std::path::Path::new("profile"));
    assert!(
        args.iter().any(|value| value == "--user-data-dir=profile"),
        "capture browser must run in its own profile: {args:?}"
    );
}
