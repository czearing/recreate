use crate::cdp::Cdp;
use anyhow::{Context, Result};
use serde_json::json;

pub(super) async fn activate_hover(cdp: &mut Cdp) -> Result<()> {
    for _ in 0..40 {
        let point = cdp
            .evaluate(
                "(()=>{const root=window.__recreateFidelityRoot;\
                 const rect=root.getBoundingClientRect();return {\
                 x:rect.x+rect.width/2,y:rect.y+rect.height/2}})()",
            )
            .await?;
        move_pointer(
            cdp,
            point["x"].as_f64().context("hover point missing x")?,
            point["y"].as_f64().context("hover point missing y")?,
        )
        .await?;
        if cdp
            .evaluate("window.__recreateFidelityRoot.matches(':hover')")
            .await?
            .as_bool()
            .unwrap_or(false)
        {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    anyhow::bail!("fidelity hover did not activate")
}

pub(super) async fn wait_interactable(cdp: &mut Cdp) -> Result<()> {
    for _ in 0..480 {
        let interactable = cdp
            .evaluate(
                "(()=>{const root=window.__recreateFidelityRoot;if(!root)return false;\
                 const rect=root.getBoundingClientRect();const hit=document.elementFromPoint(\
                 rect.x+rect.width/2,rect.y+rect.height/2);return !!hit&&root.contains(hit)})()",
            )
            .await?
            .as_bool()
            .unwrap_or(false);
        if interactable {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    anyhow::bail!("fidelity target remained covered")
}

pub(super) async fn move_pointer(cdp: &mut Cdp, x: f64, y: f64) -> Result<()> {
    cdp.send(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseMoved","x":x,"y":y}),
    )
    .await?;
    Ok(())
}

pub(super) async fn click_pointer(cdp: &mut Cdp, x: f64, y: f64) -> Result<()> {
    move_pointer(cdp, x, y).await?;
    for event_type in ["mousePressed", "mouseReleased"] {
        cdp.send(
            "Input.dispatchMouseEvent",
            json!({
                "type": event_type,
                "x": x,
                "y": y,
                "button": "left",
                "clickCount": 1
            }),
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn press_escape(cdp: &mut Cdp) -> Result<()> {
    for event_type in ["keyDown", "keyUp"] {
        cdp.send(
            "Input.dispatchKeyEvent",
            json!({
                "type": event_type,
                "key": "Escape",
                "code": "Escape",
                "windowsVirtualKeyCode": 27,
                "nativeVirtualKeyCode": 27
            }),
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn settle(delay: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
}
