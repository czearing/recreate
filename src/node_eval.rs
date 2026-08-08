//! Running a shipped page script under Node, so a rule written in JavaScript can be
//! constrained by tests without a browser.
//!
//! Every rule the capture applies inside the page is JavaScript, and every test of one used
//! to carry its own copy of the same dance: a temporary directory, a file, a `node`
//! invocation, and the same unwrapping of the result. One copy of that here means a test of
//! a page rule is a line of setup rather than a block of it, and there is one place to fix
//! when the way a script is run has to change.

use serde_json::Value;

/// Runs `script` under Node and parses the single JSON value it logged.
pub fn json(script: &str) -> Value {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("script.mjs");
    std::fs::write(&path, script).unwrap();
    let output = std::process::Command::new("node")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Runs `expression` in the presence of `preamble` and reports what it evaluated to.
pub fn evaluate(preamble: &str, expression: &str) -> Value {
    json(&format!(
        "{preamble}\nconsole.log(JSON.stringify({expression}));"
    ))
}
