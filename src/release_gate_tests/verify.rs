use super::{VIEWPORTS, runtime::Server, support::collect_errors};
use crate::{browser, capture, cli::CaptureArgs, lifecycle_script, model::Specification};
use anyhow::Result;
use serde_json::json;
use std::path::{Path, PathBuf};

pub struct Verification {
    pub parity_mismatches: usize,
    pub parity_details: Vec<String>,
    pub horizontal_overflows: usize,
    pub console_errors: usize,
    pub network_errors: usize,
    pub keyboard_activation: bool,
    pub focus_restoration: bool,
    pub reduced_motion: bool,
}

pub async fn generated(spec: &Specification, dist: &Path, port: u16) -> Result<Verification> {
    let server = Server::start(dist)?;
    let args = CaptureArgs {
        url: Some(server.url()),
        reuse: false,
        target: None,
        cdp_url: format!("http://127.0.0.1:{port}"),
        out: PathBuf::new(),
        viewports: String::new(),
    };
    let (_, mut cdp) = browser::target(&args).await?;
    cdp.enable(&["Page", "Runtime", "Network", "DOM", "CSS"])
        .await?;
    cdp.send(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({"source": lifecycle_script::SOURCE}),
    )
    .await?;
    let mut parity_mismatches = 0;
    let mut parity_details = Vec::new();
    let mut horizontal_overflows = 0;
    for (index, expected) in spec.states.iter().enumerate() {
        let &(width, height) = VIEWPORTS.get(index).expect("viewport evidence");
        let actual = capture::capture_state(
            &mut cdp,
            super::support::viewport(width, height),
            index == 0,
        )
        .await?;
        let parity = super::support::parity(expected, &actual);
        parity_mismatches += parity.mismatches;
        if parity_details.len() < 20 {
            parity_details.extend(
                parity
                    .details
                    .into_iter()
                    .take(20usize.saturating_sub(parity_details.len())),
            );
        }
        if cdp
            .evaluate(
                "document.documentElement.scrollWidth > innerWidth || document.body.scrollWidth > innerWidth",
            )
            .await?
            .as_bool()
            == Some(true)
        {
            horizontal_overflows += 1;
        }
    }
    let (keyboard_activation, focus_restoration, reduced_motion) =
        verify_interaction(spec, &mut cdp).await?;
    let (console_errors, network_errors) = collect_errors(&mut cdp);
    drop(server);
    Ok(Verification {
        parity_mismatches,
        parity_details,
        horizontal_overflows,
        console_errors,
        network_errors,
        keyboard_activation,
        focus_restoration,
        reduced_motion,
    })
}

async fn verify_interaction(
    spec: &Specification,
    cdp: &mut crate::cdp::Cdp,
) -> Result<(bool, bool, bool)> {
    let Some(interaction) = spec.interactions.first() else {
        return Ok((true, true, true));
    };
    crate::browser::set_viewport(cdp, VIEWPORTS[0].0, VIEWPORTS[0].1).await?;
    settle(cdp).await?;
    cdp.evaluate("document.querySelector('[data-recreate-control]').focus()")
        .await?;
    dispatch_key(cdp, "Enter", 13).await?;
    settle(cdp).await?;
    let keyboard = cdp
        .evaluate("document.querySelector('[role=\"dialog\"]') !== null")
        .await?
        == json!(true);
    for expected in &interaction.states {
        crate::browser::set_viewport(cdp, expected.viewport.width, expected.viewport.height)
            .await?;
        settle(cdp).await?;
        let actual = capture::read_state(cdp, expected.viewport.clone()).await?;
        let parity = super::support::parity(expected, &actual);
        assert_eq!(parity.mismatches, 0, "interaction: {:?}", parity.details);
    }
    crate::browser::set_viewport(cdp, VIEWPORTS[0].0, VIEWPORTS[0].1).await?;
    dispatch_key(cdp, "Escape", 27).await?;
    settle(cdp).await?;
    let restored = cdp
        .evaluate("document.activeElement?.hasAttribute('data-recreate-control') === true")
        .await?
        == json!(true);
    cdp.send(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    settle(cdp).await?;
    let media_reduced = cdp
        .evaluate("matchMedia('(prefers-reduced-motion: reduce)').matches")
        .await?
        == json!(true);
    dispatch_key(cdp, "Enter", 13).await?;
    settle(cdp).await?;
    let reduced = cdp
        .evaluate(
            "(() => { const node=document.querySelector('[role=\"dialog\"]'); \
             return node.getAnimations({subtree:true}).every(animation => { \
             const duration=Number(animation.effect?.getComputedTiming().duration || 0); \
             return duration===0 || (!animation.pending && animation.playState!=='running'); \
             }); })()",
        )
        .await?
        == json!(true);
    Ok((keyboard, restored, media_reduced && reduced))
}

async fn dispatch_key(cdp: &mut crate::cdp::Cdp, key: &str, code: u32) -> Result<()> {
    for kind in ["rawKeyDown", "keyUp"] {
        cdp.send(
            "Input.dispatchKeyEvent",
            json!({"type":kind,"key":key,"code":key,"windowsVirtualKeyCode":code}),
        )
        .await?;
    }
    Ok(())
}

async fn settle(cdp: &mut crate::cdp::Cdp) -> Result<()> {
    cdp.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))")
        .await?;
    Ok(())
}
