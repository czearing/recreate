use crate::cdp::Cdp;
use serde_json::json;

pub(crate) async fn focus(cdp: &mut Cdp) -> anyhow::Result<()> {
    cdp.send("Page.bringToFront", json!({})).await?;
    cdp.send(
        "Emulation.setFocusEmulationEnabled",
        json!({"enabled":true}),
    )
    .await?;
    Ok(())
}
