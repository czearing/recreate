use std::{fs, path::Path};

#[test]
fn shipping_sources_are_small_and_isolated() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|value| value != "rs") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        let lines = source.lines().count();
        assert!(
            lines < 200,
            "{} has {lines} lines; shipping modules must stay below 200",
            path.display()
        );
        for forbidden in [
            "recreate::",
            "crate::capture",
            "crate::generate",
            "../src/",
            "fidelity.rs",
        ] {
            assert!(
                !source.contains(forbidden),
                "{} violates oracle isolation with {forbidden}",
                path.display()
            );
        }
    }
}

#[test]
fn fixture_corpus_contains_equivalence_and_mutations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    assert!(root.join("source.html").is_file());
    assert!(root.join("equivalent.html").is_file());
    assert!(root.join("corpus.json").is_file());
    let mutants = fs::read_dir(root.join("mutants")).unwrap().count();
    assert!(
        mutants >= 4,
        "qualification corpus must cover multiple domains"
    );
}

#[test]
fn engine_and_oracle_share_browser_transport() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let shared = fs::read_to_string(workspace.join("browser/src/cdp.rs")).unwrap();
    let engine = fs::read_to_string(workspace.join("src/cdp.rs")).unwrap();
    let oracle = fs::read_to_string(workspace.join("oracle/src/cdp.rs")).unwrap();
    assert!(shared.contains("pub struct Cdp"));
    assert_eq!(engine.trim(), "pub use recreate_browser::Cdp;");
    assert_eq!(oracle.trim(), "pub use recreate_browser::Cdp;");
}
