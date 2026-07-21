use crate::{browser, cdp::Cdp, cli::CaptureArgs, lifecycle_script};
use anyhow::{Context, Result};
use serde_json::json;

use super::interaction_runtime_support as support;

const VIEWPORTS: [(u32, u32, &str, &str, u32); 2] = [
    (1440, 900, "Enter", "Enter", 13),
    (390, 844, " ", "Space", 32),
];

#[tokio::test]
#[ignore = "requires RECREATE_CDP_URL and RECREATE_AUTHENTICATED_URL"]
async fn durable_authenticated_interaction_is_keyboard_accessible() -> Result<()> {
    let endpoint = required("RECREATE_CDP_URL")?;
    let runtime = required("RECREATE_AUTHENTICATED_URL")?;
    let mut cdp = connect(&runtime, &endpoint).await?;
    for (width, height, key, code, key_code) in VIEWPORTS {
        load(&mut cdp, width, height).await?;
        tab_to_trigger(&mut cdp).await?;
        assert_trigger(&mut cdp, "false").await?;
        support::press(&mut cdp, key, code, key_code).await?;
        ready(&mut cdp).await?;
        assert_open(&mut cdp).await?;
        support::press(&mut cdp, "Escape", "Escape", 27).await?;
        ready(&mut cdp).await?;
        assert_trigger(&mut cdp, "false").await?;
    }
    validate_responsive_persistence(&mut cdp).await?;
    assert_eq!(support::errors(&mut cdp), (0, 0));
    Ok(())
}

async fn validate_responsive_persistence(cdp: &mut Cdp) -> Result<()> {
    load(cdp, 1440, 900).await?;
    tab_to_trigger(cdp).await?;
    support::press(cdp, "Enter", "Enter", 13).await?;
    ready(cdp).await?;
    browser::set_viewport(cdp, 390, 844).await?;
    ready(cdp).await?;
    assert_open(cdp).await?;
    support::press(cdp, "Escape", "Escape", 27).await?;
    ready(cdp).await?;
    assert_trigger(cdp, "false").await
}

async fn tab_to_trigger(cdp: &mut Cdp) -> Result<()> {
    for _ in 0..100 {
        support::press(cdp, "Tab", "Tab", 9).await?;
        if bool_value(cdp, "document.activeElement?.dataset.recreateTrigger==='1'").await? {
            return Ok(());
        }
    }
    anyhow::bail!("interaction trigger was not reachable within 100 Tab presses")
}

async fn assert_trigger(cdp: &mut Cdp, expanded: &str) -> Result<()> {
    let value = cdp
        .evaluate(
            "({marker:document.activeElement?.dataset.recreateTrigger,\
              expanded:document.activeElement?.getAttribute('aria-expanded'),\
              popup:document.activeElement?.getAttribute('aria-haspopup'),\
              tag:document.activeElement?.tagName.toLowerCase()})",
        )
        .await?;
    assert_eq!(value["marker"].as_str(), Some("1"));
    assert_eq!(value["expanded"].as_str(), Some(expanded));
    assert_eq!(value["popup"].as_str(), Some("listbox"));
    assert_eq!(value["tag"].as_str(), Some("button"));
    Ok(())
}

async fn assert_open(cdp: &mut Cdp) -> Result<()> {
    let value = cdp
        .evaluate(
            "(() => {const trigger=document.querySelector('[aria-haspopup=listbox]');\
              const listbox=document.querySelector('[role=listbox]');\
              const options=[...document.querySelectorAll('[role=option]')];\
              return {expanded:trigger?.getAttribute('aria-expanded'),\
                active:document.activeElement===listbox,tabIndex:listbox?.tabIndex,\
                options:options.length,selected:options.filter(option=>\
                  option.getAttribute('aria-selected')==='true').length};})()",
        )
        .await?;
    assert_eq!(value["expanded"].as_str(), Some("true"));
    assert_eq!(value["active"].as_bool(), Some(true));
    assert_eq!(value["tabIndex"].as_i64(), Some(-1));
    assert_eq!(value["options"].as_u64(), Some(11));
    assert_eq!(value["selected"].as_u64(), Some(1));
    Ok(())
}

async fn load(cdp: &mut Cdp, width: u32, height: u32) -> Result<()> {
    browser::set_viewport(cdp, width, height).await?;
    cdp.send("Page.reload", json!({"ignoreCache":true})).await?;
    ready(cdp).await
}

async fn ready(cdp: &mut Cdp) -> Result<()> {
    cdp.evaluate(
        "new Promise(resolve=>{const done=()=>requestAnimationFrame(()=>\
         requestAnimationFrame(resolve));document.readyState==='complete'?done():\
         addEventListener('load',done,{once:true})})",
    )
    .await?;
    Ok(())
}

async fn bool_value(cdp: &mut Cdp, expression: &str) -> Result<bool> {
    Ok(cdp.evaluate(expression).await?.as_bool() == Some(true))
}

async fn connect(url: &str, endpoint: &str) -> Result<Cdp> {
    let args = CaptureArgs {
        url: Some(url.into()),
        reuse: false,
        reload: false,
        baseline_only: false,
        spec_only: false,
        target: None,
        cdp_url: endpoint.into(),
        out: Default::default(),
        viewports: String::new(),
    };
    let (_, mut cdp) = browser::target(&args).await?;
    cdp.enable(&["Page", "Runtime", "Network", "DOM", "CSS"])
        .await?;
    cdp.send("Network.setCacheDisabled", json!({"cacheDisabled":true}))
        .await?;
    cdp.send(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({"source": lifecycle_script::SOURCE}),
    )
    .await?;
    Ok(cdp)
}

fn required(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} is required"))
}
