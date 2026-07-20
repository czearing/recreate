use crate::interactions_input::{click_matching, focused_path};
use crate::{
    browser, capture,
    cdp::Cdp,
    interaction_state,
    model::{Interaction, PageState},
};
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

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
  const candidateVisible = element => {
    if (visible(element)) return true;
    if (!/^more options$/i.test(labelOf(element))) return false;
    const owner = element.closest('[data-testid="notebook-card"],[role="button"]') ||
      element.parentElement;
    return owner ? visible(owner) : false;
  };
  const controls = Array.from(document.querySelectorAll(
    'button,[role="button"],[aria-haspopup],[aria-expanded],summary,input[type="checkbox"],input[type="radio"]'
  ));
  return controls.filter(element =>
    candidateVisible(element) &&
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
  .sort((a, b) => b.priority - a.priority);
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
        if candidate.label.eq_ignore_ascii_case("More options")
            && let Some(shared) = interactions.iter().find(|interaction: &&Interaction| {
                interaction
                    .trigger_label
                    .eq_ignore_ascii_case(&candidate.label)
                    && interaction.trigger_tag == candidate.tag
            })
        {
            let mut repeated = (*shared).clone();
            repeated.trigger_path = candidate.path;
            repeated.trigger_occurrence = Some(candidate.occurrence);
            interactions.push(repeated);
            continue;
        }
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
            trigger_occurrence: Some(candidate.occurrence),
            focused_path,
            states,
        });
    }
    Ok(interactions)
}

pub fn deduplicate(interactions: &mut Vec<Interaction>) {
    let mut positions = HashMap::<(String, String, String), usize>::new();
    let mut unique: Vec<Interaction> = Vec::new();
    for interaction in interactions.drain(..) {
        let key = if interaction
            .trigger_label
            .eq_ignore_ascii_case("More options")
        {
            (
                String::new(),
                interaction.trigger_tag.clone(),
                interaction.trigger_label.to_ascii_lowercase(),
            )
        } else {
            (
                interaction.trigger_path.clone(),
                interaction.trigger_tag.clone(),
                interaction.trigger_label.clone(),
            )
        };
        if let Some(index) = positions.get(&key).copied() {
            if evidence_score(&interaction) > evidence_score(&unique[index]) {
                unique[index] = interaction;
            }
        } else {
            positions.insert(key, unique.len());
            unique.push(interaction);
        }
    }
    *interactions = unique;
}

fn evidence_score(interaction: &Interaction) -> usize {
    interaction
        .states
        .iter()
        .map(|state| state.nodes.len())
        .sum()
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
    use super::deduplicate;
    use crate::model::Interaction;

    #[test]
    fn repeated_controls_keep_independent_bindings() {
        assert!(!super::CANDIDATES.contains("values.findIndex"));
        assert!(super::CANDIDATES.contains("candidate.priority >= 16"));
        assert!(super::CANDIDATES.contains("candidateVisible(element)"));
        assert!(super::CANDIDATES.contains("[data-testid=\"notebook-card\"]"));
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

    #[test]
    fn duplicate_trigger_evidence_keeps_the_richest_state() {
        let mut sparse = Interaction {
            trigger_path: "card>button".into(),
            trigger_tag: "button".into(),
            trigger_label: "More options".into(),
            trigger_occurrence: None,
            focused_path: None,
            states: vec![crate::model::PageState {
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
            }],
        };
        let mut rich = sparse.clone();
        rich.trigger_path = "other-card>button".into();
        rich.states[0].nodes.push(crate::model::Node {
            path: "menu".into(),
            parent: None,
            tag: "div".into(),
            text: "Pin".into(),
            attributes: Default::default(),
            rect: crate::model::Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            style: Default::default(),
            before: None,
            after: None,
        });
        sparse.focused_path = Some("sparse".into());
        let mut values = vec![sparse, rich];
        deduplicate(&mut values);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].trigger_path, "other-card>button");
        assert_eq!(values[0].states[0].nodes.len(), 1);
    }
}
