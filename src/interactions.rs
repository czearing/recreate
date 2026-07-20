use crate::interactions_input::{click_matching, focused_path};
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
      style.visibility !== 'hidden';
  };
  const labelOf = element => (
    element.getAttribute('aria-label') ||
    element.innerText ||
    element.value ||
    ''
  ).replace(/\s+/g, ' ').trim().slice(0, 120);
  const controls = Array.from(document.querySelectorAll(
    'button,[role="button"],[aria-haspopup],[aria-expanded],summary,input[type="checkbox"],input[type="radio"]'
  ));
  return controls.filter(element =>
    visible(element) &&
    !element.disabled &&
    element.getAttribute('aria-disabled') !== 'true' &&
    !(element.type === 'submit' && element.closest('form')) &&
    !element.closest('[contenteditable="true"]')
  ).map(element => {
    const label = labelOf(element);
    return {
    path: pathOf(element),
    tag: element.tagName.toLowerCase(),
    label,
    occurrence: controls.filter(candidate =>
      candidate.tagName === element.tagName && labelOf(candidate) === label
    ).indexOf(element),
    priority:
      (/^(search|open account menu|app launcher)$/i.test(label) ? 20 : 0) +
      (/^more options$/i.test(label) ? 16 : 0) +
      (/^more tasks$/i.test(label) ? 12 : 0) +
      (/^(upload sources|use voice)$/i.test(label) ? 4 : 0) +
      (element.hasAttribute('aria-haspopup') ? 8 : 0) +
      (element.hasAttribute('aria-expanded') ? 4 : 0) +
      (element.tagName === 'SUMMARY' ? 1 : 0)
  }}).filter(candidate =>
    candidate.priority >= 16 || /^more tasks$/i.test(candidate.label)
  )
  .sort((a, b) => b.priority - a.priority)
  .filter((candidate, index, values) =>
    values.findIndex(value => value.label === candidate.label) === index
  );
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
    occurrence: usize,
}

pub async fn capture(cdp: &mut Cdp, baselines: &[PageState]) -> Result<Vec<Interaction>> {
    let Some(first) = baselines.first() else {
        return Ok(Vec::new());
    };
    let mut initial = Some(restore(cdp, &first.viewport, &first.url, false).await?);
    let candidates: Vec<Candidate> = serde_json::from_value(cdp.evaluate(CANDIDATES).await?)?;
    let mut interactions = Vec::new();
    for candidate in candidates {
        let candidate_started = std::time::Instant::now();
        let reuse_page = true;
        let mut fresh = match initial.take() {
            Some(state) => state,
            None => restore(cdp, &first.viewport, &first.url, !reuse_page).await?,
        };
        let mut opened = None;
        for attempt in 0..2 {
            let clicked = click_matching(
                cdp,
                &candidate.path,
                &candidate.tag,
                &candidate.label,
                Some(candidate.occurrence),
                false,
            )
            .await?;
            if !clicked {
                break;
            }
            let settled = settle(cdp).await?;
            let focused = focused_path(cdp).await?;
            if cdp.evaluate("location.href").await?.as_str() != Some(first.url.as_str()) {
                break;
            }
            let state = capture::read_interaction_state(cdp, first.viewport.clone()).await?;
            if captures_visual_state(&candidate.label)
                || discovery_differs(&candidate.label, &candidate.path, &state, &fresh)
            {
                opened = Some((state, fresh, settled, focused));
                break;
            }
            if attempt == 0 {
                fresh = restore(cdp, &first.viewport, &first.url, !reuse_page).await?;
            }
        }
        let Some((mut state, fresh, settled, focused_path)) = opened else {
            continue;
        };
        interaction_state::compact(&mut state, &fresh, settled);
        let mut states = vec![state];
        for baseline in responsive_baselines(&candidate.label, baselines) {
            let fresh = restore(cdp, &baseline.viewport, &baseline.url, !reuse_page).await?;
            if !click_matching(
                cdp,
                &candidate.path,
                &candidate.tag,
                &candidate.label,
                Some(candidate.occurrence),
                false,
            )
            .await?
            {
                continue;
            }

            let settled = settle(cdp).await?;
            let mut state = capture::read_interaction_state(cdp, baseline.viewport.clone()).await?;
            if discovery_differs(&candidate.label, &candidate.path, &state, baseline) {
                interaction_state::compact(&mut state, &fresh, settled);
                states.push(state);
            }
        }
        eprintln!(
            "captured interaction {:?} in {:.2}s",
            candidate.label,
            candidate_started.elapsed().as_secs_f64()
        );
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

fn captures_visual_state(label: &str) -> bool {
    label.eq_ignore_ascii_case("More options") || label.eq_ignore_ascii_case("More tasks")
}

fn responsive_baselines<'a>(_label: &str, _baselines: &'a [PageState]) -> Vec<&'a PageState> {
    Vec::new()
}

