use crate::{
    browser, capture,
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
      style.visibility !== 'hidden' && Number(style.opacity || 1) > 0;
  };
  return Array.from(document.querySelectorAll(
    'button,[role="button"],[aria-haspopup],[aria-expanded],summary,input[type="checkbox"],input[type="radio"]'
  )).filter(element =>
    visible(element) &&
    !element.disabled &&
    element.getAttribute('aria-disabled') !== 'true' &&
    element.type !== 'submit' &&
    !element.closest('form,[contenteditable="true"]')
  ).map(element => ({
    path: pathOf(element),
    tag: element.tagName.toLowerCase(),
    label: (
      element.getAttribute('aria-label') ||
      element.innerText ||
      element.value ||
      ''
    ).replace(/\s+/g, ' ').trim().slice(0, 120),
    priority:
      (element.hasAttribute('aria-haspopup') ? 4 : 0) +
      (element.hasAttribute('aria-expanded') ? 2 : 0) +
      (element.tagName === 'SUMMARY' ? 1 : 0)
  })).filter(candidate =>
    candidate.priority > 0 || candidate.tag === 'input'
  ).sort((a, b) => b.priority - a.priority).slice(0, 16);
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
    let mut restored = Some(restore(cdp, &first.viewport).await?);
    let candidates: Vec<Candidate> = serde_json::from_value(cdp.evaluate(CANDIDATES).await?)?;
    let mut interactions = Vec::new();
    for (index, candidate) in candidates.into_iter().enumerate() {
        let fresh = if index == 0 {
            restored.take().expect("first restored state")
        } else {
            restore(cdp, &first.viewport).await?
        };
        let mut fresh_baselines = vec![fresh];
        for baseline in baselines.iter().skip(1) {
            browser::set_viewport(cdp, baseline.viewport.width, baseline.viewport.height).await?;
            settle(cdp).await?;
            fresh_baselines.push(capture::read_state(cdp, baseline.viewport.clone()).await?);
        }
        browser::set_viewport(cdp, first.viewport.width, first.viewport.height).await?;
        settle(cdp).await?;
        if !click(cdp, &candidate.path).await? {
            continue;
        }
        settle(cdp).await?;
        let focused_path = focused_path(cdp).await?;
        let mut states = Vec::new();
        for (index, baseline) in baselines.iter().enumerate() {
            browser::set_viewport(cdp, baseline.viewport.width, baseline.viewport.height).await?;
            let settled = settle(cdp).await?;
            if cdp.evaluate("location.href").await?.as_str() != Some(first.url.as_str()) {
                continue;
            }
            let mut state = capture::read_state(cdp, baseline.viewport.clone()).await?;
            let fresh = &fresh_baselines[index];
            if interaction_state::differs(&state, fresh) {
                interaction_state::compact(&mut state, fresh, settled);
                states.push(state);
            }
        }
        if !states.is_empty() {
            interactions.push(Interaction {
                trigger_path: candidate.path,
                trigger_tag: candidate.tag,
                trigger_label: candidate.label,
                focused_path,
                states,
            });
        }
    }
    Ok(interactions)
}

async fn settle(cdp: &mut Cdp) -> Result<bool> {
    Ok(cdp.evaluate(SETTLE).await?.as_bool() == Some(true))
}

async fn focused_path(cdp: &mut Cdp) -> Result<Option<String>> {
    let value = cdp
        .evaluate(
            r#"(() => {
              const element = document.activeElement;
              if (!element || element === document.body) return null;
              const parts = [];
              for (let node = element; node && node !== document.documentElement; node = node.parentElement) {
                const peers = node.parentElement
                  ? Array.from(node.parentElement.children).filter(child => child.tagName === node.tagName)
                  : [node];
                parts.push(`${node.tagName.toLowerCase()}:nth-of-type(${peers.indexOf(node) + 1})`);
              }
              return `html>${parts.reverse().join('>')}`;
            })()"#,
        )
        .await?;
    Ok(value.as_str().map(str::to_string))
}

async fn click(cdp: &mut Cdp, path: &str) -> Result<bool> {
    let expression = format!(
        "(() => {{ const element=document.querySelector({}); if(!element)return false; \
         element.focus({{preventScroll:true}}); element.click(); return true; }})()",
        serde_json::to_string(path)?
    );
    Ok(cdp.evaluate(&expression).await?.as_bool() == Some(true))
}

async fn restore(cdp: &mut Cdp, viewport: &crate::model::Viewport) -> Result<PageState> {
    capture::capture_state(cdp, viewport.clone(), true).await
}
