use crate::{
    browser,
    cli::CaptureArgs,
    generate, interactions, lifecycle_script,
    model::{BrowserCookie, PageState, Specification, Viewport},
    page_script, probe, validate,
};
use anyhow::{Context, Result};
use base64::Engine;
use serde_json::json;
use std::{fs, time::Duration};

pub async fn run(args: CaptureArgs) -> Result<()> {
    let viewports = probe::parse_viewports(&args.viewports)?;
    fs::create_dir_all(&args.out)?;
    let (target, mut cdp) = browser::target(&args).await?;
    cdp.enable(&["Page", "Runtime", "Network", "DOM", "CSS"])
        .await?;
    cdp.send(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({ "source": lifecycle_script::SOURCE }),
    )
    .await?;
    let requested_url = args.url.clone().unwrap_or_else(|| target.url.clone());
    let mut states = Vec::new();
    for (index, viewport) in viewports.into_iter().enumerate() {
        let state = capture_state(&mut cdp, viewport.clone(), index == 0).await?;
        let screenshot = cdp
            .send("Page.captureScreenshot", json!({ "format": "png" }))
            .await?;
        if let Some(data) = screenshot["data"].as_str() {
            fs::write(
                args.out
                    .join(format!("source-{}x{}.png", viewport.width, viewport.height)),
                base64::engine::general_purpose::STANDARD.decode(data)?,
            )?;
        }
        states.push(state);
    }
    let captured_url = states
        .first()
        .map(|state| state.url.clone())
        .unwrap_or_else(|| requested_url.clone());
    let interaction_states = interactions::capture(&mut cdp, &states).await?;
    let specification = Specification {
        schema_version: 1,
        requested_url,
        captured_url,
        states,
        interactions: interaction_states,
    };
    fs::write(
        args.out.join("spec.json"),
        serde_json::to_vec_pretty(&specification)?,
    )?;
    let cookies = browser_cookies(&mut cdp).await;
    generate::write_project(&specification, &args.out, &cookies).await?;
    let acceptance = validate::validate(&specification, &args.out)?;
    fs::write(
        args.out.join("acceptance.json"),
        serde_json::to_vec_pretty(&acceptance)?,
    )?;
    if !acceptance.passed {
        anyhow::bail!("capture validation failed");
    }
    println!("{}", serde_json::to_string_pretty(&acceptance)?);
    Ok(())
}

pub async fn capture_state(
    cdp: &mut crate::cdp::Cdp,
    viewport: Viewport,
    reload: bool,
) -> Result<PageState> {
    browser::set_viewport(cdp, viewport.width, viewport.height).await?;
    if reload {
        cdp.send("Page.reload", json!({ "ignoreCache": false }))
            .await?;
    }
    wait_ready(cdp).await?;
    read_state(cdp, viewport).await
}

pub async fn read_state(cdp: &mut crate::cdp::Cdp, viewport: Viewport) -> Result<PageState> {
    let source = page_script::source();
    let raw = cdp.evaluate(&source).await?;
    let text = raw.as_str().context("capture script returned non-string")?;
    let mut state: PageState = serde_json::from_str(text)?;
    state.viewport = viewport;
    Ok(state)
}

async fn browser_cookies(cdp: &mut crate::cdp::Cdp) -> Vec<BrowserCookie> {
    cdp.send("Network.getAllCookies", json!({}))
        .await
        .ok()
        .and_then(|value| serde_json::from_value(value["cookies"].clone()).ok())
        .unwrap_or_default()
}

async fn wait_ready(cdp: &mut crate::cdp::Cdp) -> Result<()> {
    let started = std::time::Instant::now();
    let mut previous = String::new();
    let mut stable = 0;
    for _ in 0..120 {
        let value = cdp
            .evaluate(
                r#"(() => {
                  const visible = Array.from(document.querySelectorAll('*'))
                    .filter(element => {
                      const rect = element.getBoundingClientRect();
                      const style = getComputedStyle(element);
                      return rect.width > 0 && rect.height > 0 &&
                        style.display !== 'none' && style.visibility !== 'hidden' &&
                        Number(style.opacity || 1) > 0;
                    })
                    .slice(0, 80)
                    .map(element => {
                      const rect = element.getBoundingClientRect();
                      const style = getComputedStyle(element);
                      return [
                        element.tagName, Math.round(rect.x), Math.round(rect.y),
                        Math.round(rect.width), Math.round(rect.height),
                        style.display, style.opacity, style.transform
                      ].join(':');
                    }).join('|');
                  return {
                  ready: document.readyState === 'complete' &&
                    document.fonts.status === 'loaded' &&
                    window.__recreateLifecycleDone === true &&
                    (window.__recreatePendingRequests || 0) === 0,
                  signature: visible,
                  blocking: Array.from(document.querySelectorAll('*')).some(element => {
                    const rect = element.getBoundingClientRect();
                    const style = getComputedStyle(element);
                    const area = rect.width * rect.height;
                    const controls = element.querySelectorAll(
                      'a,button,input,select,textarea,[role="button"]'
                    ).length;
                    return (
                      area > innerWidth * innerHeight * 0.6 &&
                      ['absolute','fixed'].includes(style.position) &&
                      style.display !== 'none' &&
                      style.visibility !== 'hidden' &&
                      Number(style.opacity || 1) > 0 &&
                      controls <= 3 &&
                      (element.innerText || '').trim().length < 300
                    );
                  })
                };
                })()"#,
            )
            .await?;
        let signature = value["signature"].as_str().unwrap_or_default();
        let blocking = value["blocking"].as_bool() == Some(true);
        let blocking_grace_elapsed = started.elapsed() >= Duration::from_secs(15);
        if value["ready"].as_bool() == Some(true)
            && (!blocking || blocking_grace_elapsed)
            && !signature.is_empty()
            && signature == previous
        {
            stable += 1;
            if stable >= 3 {
                return Ok(());
            }
        } else {
            stable = 0;
        }
        previous = signature.to_string();
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    anyhow::bail!("page did not become stable")
}
