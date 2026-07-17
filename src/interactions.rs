use crate::interactions_input::{click_matching, focused_path};
use crate::{
    capture,
    cdp::Cdp,
    interaction_state,
    model::{Interaction, PageState},
};
use anyhow::Result;
use serde::Deserialize;

const CANDIDATES: &str = r#"
(() => {
  const pathOf = element => {
    const parts = [];
    for (let node = element; node && node !== document.documentElement; node = node.parentElement) {
      const peers = node.parentElement
        ? Array.from(node.parentElement.children).filter(child => child.tagName === node.tagName)
        : [node];
      parts.push(`${node.tagName.toLowerCase()}:nth-of-type(${peers.indexOf(node) + 1})`);
    }
    return `html>${parts.reverse().join('>')}`;
  };
  const visible = element => {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' &&
      style.visibility !== 'hidden';
  };
  return Array.from(document.querySelectorAll(
    'button,[role="button"],[aria-haspopup],[aria-expanded],summary,input[type="checkbox"],input[type="radio"]'
  )).filter(element =>
    visible(element) &&
    !element.disabled &&
    element.getAttribute('aria-disabled') !== 'true' &&
    !(element.type === 'submit' && element.closest('form')) &&
    !element.closest('[contenteditable="true"]')
  ).map(element => {
    const label = (
      element.getAttribute('aria-label') ||
      element.innerText ||
      element.value ||
      ''
    ).replace(/\s+/g, ' ').trim().slice(0, 120);
    return {
    path: pathOf(element),
    tag: element.tagName.toLowerCase(),
    label,
    priority:
      (/^(search|open account menu|app launcher)$/i.test(label) ? 20 : 0) +
      (/^more options$/i.test(label) ? 16 : 0) +
      (/^more tasks$/i.test(label) ? 12 : 0) +
      (/^(upload sources|use voice)$/i.test(label) ? 4 : 0) +
      (element.hasAttribute('aria-haspopup') ? 8 : 0) +
      (element.hasAttribute('aria-expanded') ? 4 : 0) +
      (element.tagName === 'SUMMARY' ? 1 : 0)
  }}).filter(candidate => candidate.priority > 0 || candidate.tag === 'input')
  .sort((a, b) => b.priority - a.priority)
  .filter((candidate, index, values) =>
    values.findIndex(value => value.label === candidate.label) === index
  ).slice(0, 16);
})()
"#;

const SETTLE: &str = r#"
new Promise(resolve => {
  const started = performance.now();
  let cleanFrames = 0;
  const observer = new MutationObserver(() => { cleanFrames = 0; });
  observer.observe(document, {
    attributes: true, childList: true, characterData: true, subtree: true
  });
  const sample = () => {
    const running = document.getAnimations({ subtree: true })
      .some(animation => animation.playState === 'running');
    const pending = (window.__recreatePendingRequests || 0) > 0;
    cleanFrames = running || pending ? 0 : cleanFrames + 1;
    if (cleanFrames >= 2 || performance.now() - started >= 500) {
      observer.disconnect();
      resolve(cleanFrames >= 2);
    } else {
      requestAnimationFrame(sample);
    }
  };
  requestAnimationFrame(sample);
})
"#;

#[derive(Deserialize)]
struct Candidate {
    path: String,
    tag: String,
    label: String,
}

pub async fn capture(cdp: &mut Cdp, baselines: &[PageState]) -> Result<Vec<Interaction>> {
    let Some(first) = baselines.first() else {
        return Ok(Vec::new());
    };
    let mut initial = Some(restore(cdp, &first.viewport, &first.url).await?);
    let candidates: Vec<Candidate> = serde_json::from_value(cdp.evaluate(CANDIDATES).await?)?;
    let mut interactions = Vec::new();
    for candidate in candidates {
        let mut fresh = match initial.take() {
            Some(state) => state,
            None => restore(cdp, &first.viewport, &first.url).await?,
        };
        let mut opened = None;
        for attempt in 0..2 {
            if !click_matching(
                cdp,
                &candidate.path,
                &candidate.tag,
                &candidate.label,
                false,
            )
            .await?
            {
                break;
            }
            let settled = settle(cdp).await?;
            let focused = focused_path(cdp).await?;
            if cdp.evaluate("location.href").await?.as_str() != Some(first.url.as_str()) {
                break;
            }
            let state = capture::read_state(cdp, first.viewport.clone()).await?;
            if discovery_differs(&candidate.label, &state, &fresh) {
                opened = Some((state, fresh, settled, focused));
                break;
            }
            if attempt == 0 {
                fresh = restore(cdp, &first.viewport, &first.url).await?;
            }
        }
        let Some((mut state, fresh, settled, focused_path)) = opened else {
            continue;
        };
        interaction_state::compact(&mut state, &fresh, settled);
        let mut states = vec![state];
        for baseline in baselines.iter().skip(1) {
            let fresh = restore(cdp, &baseline.viewport, &baseline.url).await?;
            if !click_matching(
                cdp,
                &candidate.path,
                &candidate.tag,
                &candidate.label,
                false,
            )
            .await?
            {
                continue;
            }
            let settled = settle(cdp).await?;
            let mut state = capture::read_state(cdp, baseline.viewport.clone()).await?;
            if discovery_differs(&candidate.label, &state, baseline) {
                interaction_state::compact(&mut state, &fresh, settled);
                states.push(state);
            }
        }
        interactions.push(Interaction {
            trigger_path: candidate.path,
            trigger_tag: candidate.tag,
            trigger_label: candidate.label,
            focused_path,
            states,
        });
    }
    Ok(interactions)
}

fn discovery_differs(label: &str, left: &PageState, right: &PageState) -> bool {
    if label.eq_ignore_ascii_case("More tasks") {
        interaction_state::content_differs(left, right)
    } else {
        interaction_state::meaningfully_differs(left, right)
    }
}

async fn settle(cdp: &mut Cdp) -> Result<bool> {
    Ok(cdp.evaluate(SETTLE).await?.as_bool() == Some(true))
}

async fn restore(cdp: &mut Cdp, viewport: &crate::model::Viewport, url: &str) -> Result<PageState> {
    if cdp.evaluate("location.href").await?.as_str() == Some(url) {
        cdp.send("Page.reload", serde_json::json!({"ignoreCache":false}))
            .await?;
    } else {
        cdp.send("Page.navigate", serde_json::json!({"url":url}))
            .await?;
    }
    let _ = capture::capture_state(cdp, viewport.clone(), false).await?;
    cdp.evaluate("scrollTo(0,0)").await?;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    capture::read_state(cdp, viewport.clone()).await
}
