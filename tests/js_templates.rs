//! Every extracted JavaScript template must parse before the browser sees it.
//!
//! The generator's client-side code used to live inside single-line Rust string
//! literals, where nothing could read it. A syntax error in that JavaScript was
//! invisible to `cargo build`, invisible to `cargo test`, and surfaced only when
//! the browser-driven fidelity gate failed several minutes later -- and then as
//! a fidelity failure, which names the wrong cause. Moving the code into
//! `src/generate/templates/*.mjs` makes it readable by a real JavaScript parser,
//! and this file is what actually reads it. Without this test the extraction is
//! only a cosmetic reshuffle: the detection latency it exists to remove would be
//! entirely unchanged.
//!
//! `node --check` parses a file and reports syntax errors without executing it.
//! It selects module goal from the `.mjs` extension, so these files are parsed
//! as ES modules exactly as the browser will parse them. It does not resolve
//! imports and it catches no semantic errors; well-formedness is the whole
//! claim, and byte-identical generated output remains the proof that the
//! substitution is correct.
//!
//! A MISSING `node` IS A FAILURE, NEVER A SKIP. A skipped test and a passing
//! test are indistinguishable in the only signal anyone reads, so a test that
//! quietly steps aside when its parser is absent reports success for a check
//! that never ran -- reinstating exactly the blindness this file exists to
//! remove, and adding a green checkmark on top.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn templates_dir() -> PathBuf {
    crate_root().join("src/generate/templates")
}

/// Files in the template directory that a JavaScript parser cannot accept, each
/// with the reason it is exempt.
///
/// `.jsx` files carry JSX syntax, which is not JavaScript and which no
/// JavaScript parser accepts. `.fragment` files are spliced into a surrounding
/// literal and are deliberately unbalanced, so they are not a complete
/// production in any grammar. Both are excluded by NAME rather than by silently
/// ignoring parse failures, so adding an unparsed template is a decision someone
/// has to write down.
const UNPARSEABLE: [(&str, &str); 4] = [
    ("app_component.jsx", "JSX, not JavaScript"),
    ("state_anchored.jsx", "JSX, not JavaScript"),
    (
        "activate_transition.fragment",
        "spliced into a surrounding literal; intentionally unbalanced",
    ),
    (
        "state_effects.fragment",
        "spliced into a surrounding literal; intentionally unbalanced",
    ),
];

/// Runs `node --check` and returns the parser's own complaint on failure.
///
/// A parser that cannot be launched is reported as a distinct error rather than
/// folded into "did not parse", because the two demand opposite responses: fix
/// the template, or fix the machine.
fn node_check(path: &Path) -> Result<(), String> {
    let output = Command::new("node")
        .arg("--check")
        .arg(path)
        .output()
        .map_err(|error| format!("cannot run `node --check`: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
}

fn templates() -> Vec<PathBuf> {
    let directory = templates_dir();
    let entries = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
    let mut paths: Vec<PathBuf> = entries
        .map(|entry| entry.expect("unreadable directory entry").path())
        .filter(|path| path.is_file())
        .collect();
    paths.sort();
    paths
}

/// The parser must exist. Absence is an environment defect, and it is reported
/// here as its own failure so it can never be mistaken for a clean run.
#[test]
fn node_is_the_parser_and_must_be_present() {
    let Ok(output) = Command::new("node").arg("--version").output() else {
        panic!(
            "`node` is not on PATH, so no template was parsed. This test fails \
             rather than skipping: a skipped parse check is indistinguishable \
             from a passing one, and would restore the multi-minute detection \
             latency that extracting these templates exists to remove."
        );
    };
    assert!(
        output.status.success(),
        "`node --version` failed, so the parser cannot be trusted to check anything"
    );
}

/// Every `.mjs` template parses as an ES module.
///
/// The directory is read at run time rather than from a hardcoded list, so a
/// template added later is checked automatically instead of being forgotten.
#[test]
fn every_mjs_template_parses() {
    let modules: Vec<PathBuf> = templates()
        .into_iter()
        .filter(|path| path.extension().is_some_and(|value| value == "mjs"))
        .collect();

    assert!(
        modules.len() >= 7,
        "expected the extracted module set, found {} files in {} -- a check that \
         parses nothing passes for the wrong reason",
        modules.len(),
        templates_dir().display()
    );

    let started = Instant::now();
    let mut failures = Vec::new();
    for path in &modules {
        if let Err(reason) = node_check(path) {
            failures.push(format!("{}: {reason}", path.display()));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} templates did not parse:\n{}",
        failures.len(),
        modules.len(),
        failures.join("\n")
    );
    println!(
        "parsed {} modules in {}ms",
        modules.len(),
        started.elapsed().as_millis()
    );
}

/// Nothing in the directory escapes classification.
///
/// Every file is either parsed by the test above or named in `UNPARSEABLE` with
/// a reason. This is what stops a future template being dropped in with an
/// extension matching neither rule, and so never read by anything until the
/// browser gate fails.
#[test]
fn every_template_is_either_parsed_or_documented_as_unparseable() {
    let mut unclassified = Vec::new();
    for path in templates() {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let parsed = path.extension().is_some_and(|value| value == "mjs");
        let documented = UNPARSEABLE.iter().any(|(known, _)| *known == name);
        if !parsed && !documented {
            unclassified.push(name);
        }
    }
    assert!(
        unclassified.is_empty(),
        "these templates are neither parsed nor documented as unparseable: {unclassified:?}"
    );

    let present = UNPARSEABLE
        .iter()
        .filter(|(name, _)| templates_dir().join(name).exists())
        .count();
    assert_eq!(
        present,
        UNPARSEABLE.len(),
        "an exemption in UNPARSEABLE no longer matches a real file; a stale \
         exemption silently widens what goes unchecked"
    );
}

/// Templates are LF-only on disk.
///
/// `include_str!` embeds whatever bytes are present, so a template checked out
/// with CRLF would emit CRLF into generated projects where the original Rust
/// `\n` escapes emitted LF. That breaks byte-identity invisibly and differently
/// per machine, which is why `.gitattributes` pins `src/generate/templates/*.mjs`
/// to `eol=lf`. This test is what proves the pin is working.
#[test]
fn templates_are_lf_only() {
    let mut offenders = Vec::new();
    for path in templates() {
        let bytes = fs::read(&path).expect("unreadable template");
        if bytes.windows(2).any(|pair| pair == b"\r\n") {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "CRLF found in templates, which would change generated bytes: {offenders:?}"
    );
}