fn discovery_differs(label: &str, trigger: &str, left: &PageState, right: &PageState) -> bool {
    if label.eq_ignore_ascii_case("More tasks") {
        interaction_state::content_differs(left, right)
    } else {
        interaction_state::meaningfully_differs(left, right)
            || interaction_state::surface_differs(left, right, trigger, label)
    }
}

async fn settle(cdp: &mut Cdp) -> Result<bool> {
    Ok(cdp.evaluate(SETTLE).await?.as_bool() == Some(true))
}

async fn restore(
    cdp: &mut Cdp,
    viewport: &crate::model::Viewport,
    url: &str,
    reload: bool,
) -> Result<PageState> {
    let same_url = cdp.evaluate("location.href").await?.as_str() == Some(url);
    if same_url && !reload {
        cdp.evaluate(
            "document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));\
             for(const element of document.querySelectorAll('*')){\
               if(element.scrollLeft||element.scrollTop)element.scrollTo(0,0)\
             }scrollTo(0,0);document.activeElement?.blur()",
        )
        .await?;
        browser::set_viewport(cdp, viewport.width, viewport.height).await?;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        return capture::read_interaction_state(cdp, viewport.clone()).await;
    }
    if same_url {
        capture::prepare_state(cdp, viewport, true).await?;
    } else {
        cdp.send("Page.navigate", serde_json::json!({"url":url}))
            .await?;
        let _ = capture::capture_state(cdp, viewport.clone(), false).await?;
    }
    cdp.evaluate("scrollTo(0,0)").await?;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    capture::read_interaction_state(cdp, viewport.clone()).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn repeated_controls_share_one_semantic_capture() {
        assert!(!super::CANDIDATES.contains("candidate.label.toLowerCase() === 'more options'"));
        assert!(
            super::CANDIDATES
                .contains("values.findIndex(value => value.label === candidate.label) === index")
        );
        assert!(super::CANDIDATES.contains("candidate.priority >= 16"));
    }

    #[test]
    fn interactions_capture_once_and_anchor_responsively_at_runtime() {
        let mut baselines = Vec::new();
        for width in [1920, 1440, 768, 390, 320] {
            let mut state = crate::model::PageState {
                url: String::new(),
                title: String::new(),
                viewport: crate::model::Viewport::default(),
                nodes: Vec::new(),
                startup_nodes: Vec::new(),
                startup_delay_ms: 0,
                startup_duration_ms: 0,
                animations: Vec::new(),
                state_styles: Vec::new(),
                attribute_sequences: Vec::new(),
                css_rules: Vec::new(),
                asset_urls: Vec::new(),
                asset_data: Default::default(),
            };
            state.viewport.width = width;
            baselines.push(state);
        }
        assert!(super::responsive_baselines("Open account menu", &baselines).is_empty());
        assert!(super::responsive_baselines("More tasks", &baselines).is_empty());
    }

    #[test]
    fn visual_controls_do_not_retry_semantic_discovery() {
        assert!(super::captures_visual_state("More options"));
        assert!(super::captures_visual_state("More tasks"));
        assert!(!super::captures_visual_state("Search"));
    }
}
