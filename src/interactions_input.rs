use crate::cdp::Cdp;
use anyhow::Result;

pub fn text_entry(tag: &str) -> bool {
    tag.eq_ignore_ascii_case("textarea") || tag.eq_ignore_ascii_case("input")
}

const FOCUSED: &str = r#"(() => {
__NODE_PATH__
  const element = document.activeElement;
  if (!element || element === document.body) return null;
  return pathOf(element);
})()"#;

/// The focused-element reader, with the shared path definition spliced in.
pub fn focused_script() -> String {
    crate::node_path::embed(FOCUSED)
}

pub async fn focused_path(cdp: &mut Cdp) -> Result<Option<String>> {
    let value = cdp.evaluate(&focused_script()).await?;
    Ok(value.as_str().map(str::to_string))
}

pub async fn click_matching(
    cdp: &mut Cdp,
    path: &str,
    tag: &str,
    label: &str,
    occurrence: Option<usize>,
    require_control: bool,
) -> Result<bool> {
    let Some(position) = aim_matching(cdp, path, tag, label, occurrence, require_control).await?
    else {
        return Ok(false);
    };
    press(cdp, path, position).await?;
    Ok(true)
}

/// Points at the element and focuses it without activating it.
///
/// Focus is a consequence of pointing, not a behaviour of the page, so it has to land before
/// the evidence for an activation is measured. Measuring across the focus change instead makes
/// every focusable control look like it did something, because a focus ring is a style change.
pub async fn aim_matching(
    cdp: &mut Cdp,
    path: &str,
    tag: &str,
    label: &str,
    occurrence: Option<usize>,
    require_control: bool,
) -> Result<Option<(f64, f64)>> {
    let (matching, fallback) = if tag.is_empty() {
        ("candidate=>candidate".into(), "null".into())
    } else {
        let tag = serde_json::to_string(tag)?;
        let label = serde_json::to_string(label)?;
        let control = if require_control {
            "candidate.hasAttribute('data-recreate-control')&&"
        } else {
            ""
        };
        let fallback = occurrence.map_or_else(
            || format!("Array.from(document.querySelectorAll({tag})).find(matches)"),
            |index| {
                format!("Array.from(document.querySelectorAll({tag})).filter(matches)[{index}]")
            },
        );
        (
            format!(
                "candidate=>candidate&&{control}candidate.tagName.toLowerCase()==={tag}&&\
                 (candidate.getAttribute('aria-label')||candidate.innerText||candidate.value||'')\
                 .replace(/\\s+/g,' ').trim()==={label}"
            ),
            fallback,
        )
    };
    let expression =
        super::interactions_approach::aim(&serde_json::to_string(path)?, &matching, &fallback);
    let position = cdp.evaluate(&expression).await?;
    let Some(position) = position.as_array() else {
        return Ok(None);
    };
    let (Some(x), Some(y)) = (
        position.first().and_then(serde_json::Value::as_f64),
        position.get(1).and_then(serde_json::Value::as_f64),
    ) else {
        return Ok(None);
    };
    Ok(Some((x, y)))
}

pub async fn press(cdp: &mut Cdp, path: &str, (x, y): (f64, f64)) -> Result<()> {
    for event_type in ["mouseMoved", "mousePressed", "mouseReleased"] {
        let mut params = serde_json::json!({"type":event_type,"x":x,"y":y});
        if event_type != "mouseMoved" {
            params["button"] = serde_json::json!("left");
            params["clickCount"] = serde_json::json!(1);
        }

        cdp.send("Input.dispatchMouseEvent", params).await?;
    }
    cdp.evaluate(&format!(
        "document.querySelector({})?.removeAttribute('data-recreate-preserve-scroll')",
        serde_json::to_string(path)?
    ))
    .await?;
    Ok(())
}

pub async fn submit_text_matching(
    cdp: &mut Cdp,
    path: &str,
    tag: &str,
    label: &str,
    occurrence: Option<usize>,
) -> Result<bool> {
    let tag_json = serde_json::to_string(tag)?;
    let label_json = serde_json::to_string(label)?;
    let fallback = occurrence.map_or_else(
        || format!("Array.from(document.querySelectorAll({tag_json})).find(matches)"),
        |index| {
            format!("Array.from(document.querySelectorAll({tag_json})).filter(matches)[{index}]")
        },
    );
    let expression = super::interactions_approach::text_entry(
        &serde_json::to_string(path)?,
        &tag_json,
        &label_json,
        &fallback,
    );
    Ok(cdp.evaluate(&expression).await?.as_bool() == Some(true))
}
