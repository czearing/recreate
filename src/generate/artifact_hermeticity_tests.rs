//! The artifact must not carry the address of the machine that made it.
//!
//! A capture serves the page from an ephemeral loopback port, so every URL belonging to the
//! capture rig is a value that (a) names a server which stops existing when the capture
//! ends, and (b) is different on the next run. Either alone would be a defect; together
//! they are worse than a wrong value, because two captures of one static page then emit
//! different bytes and no diff of generated output can separate a repair from noise.
//!
//! Asserted as one sweep over the whole written project rather than against the one line
//! that leaked, because a leak is only ever noticed here by the shape of the value, and the
//! next place it appears will not be the place it appeared last time.

use super::*;
use crate::model::{Attributes, Node, PageState, Rect, Specification, Viewport};

/// The kind of address a capture actually runs on: loopback, ephemeral port, and never
/// reachable from wherever the recreation is later served.
const RIG_URL: &str = "http://localhost:56008/";

fn specification() -> Specification {
    let body = "html>body:nth-of-type(1)";
    let nodes = ["html", body, &format!("{body}>div:nth-of-type(1)")]
        .iter()
        .enumerate()
        .map(|(index, path)| Node {
            writing_mode: Default::default(),
            blocking_overlay: false,
            disabled: false,
            rtl: false,
            path: (*path).to_string(),
            parent: (index > 0).then(|| ["html", body][index - 1].to_string()),
            tag: ["html", "body", "div"][index].into(),
            text: String::new(),
            attributes: Attributes::new(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            style: [("display".into(), "block".into())].into(),
            before: None,
            after: None,
        })
        .collect();
    Specification {
        schema_version: 1,
        requested_url: RIG_URL.into(),
        captured_url: RIG_URL.into(),
        states: vec![PageState {
            url: RIG_URL.into(),
            title: "rig".into(),
            viewport: Viewport {
                width: 1200,
                height: 800,
                ..Default::default()
            },
            nodes,
            ..Default::default()
        }],
        interactions: Vec::new(),
        transitions: Vec::new(),
    }
}

/// No generated source names the capture origin.
///
/// The session-storage key under which the app hands state back to itself across a reload
/// had been built from the captured URL. The recreation is a different document at a
/// different address, so the capture origin never identified it in the first place — it
/// only made the emitted bytes depend on which port the operating system happened to hand
/// out that second.
#[tokio::test]
async fn writes_no_source_naming_the_capture_origin() {
    let directory = tempfile::tempdir().unwrap();
    write_project(&specification(), directory.path(), &[])
        .await
        .unwrap();
    let source = directory.path().join("react/src");
    for (kind, text) in [
        ("script", super::tests::read_source_tree(&source)),
        ("stylesheet", super::tests::read_css_tree(&source)),
    ] {
        for address in ["localhost:", "127.0.0.1"] {
            assert!(
                !text.contains(address),
                "the capture rig's address survived into a generated {kind}, so the \
                 artifact points at a server that dies with the capture: {address}"
            );
        }
    }
}

/// The key still separates two recreations, which is the whole reason it is a key.
///
/// Removing the captured URL must not collapse every recreation onto one storage slot;
/// replacing an identity with a constant would trade a reproducibility defect for a
/// correctness one. The recreation's own path is what distinguishes it, and session storage
/// is already scoped to its origin by the platform.
#[tokio::test]
async fn keys_returned_state_by_where_the_recreation_is_served() {
    let directory = tempfile::tempdir().unwrap();
    write_project(&specification(), directory.path(), &[])
        .await
        .unwrap();
    let app = super::tests::read_source_tree(&directory.path().join("react/src"));
    assert!(
        app.contains("const returnStorageKey=`recreateReturnState:${location.pathname}`"),
        "the returned-state key must still name something that tells two recreations apart"
    );
}
