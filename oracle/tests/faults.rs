use recreate_oracle::{
    artifact,
    checkpoint::DOMAINS,
    digest,
    model::{
        Artifact, Checkpoint, Coverage, Domain, Environment, Obligation, ObligationStatus,
        Scenario, Step, Viewport,
    },
};
use std::collections::BTreeMap;

fn valid_artifact() -> Artifact {
    let domains = DOMAINS
        .iter()
        .map(|name| {
            (
                (*name).into(),
                Domain {
                    digest: digest::json(&serde_json::Value::Null).unwrap(),
                    value: serde_json::Value::Null,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    artifact::seal(Artifact {
        format: "recreate-oracle/v1".into(),
        source: "https://source.invalid".into(),
        environment: Environment {
            schema: 1,
            browser_product: "Chromium/test".into(),
            browser_revision: "revision".into(),
            protocol_version: "1.3".into(),
            command_line_digest: "flags".into(),
            operating_system: "test".into(),
            architecture: "test".into(),
            locale: "en-US".into(),
            timezone: "UTC".into(),
            color_scheme: "light".into(),
            reduced_motion: false,
            device_scale_factor_milli: 1000,
        },
        scenarios: vec![Scenario {
            id: "responsive-test".into(),
            steps: vec![Step::SetViewport {
                width: 320,
                height: 800,
            }],
        }],
        obligations: vec![Obligation {
            id: "test".into(),
            kind: "test".into(),
            status: ObligationStatus::Qualified,
            scenarios: vec!["responsive-test".into()],
        }],
        checkpoints: vec![Checkpoint {
            scenario: "responsive-test".into(),
            step: 0,
            viewport: Viewport {
                width: 320,
                height: 800,
            },
            domains,
        }],
        coverage: Coverage {
            widths_required: 1,
            widths_observed: 1,
            domains_required: DOMAINS.iter().map(ToString::to_string).collect(),
            incomplete: vec![],
        },
        payload_digest: String::new(),
    })
    .unwrap()
}

#[test]
fn round_trip_preserves_verified_artifact() {
    let artifact = valid_artifact();
    let encoded = serde_json::to_vec(&artifact).unwrap();
    let decoded: Artifact = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(artifact, decoded);
    artifact::verify(&decoded).unwrap();
}

#[test]
fn infrastructure_faults_fail_closed() {
    let mut corrupt_leaf = valid_artifact();
    corrupt_leaf.checkpoints[0]
        .domains
        .get_mut("style")
        .unwrap()
        .value = serde_json::json!("poisoned");
    let corrupt_leaf = artifact::seal(corrupt_leaf).unwrap();
    assert!(artifact::verify(&corrupt_leaf).is_err());

    let mut missing_checkpoint = valid_artifact();
    missing_checkpoint.checkpoints.clear();
    let missing_checkpoint = artifact::seal(missing_checkpoint).unwrap();
    assert!(artifact::verify(&missing_checkpoint).is_err());

    let mut missing_domain = valid_artifact();
    missing_domain.checkpoints[0].domains.remove("compositor");
    let missing_domain = artifact::seal(missing_domain).unwrap();
    assert!(artifact::verify(&missing_domain).is_err());

    let mut opaque = valid_artifact();
    opaque.obligations[0].status = ObligationStatus::Opaque;
    let opaque = artifact::seal(opaque).unwrap();
    assert!(artifact::verify(&opaque).is_err());
}
