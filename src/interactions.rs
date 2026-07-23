use crate::interactions_input::{click_matching, focused_path, submit_text_matching};
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
    element.getAttribute('placeholder') ||
    element.innerText ||
    element.value ||
    ''
  ).replace(/\s+/g, ' ').trim().slice(0, 120);
  const candidateVisible = element => {
    if (visible(element)) return true;
    if (!element.matches('button,[role="button"],[aria-haspopup]')) return false;
    const owner = element.closest('article,li,[role="button"]') ||
      element.parentElement;
    return owner ? visible(owner) : false;
  };
  const followsRepeatedContent = element => {
    let cursor = element;
    for (let depth = 0; cursor?.parentElement && depth < 4; depth++) {
      const siblings = Array.from(cursor.parentElement.children);
      for (const sibling of siblings.slice(0, siblings.indexOf(cursor)).reverse()) {
        const children = Array.from(sibling.children).filter(visible);
        const groups = new Map();
        for (const child of children) {
          const key = `${child.tagName}:${child.getAttribute('role') || ''}`;
          const group = groups.get(key) || [];
          group.push(child.getBoundingClientRect());
          groups.set(key, group);
        }
        for (const group of groups.values()) {
          if (group.length < 3) continue;
          const first = group[0];
          const sameSize = group.every(rect =>
            Math.abs(rect.width - first.width) <= 2 &&
            Math.abs(rect.height - first.height) <= 2
          );
          const distributed = group.some(rect =>
            Math.abs(rect.x - first.x) > 2 || Math.abs(rect.y - first.y) > 2
          );
          if (sameSize && distributed) return true;
        }
      }
      cursor = cursor.parentElement;
    }
    return false;
  };
  const localStateControl = element =>
    Array.from(element.parentElement?.children || []).some(sibling =>
      sibling !== element && (
        sibling.disabled || sibling.getAttribute('aria-disabled') === 'true'
      )
    ) || followsRepeatedContent(element);
  const controls = Array.from(document.querySelectorAll(
  'a[href],button,[role="button"],[role="tab"],[aria-haspopup],[aria-expanded],' +
  '[aria-pressed],[aria-selected],summary,' +
  'input:not([type="hidden"]),textarea,select'
  ));
  return controls.filter(element =>
    candidateVisible(element) &&
    !element.disabled &&
    element.getAttribute('aria-disabled') !== 'true' &&
    !(element.type === 'submit' && element.closest('form')) &&
    !element.closest('[contenteditable="true"]')
  ).map(element => {
   const rect = element.getBoundingClientRect();
   const label = labelOf(element);
    return {
    path: pathOf(element),
    tag: element.tagName.toLowerCase(),
    label,
    occurrence: controls.filter(candidate =>
      candidate.tagName === element.tagName && labelOf(candidate) === label
    ).indexOf(element),
    priority:
      ((element.matches('textarea,input[type="text"],input:not([type])') &&
        !element.matches('[role="combobox"],[aria-haspopup]')) ? 18 : 0) +
      (element.hasAttribute('aria-haspopup') ? 16 : 0) +
      (element.hasAttribute('aria-expanded') ? 12 : 0) +
      (element.getAttribute('role') === 'tab' ? 14 : 0) +
      (element.hasAttribute('aria-pressed') ? 14 : 0) +
      (element.hasAttribute('aria-selected') ? 10 : 0) +
      (element.hasAttribute('aria-label') && rect.top >= 0 && rect.bottom <= 80 ? 8 : 0) +
      (element.tagName === 'SUMMARY' ? 1 : 0),
    state_control: element.getAttribute('role') === 'tab' ||
      element.hasAttribute('aria-pressed') || element.hasAttribute('aria-selected')
  }}).filter(candidate => candidate.priority >= 8 ||
    ((candidate.tag === 'button' || candidate.tag === 'summary') &&
      localStateControl(document.querySelector(candidate.path))))
  .sort((a, b) => b.priority - a.priority);
})()
"#;

