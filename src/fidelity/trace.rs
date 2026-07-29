use super::{Frame, Trace, input};
use crate::{browser, cdp::Cdp, cli::FidelityArgs, fidelity_script};
use anyhow::{Context, Result};
use serde_json::json;

pub(super) async fn reset(args: &FidelityArgs, target_id: &str) -> Result<()> {
    let target = browser::list(&args.cdp_url)
        .await?
        .into_iter()
        .find(|value| value.id == target_id)
        .with_context(|| format!("target not found: {target_id}"))?;
    let mut cdp = Cdp::connect(&target.websocket_url).await?;
    cdp.enable(&["Page", "Runtime"]).await?;
    cdp.send("Page.reload", json!({})).await?;
    for _ in 0..80 {
        let ready = cdp
            .evaluate(
                "document.readyState==='complete'&&document.body&&document.body.children.length>0",
            )
            .await?
            .as_bool()
            .unwrap_or(false);
        if ready {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    anyhow::bail!("target did not reload: {target_id}")
}

pub(super) async fn capture(args: &FidelityArgs, target: &str) -> Result<Trace> {
    let target = browser::list(&args.cdp_url)
        .await?
        .into_iter()
        .find(|value| value.id == target)
        .with_context(|| format!("target not found: {target}"))?;
    let mut cdp = Cdp::connect(&target.websocket_url).await?;
    cdp.enable(&["Page", "Runtime"]).await?;
    cdp.send("Page.bringToFront", json!({})).await?;
    browser::set_viewport(&mut cdp, args.width, args.height).await?;
    cdp.send(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"no-preference"}]}),
    )
    .await?;
    input::move_pointer(&mut cdp, -100.0, -100.0).await?;
    input::press_escape(&mut cdp).await?;
    input::click_pointer(
        &mut cdp,
        f64::from(args.width.saturating_sub(20)),
        f64::from(args.height.saturating_sub(20)),
    )
    .await?;
    input::press_escape(&mut cdp).await?;
    input::settle(50).await;
    let mut descriptor = cdp.evaluate(&fidelity_script::prepare(&args.label)).await?;
    if descriptor["x"].as_f64().is_none() {
        cdp.send("Page.reload", json!({})).await?;
        for _ in 0..20 {
            input::settle(250).await;
            descriptor = cdp.evaluate(&fidelity_script::prepare(&args.label)).await?;
            if descriptor["x"].as_f64().is_some() {
                break;
            }
        }
    }
    descriptor["x"].as_f64().with_context(|| {
        format!(
            "hover target {:?} not found on {} ({})",
            args.label, target.id, target.url,
        ) + &format!(": {descriptor}")
    })?;
    descriptor["y"].as_f64().context("hover target missing y")?;
    input::wait_interactable(&mut cdp).await?;
    descriptor = cdp.evaluate(&fidelity_script::prepare(&args.label)).await?;
    descriptor["x"].as_f64().context("hover target missing x")?;
    descriptor["y"].as_f64().context("hover target missing y")?;
    let mut hover = vec![frame(&mut cdp, 0).await?];
    input::activate_hover(&mut cdp).await?;
    sample(&mut cdp, &mut hover, &[0, 16, 16, 32, 56, 80, 120]).await?;
    input::move_pointer(&mut cdp, -100.0, -100.0).await?;
    let mut leave = Vec::new();
    sample(&mut cdp, &mut leave, &[0, 16, 16, 32, 56, 80, 120]).await?;
    Ok(Trace {
        label: descriptor["label"].as_str().unwrap_or_default().into(),
        hover,
        leave,
    })
}

async fn sample(cdp: &mut Cdp, frames: &mut Vec<Frame>, delays: &[u64]) -> Result<()> {
    let mut elapsed = 0;
    for delay in delays {
        input::settle(*delay).await;
        elapsed += delay;
        frames.push(frame(cdp, elapsed).await?);
    }
    Ok(())
}

async fn frame(cdp: &mut Cdp, elapsed_ms: u64) -> Result<Frame> {
    Ok(Frame {
        elapsed_ms,
        snapshot: serde_json::from_value(cdp.evaluate(fidelity_script::SNAPSHOT).await?)?,
    })
}
