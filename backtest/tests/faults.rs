use recreate_backtest::{
    capture,
    digest,
    model::{Artifact, SourceIdentity, SCHEMA_VERSION},
};

#[test]
fn corrupted_artifact_fails_closed() {
    let mut artifact = Artifact {
        schema_version: SCHEMA_VERSION,
        source: SourceIdentity {
            requested_url: "https://source.example".into(),
            rendered_url: "https://source.example".into(),
            browser: "Chromium".into(),
            fingerprint: "source".into(),
        },
        states: Vec::new(),
        digest: String::new(),
    };
    artifact.digest = digest::json(&artifact).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("artifact.json");
    capture::write_artifact(&path, &artifact).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["source"]["fingerprint"] = "tampered".into();
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(capture::read_artifact(&path).is_err());
}

