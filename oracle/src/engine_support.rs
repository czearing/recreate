use crate::{artifact, cli::CompareArgs, collector::collect, compare};
use std::{fs, time::Instant};

pub(crate) async fn compare(args: CompareArgs) -> anyhow::Result<()> {
    let expected = if args.diagnostic {
        artifact::read_diagnostic(&args.artifact)?
    } else {
        artifact::read(&args.artifact)?
    };
    let mut browser = crate::browser_factory::start(&args.browser).await?;
    browser.prepare().await?;
    anyhow::ensure!(
        expected.environment == browser.environment().await?,
        "browser environment differs from source artifact"
    );
    let started = Instant::now();
    let actual = collect(&mut browser, &args.candidate, &expected.scenarios).await?;
    browser.close().await;
    let report = compare::artifacts(&expected, &actual, started.elapsed());
    let encoded = serde_json::to_vec_pretty(&report)?;
    if let Some(path) = args.out {
        fs::write(path, &encoded)?;
    }
    println!("{}", String::from_utf8(encoded)?);
    anyhow::ensure!(report.certified, "candidate is not certified");
    Ok(())
}

pub(crate) fn has_ambiguous_nodes(checkpoint: &crate::model::Checkpoint) -> bool {
    checkpoint.domains["structure"].value["ambiguous"]
        .as_array()
        .is_some_and(|items| !items.is_empty())
}

pub(crate) fn has_network_evidence(checkpoint: &crate::model::Checkpoint) -> bool {
    let asynchronous = &checkpoint.domains["async"].value;
    asynchronous["network"]
        .as_array()
        .is_some_and(|entries| !entries.is_empty())
        || asynchronous["resources"]
            .as_array()
            .is_some_and(|entries| !entries.is_empty())
        || asynchronous["documentState"]["network"].is_string()
}

pub(crate) fn has_unavailable_network_body(checkpoint: &crate::model::Checkpoint) -> bool {
    checkpoint.domains["async"].value["network"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|entry| entry["body_unavailable"] == true)
}