const PREFLIGHT: &str = r#"
(() => {
  let hash = 2166136261;
  const add = value => {
    const text = String(value);
    for (let index = 0; index < text.length; index++) {
      hash ^= text.charCodeAt(index);
      hash = Math.imul(hash, 16777619);
    }
  };
  add(`${document.documentElement.scrollWidth}:${document.documentElement.scrollHeight}`);
  for (const element of document.querySelectorAll('*')) {
    const rect = element.getBoundingClientRect();
    add(element.tagName);
    add(element.childElementCount);
    add(`${Math.round(rect.x * 2)}:${Math.round(rect.y * 2)}:${Math.round(rect.width * 2)}:${Math.round(rect.height * 2)}`);
    add(`${element.scrollLeft}:${element.scrollTop}`);
    for (const name of ['aria-expanded', 'aria-pressed', 'aria-selected', 'disabled', 'hidden']) {
      add(element.getAttribute(name) || '');
    }
  }
  return `${document.querySelectorAll('*').length}:${hash >>> 0}`;
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
    priority: u8,
    state_control: bool,
}

impl Candidate {
    fn uses_text_entry(&self) -> bool {
        crate::interactions_input::text_entry(&self.tag)
    }
}

pub async fn capture(cdp: &mut Cdp, baselines: &[PageState]) -> Result<Vec<Interaction>> {
    let Some(first) = baselines.first() else {
        return Ok(Vec::new());
    };
    let mut initial = Some(restore(cdp, first, false).await?);
    let candidates: Vec<Candidate> = serde_json::from_value(cdp.evaluate(CANDIDATES).await?)?;
    let mut interactions = Vec::new();
    for candidate in candidates {
        if candidate.occurrence > 0
            && let Some(shared) = interactions
                .iter_mut()
                .find(|interaction: &&mut Interaction| {
                    interaction
                        .trigger_label
                        .eq_ignore_ascii_case(&candidate.label)
                        && interaction.trigger_tag == candidate.tag
                        && surface_backed(interaction, baselines)
                })
        {
            shared.trigger_occurrence = None;
            continue;
        }
        let candidate_started = std::time::Instant::now();
        let reuse_page = true;
        let mut fresh = match initial.take() {
            Some(state) => state,
            None => restore(cdp, first, !reuse_page).await?,
        };
        let mut opened = None;
        for attempt in 0..2 {
            let before = if candidate.priority < 8 {
                Some(cdp.evaluate(PREFLIGHT).await?)
            } else {
                None
            };
            let clicked = activate(cdp, &candidate).await?;
            if !clicked {
                break;
            }
            let settled = settle(cdp, candidate.uses_text_entry()).await?;
            let unchanged = match before {
                Some(before) => cdp.evaluate(PREFLIGHT).await? == before,
                None => false,
            };
            if unchanged {
                break;
            }
            let focused = focused_path(cdp).await?;
            if cdp.evaluate("location.href").await?.as_str() != Some(first.url.as_str()) {
                break;
            }
            let state = capture::read_interaction_state(cdp, first.viewport.clone()).await?;
            if candidate.state_control
                || discovery_differs(&candidate.label, &candidate.path, &state, &fresh)
            {
                opened = Some((state, fresh, settled, focused));
                break;
            }
            if attempt == 0 {
                fresh = restore(cdp, first, !reuse_page).await?;
            }
        }
        let Some((mut state, fresh, settled, focused_path)) = opened else {
            continue;
        };
        close(cdp, &candidate).await?;
        crate::interaction_rebase::unchanged(&mut state, &fresh, first);
        interaction_state::compact(&mut state, &fresh, settled);
        let responsive = responsive_baselines(
            candidate.uses_text_entry(),
            candidate.state_control,
            &state,
            &fresh,
            &candidate.path,
            &candidate.label,
            baselines,
        );
        let mut states = vec![state];
        for baseline in responsive {
            let fresh = restore(cdp, baseline, !reuse_page).await?;
            if !activate(cdp, &candidate).await? {
                continue;
            }

            let settled = settle(cdp, candidate.uses_text_entry()).await?;
            let mut state = capture::read_interaction_state(cdp, baseline.viewport.clone()).await?;
            if discovery_differs(&candidate.label, &candidate.path, &state, baseline) {
                crate::interaction_rebase::unchanged(&mut state, &fresh, baseline);
                interaction_state::compact(&mut state, &fresh, settled);
                states.push(state);
                close(cdp, &candidate).await?;
            }
        }
        if candidate.state_control {
            initial = Some(restore(cdp, first, true).await?);
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
        let key = (
            interaction.trigger_path.clone(),
            interaction.trigger_tag.clone(),
            interaction.trigger_label.clone(),
        );
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

fn responsive_baselines<'a>(
    text_entry: bool,
    state_control: bool,
    state: &PageState,
    baseline: &PageState,
    trigger: &str,
    label: &str,
    baselines: &'a [PageState],
) -> Vec<&'a PageState> {
    if state_control
        || text_entry
        || interaction_state::surface_differs(state, baseline, trigger, label)
        || geometry_differs(state, baseline)
    {
        baselines.iter().skip(1).collect()
    } else {
        Vec::new()
    }
}

fn discovery_differs(label: &str, trigger: &str, left: &PageState, right: &PageState) -> bool {
    interaction_state::meaningfully_differs(left, right)
        || interaction_state::surface_differs(left, right, trigger, label)
        || geometry_differs(left, right)
}

fn geometry_differs(left: &PageState, right: &PageState) -> bool {
    left.nodes.iter().any(|node| {
        right
            .nodes
            .iter()
            .find(|baseline| baseline.path == node.path)
            .is_some_and(|baseline| {
                (node.rect.x - baseline.rect.x).abs() > 1.0
                    || (node.rect.y - baseline.rect.y).abs() > 1.0
            })
    })
}

fn surface_backed(interaction: &Interaction, baselines: &[PageState]) -> bool {
    interaction.states.iter().any(|state| {
        baselines
            .iter()
            .find(|baseline| baseline.viewport.width == state.viewport.width)
            .is_some_and(|baseline| !crate::interaction_surface::roots(state, baseline).is_empty())
    })
}

async fn activate(cdp: &mut Cdp, candidate: &Candidate) -> Result<bool> {
    if candidate.uses_text_entry() {
        submit_text_matching(
            cdp,
            &candidate.path,
            &candidate.tag,
            &candidate.label,
            Some(candidate.occurrence),
        )
        .await
    } else {
        click_matching(
            cdp,
            &candidate.path,
            &candidate.tag,
            &candidate.label,
            Some(candidate.occurrence),
            false,
        )
        .await
    }
}

async fn close(cdp: &mut Cdp, candidate: &Candidate) -> Result<()> {
    if candidate.uses_text_entry() {
        return Ok(());
    }
    if activate(cdp, candidate).await? {
        let _ = settle(cdp, false).await?;
    }
    Ok(())
}

async fn settle(cdp: &mut Cdp, text_entry: bool) -> Result<bool> {
    let timeout = if text_entry { 1_500 } else { 500 };
    let source = SETTLE.replace(">= 500", &format!(">= {timeout}"));
    Ok(cdp.evaluate(&source).await?.as_bool() == Some(true))
}

async fn restore(cdp: &mut Cdp, baseline: &PageState, reload: bool) -> Result<PageState> {
    let same_url = cdp.evaluate("location.href").await?.as_str() == Some(baseline.url.as_str());
    if same_url && !reload {
        cdp.evaluate(
            "document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));\
             for(const element of document.querySelectorAll('[aria-expanded=\"true\"]')){\
               element.click()\
             }\
             for(const element of document.querySelectorAll('*')){\
               if(element.scrollLeft||element.scrollTop)element.scrollTo(0,0)\
             }scrollTo(0,0);document.activeElement?.blur()",
        )
        .await?;
        browser::set_viewport(cdp, baseline.viewport.width, baseline.viewport.height).await?;
        wait_frames(cdp).await?;
        let restored = capture::read_interaction_state(cdp, baseline.viewport.clone()).await?;
        if !restoration_requires_reload(&restored, baseline) {
            return Ok(restored);
        }
    }
    if same_url {
        capture::prepare_interaction_state(cdp, &baseline.viewport, true).await?;
    } else {
        cdp.send("Page.navigate", serde_json::json!({"url":baseline.url}))
            .await?;
        let _ = capture::capture_state(cdp, baseline.viewport.clone(), false).await?;
    }
    cdp.evaluate("scrollTo(0,0)").await?;
    wait_frames(cdp).await?;
    capture::read_interaction_state(cdp, baseline.viewport.clone()).await
}

async fn wait_frames(cdp: &mut Cdp) -> Result<()> {
    cdp.evaluate("new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))")
        .await?;
    Ok(())
}

fn restoration_requires_reload(restored: &PageState, baseline: &PageState) -> bool {
    interaction_state::selected_differs(restored, baseline)
        || interaction_state::surface_differs(restored, baseline, "", "")
        || geometry_differs(restored, baseline)
}

#[cfg(test)]
mod tests {
    use super::{deduplicate, restoration_requires_reload};
    use crate::model::Interaction;

    #[test]
    fn repeated_controls_keep_independent_bindings() {
        assert!(!super::CANDIDATES.contains("values.findIndex"));
        assert!(super::CANDIDATES.contains("candidate.priority >= 8"));
        assert!(super::CANDIDATES.contains("candidateVisible(element)"));
        assert!(super::CANDIDATES.contains("article,li,[role=\"button\"]"));
        assert!(super::CANDIDATES.contains("followsRepeatedContent"));
        assert!(super::CANDIDATES.contains("sameSize && distributed"));
        assert!(super::PREFLIGHT.contains("getBoundingClientRect"));
    }

    #[test]
    fn interactions_capture_every_recorded_viewport() {
        let mut baselines = Vec::new();
        for width in [1920, 1440, 768, 390, 320] {
            let mut state = crate::model::PageState {
                url: String::new(),
                title: String::new(),
                viewport: crate::model::Viewport::default(),
                dom: Default::default(),
                capture_blockers: Vec::new(),
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

        let baseline = baselines[0].clone();
        let state = baseline.clone();
        assert!(
            super::responsive_baselines(
                false, false, &state, &baseline, "button", "Action", &baselines
            )
            .is_empty()
        );
        assert_eq!(
            super::responsive_baselines(
                true, false, &state, &baseline, "textarea", "Prompt", &baselines
            )
            .len(),
            4
        );
        assert_eq!(
            super::responsive_baselines(
                false, true, &state, &baseline, "button", "Activity", &baselines
            )
            .len(),
            4
        );
    }

    #[test]
    fn discovers_text_entry_controls() {
        assert!(super::CANDIDATES.contains("input:not([type=\"hidden\"])"));
        assert!(super::CANDIDATES.contains("textarea,select"));
        assert!(super::CANDIDATES.contains("? 18 : 0"));
        assert!(super::CANDIDATES.contains("element.getAttribute('role') === 'tab'"));
        assert!(super::CANDIDATES.contains("element.hasAttribute('aria-pressed')"));
    }

    #[test]
    fn incomplete_interaction_teardown_requires_reload() {
        let baseline = state_with_paths(&["html>body", "html>body>main"]);
        let restored = state_with_paths(&["html>body", "html>body>search"]);
        assert!(restoration_requires_reload(&restored, &baseline));
        let mut shifted = baseline.clone();
        shifted.nodes[1].rect.y = 48.0;
        assert!(restoration_requires_reload(&shifted, &baseline));
        assert!(!restoration_requires_reload(&baseline, &baseline));
    }

    #[test]
    fn selective_baseline_does_not_force_a_reload() {
        let baseline = state_with_paths(&["html", "html>body", "html>body>main"]);
        let restored = state_with_paths(&["html", "html>body"]);
        assert!(!restoration_requires_reload(&restored, &baseline));
    }

    #[test]
    fn unrelated_rotating_text_does_not_fake_a_surface() {
        let baseline = state_with_paths(&["html>body", "html>body>prompt"]);
        let mut changed = baseline.clone();
        changed.nodes[1].text = "A different rotating prompt".into();

        assert!(!super::discovery_differs(
            "Open actions",
            "html>body>trigger",
            &changed,
            &baseline,
        ));
    }

    #[test]
    fn duplicate_path_evidence_keeps_the_richest_state() {
        let mut sparse = Interaction {
            trigger_path: "card>button".into(),
            trigger_tag: "button".into(),
            trigger_label: "Open actions".into(),
            trigger_occurrence: None,
            focused_path: None,
            states: vec![crate::model::PageState {
                url: String::new(),
                title: String::new(),
                viewport: crate::model::Viewport::default(),
                dom: Default::default(),
                capture_blockers: Vec::new(),
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
        rich.trigger_occurrence = Some(7);
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
        assert_eq!(values[0].trigger_path, "card>button");
        assert_eq!(values[0].trigger_occurrence, Some(7));
        assert_eq!(values[0].states[0].nodes.len(), 1);
    }

    fn state_with_paths(paths: &[&str]) -> crate::model::PageState {
        let mut state = crate::model::PageState {
            url: String::new(),
            title: String::new(),
            viewport: crate::model::Viewport::default(),
            dom: Default::default(),
            capture_blockers: Vec::new(),
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
        state.nodes = paths
            .iter()
            .map(|path| crate::model::Node {
                path: (*path).into(),
                parent: None,
                tag: "div".into(),
                text: String::new(),
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
            })
            .collect();
        state
    }
}
