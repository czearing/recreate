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
    let menu = checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.scenario == "interaction-0")
        .collect::<Vec<_>>();
    assert_eq!(menu.len(), 2, "{:?}", discovered.scenarios);
    assert_ne!(
        menu[0].domains["geometry"].digest, menu[1].domains["geometry"].digest,
        "menu activation produced no browser-visible geometry change"
    );
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
    certify(&mut browser, &expected, &source).await;
    certify(&mut browser, &expected, &equivalent).await;
    for (name, html, domain) in mutants(&source_html) {
        assert_ne!(html, source_html, "{name} mutation changed nothing");
        let actual = engine::collect(&mut browser, &data_url(&html), &expected.scenarios)
            .await
            .unwrap();
        let report = compare::artifacts(&expected, &actual, Default::default());
        assert!(!report.certified, "{name} survived");
        assert!(
            report
                .differences
                .iter()
                .any(|difference| difference.domain == domain),
            "{name} did not localize to {domain}: {:?}",
            report.differences
        );
    }
    browser.close().await;
}

async fn certify(browser: &mut Browser, expected: &Artifact, url: &str) {
    let actual = engine::collect(browser, url, &expected.scenarios)
        .await
        .unwrap();
    let report = compare::artifacts(expected, &actual, Default::default());
    assert!(report.certified, "{:?}", report.differences);
}

fn data_url(html: &str) -> String {
    format!("data:text/html;base64,{}", STANDARD.encode(html))
}

fn mutants(source: &str) -> Vec<(&'static str, String, &'static str)> {
    vec![
        (
            "menu offset",
            source.replace(
                ".menu{position:absolute;left:0;top:40px",
                ".menu{position:absolute;left:18px;top:50px",
            ),
            "geometry",
        ),
        (
            "page grey",
            source.replace(
                "document.querySelector('[aria-label=\"Open actions\"]').onclick=()=>menu.hidden=!menu.hidden;",
                "document.querySelector('[aria-label=\"Open actions\"]').onclick=()=>{menu.hidden=!menu.hidden;document.body.style.background='rgb(220,220,220)'};",
            ),
            "style",
        ),
        (
            "pivot inert",
            source.replace(
                "tab.onclick=()=>{tabs.forEach(value=>value.setAttribute('aria-selected',value===tab));panel.textContent=tab.textContent+' content'}",
                "tab.onclick=()=>{}",
            ),
            "structure",
        ),
        (
            "pivot missing",
            source.replace(
                "tabs.forEach(value=>value.setAttribute('aria-selected',value===tab));panel.textContent=tab.textContent+' content'",
                "panel.remove()",
            ),
            "structure",
        ),
        (
            "view inert",
            source.replace(
                "list.onclick=()=>view(list)",
                "list.onclick=()=>view(grid)",
            ),
            "accessibility",
        ),
    ]
}
