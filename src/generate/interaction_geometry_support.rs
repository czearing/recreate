use crate::{browser, cdp::Cdp, model::Viewport};
use anyhow::{Context, Result};
use serde_json::json;

pub fn viewport(width: u32, height: u32) -> Viewport {
    Viewport {
        width,
        height,
        dpr: 1.0,
    }
}

pub async fn load_state(cdp: &mut Cdp, viewport: &Viewport) -> Result<()> {
    browser::set_viewport(cdp, viewport.width, viewport.height).await?;
    cdp.send("Page.reload", json!({"ignoreCache":false}))
        .await?;
    cdp.evaluate(
        "new Promise(resolve => {\
         const ready=()=>document.fonts.ready.then(()=>requestAnimationFrame(()=>\
         requestAnimationFrame(resolve)));\
         document.readyState==='complete'?ready():addEventListener('load',ready,{once:true})})",
    )
    .await?;
    Ok(())
}

pub async fn assert_body_width(cdp: &mut Cdp, width: u32) -> Result<()> {
    let geometry = cdp
        .evaluate(
            "({body:document.body.getBoundingClientRect().width,\
             client:document.documentElement.clientWidth,\
             scroll:document.documentElement.scrollWidth})",
        )
        .await?;
    let expected = f64::from(width);
    assert_eq!(geometry["body"].as_f64().context("body width")?, expected);
    assert_eq!(
        geometry["client"].as_f64().context("client width")?,
        expected
    );
    assert_eq!(
        geometry["scroll"].as_f64().context("scroll width")?,
        expected
    );
    Ok(())
}

pub async fn validate_boundaries(cdp: &mut Cdp) -> Result<()> {
    for width in [1200, 391, 390, 389] {
        load_state(cdp, &viewport(width, 844)).await?;
        assert_body_width(cdp, width).await?;
        cdp.evaluate(
            "document.querySelector('button').click();\
             new Promise(resolve=>requestAnimationFrame(resolve))",
        )
        .await?;
        assert_body_width(cdp, width).await?;
    }
    Ok(())
}
