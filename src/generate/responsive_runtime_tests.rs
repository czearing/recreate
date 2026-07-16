use super::responsive_runtime_support::{
    assert_clean_events, assert_exact_parity, assert_no_overflow, assert_stable_dom_text,
    capture_matrix, connect, equivalent_style,
};
use crate::{browser, capture, model::Viewport};
use anyhow::{Context, Result};
use std::path::Path;

const BOUNDARIES: [u32; 16] = [
    319, 320, 321, 389, 390, 391, 767, 768, 769, 1438, 1439, 1440, 1441, 1560, 1919, 1920,
];

#[tokio::test]
#[ignore = "requires RECREATE_CDP_URL and RECREATE_GENERATED_URL"]
async fn generated_runtime_matches_five_captured_layouts() -> Result<()> {
    let endpoint = required("RECREATE_CDP_URL")?;
    let runtime_url = required("RECREATE_GENERATED_URL")?;
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/fixtures/responsive-bands.html");
    let source_url = url::Url::from_file_path(fixture).unwrap().to_string();
    let mut source = connect(&source_url, &endpoint).await?;
    let states = capture_matrix(&mut source).await?;
    let mut runtime = connect(&runtime_url, &endpoint).await?;
    let actual = capture_matrix(&mut runtime).await?;
    for (expected, actual) in states.iter().zip(actual.iter()) {
        assert_exact_parity(expected, actual);
    }
    assert_clean_events(&mut runtime);
    Ok(())
}

#[tokio::test]
#[ignore = "requires RECREATE_CDP_URL and RECREATE_GENERATED_URL"]
async fn generated_runtime_boundary_sweep_is_collision_free() -> Result<()> {
    let endpoint = required("RECREATE_CDP_URL")?;
    let runtime_url = required("RECREATE_GENERATED_URL")?;
    let mut runtime = connect(&runtime_url, &endpoint).await?;
    capture::capture_state(
        &mut runtime,
        Viewport {
            width: 1920,
            height: 1080,
            dpr: 1.0,
        },
        true,
    )
    .await?;
    for width in BOUNDARIES {
        assert_boundary(&mut runtime, width)
            .await
            .with_context(|| format!("{width}px boundary"))?;
    }
    assert_clean_events(&mut runtime);
    Ok(())
}

async fn assert_boundary(cdp: &mut crate::cdp::Cdp, width: u32) -> Result<()> {
    let viewport = Viewport {
        width,
        height: 900,
        dpr: 1.0,
    };
    browser::set_viewport(cdp, width, 900).await?;
    cdp.evaluate(
        "new Promise(resolve => requestAnimationFrame(() => \
         requestAnimationFrame(resolve)))",
    )
    .await?;
    let first = capture::read_state(cdp, viewport.clone()).await?;
    let second = capture::read_state(cdp, viewport).await?;
    assert_stable_dom_text(&first, &second);
    assert_no_overflow(cdp, width).await?;
    let evidence = cdp
        .evaluate(
            "(() => {\
              const card = document.querySelector('main');\
              const style = getComputedStyle(card);\
              const before = getComputedStyle(card, '::before');\
              const bands = [\
                matchMedia('(max-width:320px)').matches,\
                matchMedia('(min-width:321px) and (max-width:390px)').matches,\
                matchMedia('(min-width:391px) and (max-width:768px)').matches,\
                matchMedia('(min-width:769px) and (max-width:1440px)').matches,\
                matchMedia('(min-width:1441px)').matches\
              ];\
              return {active:bands.filter(Boolean).length,width:style.width,\
                content:before.content,color:before.color};\
            })()",
        )
        .await?;
    let (expected_width, content, color) = expected_band(width);
    assert_eq!(evidence["active"], 1, "{width}px active bands");
    assert_eq!(evidence["width"], expected_width, "{width}px card width");
    assert_eq!(evidence["content"], content, "{width}px pseudo content");
    assert_eq!(evidence["color"], color, "{width}px pseudo color");
    Ok(())
}

fn expected_band(width: u32) -> (&'static str, &'static str, &'static str) {
    match width {
        0..=320 => ("70px", "none", "rgb(255, 0, 0)"),
        321..=390 => ("80px", "\"mobile\"", "rgb(0, 0, 255)"),
        391..=768 => ("90px", "\"wide\"", "rgb(255, 0, 0)"),
        769..=1440 => ("95px", "\"wide\"", "rgb(255, 0, 0)"),
        _ => ("100px", "\"wide\"", "rgb(255, 0, 0)"),
    }
}

fn required(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} is required"))
}

#[test]
fn zero_position_units_are_browser_equivalent() {
    assert!(equivalent_style("background-position", "0% 0%", "0px 0px"));
    assert!(!equivalent_style("width", "0%", "0px"));
}
