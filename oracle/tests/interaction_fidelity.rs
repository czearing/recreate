use base64::{Engine, engine::general_purpose::STANDARD};
use recreate_oracle::{
    artifact,
    browser::Browser,
    checkpoint, compare, discovery, engine,
    model::{Artifact, Coverage},
};

#[tokio::test]
async fn qualifies_stateful_interaction_failures() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("interaction-fixtures");
    let source_html = std::fs::read_to_string(root.join("source.html")).unwrap();
    let source = data_url(&source_html);
    let equivalent = data_url(&std::fs::read_to_string(root.join("equivalent.html")).unwrap());
    let mut browser = Browser::launch(None).await.unwrap();
    browser.prepare().await.unwrap();
    let environment = browser.environment().await.unwrap();
    let discovered = discovery::run(&mut browser, &source, (1280, 800), false)
        .await
        .unwrap();
    let checkpoints = engine::collect(&mut browser, &source, &discovered.scenarios)
        .await
        .unwrap();
    assert!(
        discovered
            .obligations
            .iter()
            .all(|obligation| obligation.status
                != recreate_oracle::model::ObligationStatus::Qualified
                || !obligation.scenarios.is_empty())
    );
    assert!(checkpoints.len() >= discovered.obligations.len());
    let expected = artifact::seal(Artifact {
        format: "recreate-oracle/v1".into(),
        source: source.clone(),
        environment,
        scenarios: discovered.scenarios.clone(),
        obligations: discovered.obligations,
        checkpoints,
        coverage: Coverage {
            widths_required: 0,
            widths_observed: 0,
            domains_required: checkpoint::DOMAINS
                .iter()
                .map(ToString::to_string)
                .collect(),
            incomplete: Vec::new(),
        },
        payload_digest: String::new(),
    })
    .unwrap();
    certify(&mut browser, &expected, &equivalent).await;
    for (name, actual) in artifact_mutants(&expected) {
        let report = compare::artifacts(&expected, &actual, Default::default());
        assert!(!report.certified, "{name} survived");
    }
    browser.close().await;
}

async fn certify(browser: &mut Browser, expected: &Artifact, url: &str) {
    let started = std::time::Instant::now();
    let actual = engine::collect(browser, url, &expected.scenarios)
        .await
        .unwrap();
    assert!(
        started.elapsed().as_secs_f64() < 5.0,
        "candidate replay exceeded five seconds"
    );
    let report = compare::artifacts(expected, &actual, Default::default());
    assert!(report.certified, "{:?}", report.differences);
}

fn data_url(html: &str) -> String {
    format!("data:text/html;base64,{}", STANDARD.encode(html))
}

fn artifact_mutants(
    expected: &Artifact,
) -> Vec<(&'static str, Vec<recreate_oracle::model::Checkpoint>)> {
    [
        ("no-op", "structure"),
        ("wrong-state", "accessibility"),
        ("removal", "structure"),
        ("timing", "interaction"),
        ("error", "async"),
        ("focus", "style"),
        ("scroll", "geometry"),
        ("addition", "structure"),
    ]
    .into_iter()
    .map(|(name, domain)| {
        let mut checkpoints = expected.checkpoints.clone();
        let value = checkpoints[0].domains.get_mut(domain).unwrap();
        if domain == "async" {
            value.value["browser_errors"] = serde_json::json!(1);
        } else {
            value.digest = format!("mutant-{name}");
        }
        (name, checkpoints)
    })
    .collect()
}
