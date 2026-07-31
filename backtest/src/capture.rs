use crate::{
    browser,
    cdp::Cdp,
    deadline::Deadline,
    digest, instrumentation,
    model::{
        Action, Artifact, LayoutShiftEvidence, NodeEvidence, RasterTileEvidence, RuntimeEvidence,
        SCHEMA_VERSION, Session, SourceIdentity, State, Viewport,
    },
};
use anyhow::Context;
use base64::Engine;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

const EVIDENCE_SCRIPT: &str = r##"
(() => {
  const pathCache = new WeakMap([[document.documentElement, "html"]]);
  const pathOf = (element) => {
    if (!element) return "";
    const authored = element.getAttribute?.("data-backtest-id");
    if (authored) return authored;
    if (pathCache.has(element)) return pathCache.get(element);
    const parent = element.parentElement;
    const peers = parent
      ? Array.from(parent.children).filter((value) => value.tagName === element.tagName)
      : [element];
    const path = `${pathOf(parent)}>${element.tagName.toLowerCase()}:nth-of-type(${peers.indexOf(element) + 1})`;
    pathCache.set(element, path);
    return path;
  };
  const color = (value) => {
    const match = String(value).match(/^rgba?\((\d+),\s*(\d+),\s*(\d+)/);
    return match ? "#" + match.slice(1, 4).map((part) => Number(part).toString(16).padStart(2, "0")).join("") : String(value);
  };
  const directText = (element) => Array.from(element.childNodes)
    .filter((node) => node.nodeType === Node.TEXT_NODE)
    .map((node) => node.textContent)
    .join(" ")
    .replace(/\s+/g, " ")
    .trim();
  const nodes = {};
  const allElements = Array.from(document.querySelectorAll("*"));
  if (allElements.length > 5000) {
    throw new Error(`capture node limit exceeded: ${allElements.length}`);
  }
  const eligible = allElements
    .filter((element) => !element.closest("head"))
    .filter((element) => !["SCRIPT", "STYLE", "NOSCRIPT"].includes(element.tagName));
  const elements = eligible;
  for (const element of elements) {
    const id = pathOf(element);
    if (Object.prototype.hasOwnProperty.call(nodes, id)) {
      throw new Error(`duplicate capture target: ${id}`);
    }
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    const duration = style.animationName !== "none"
      ? Math.round((parseFloat(style.animationDuration) || 0) * (style.animationDuration.includes("ms") ? 1 : 1000))
      : null;
    const before = getComputedStyle(element, "::before");
    const after = getComputedStyle(element, "::after");
    const hasPseudoPaint = [before, after].some((pseudo) =>
      pseudo.content !== "none" &&
      pseudo.content !== "normal" &&
      pseudo.display !== "none" &&
      pseudo.visibility !== "hidden"
    );
    const rasterKind = element.tagName === "CANVAS"
      ? "canvas-content"
      : element.tagName === "VIDEO"
        ? "video-content"
        : style.backgroundImage !== "none"
          ? "background-content"
          : hasPseudoPaint
            ? "pseudo-content"
            : style.boxShadow !== "none" ||
                style.filter !== "none" ||
                style.backdropFilter !== "none" ||
                style.maskImage !== "none"
              ? "effect-content"
              : "";
    nodes[id] = {
      tag: element.tagName.toLowerCase(),
      parent: pathOf(element.parentElement),
      order: Math.max(0, Array.from(element.parentElement?.children || []).indexOf(element)),
      text: directText(element),
      visible: style.display !== "none" && style.visibility !== "hidden",
      x: Math.round(rect.x * 100) / 100,
      y: Math.round(rect.y * 100) / 100,
      width: Math.round(rect.width * 100) / 100,
      height: Math.round(rect.height * 100) / 100,
      background: color(style.backgroundColor),
      color: color(style.color),
      fontSize: style.fontSize,
      fontFamily: style.fontFamily,
      fontWeight: style.fontWeight,
      lineHeight: style.lineHeight,
      borderColor: color(style.borderTopColor),
      borderRadius: style.borderRadius,
      boxShadow: style.boxShadow,
      opacity: style.opacity,
      transform: style.transform,
      role: element.getAttribute("role") || "",
      accessibleName: element.getAttribute("aria-label") || directText(element),
      rasterKind,
      animated: style.animationName !== "none",
      animationDurationMs: duration,
      animationDelayMs: style.animationName !== "none"
        ? Math.round((parseFloat(style.animationDelay) || 0) * (style.animationDelay.includes("ms") ? 1 : 1000))
        : null,
      animationEasing: style.animationName !== "none" ? style.animationTimingFunction : "",
      animationDirection: style.animationName !== "none" ? style.animationDirection : "",
      motions: []
    };
  }
  const animations = document.getAnimations();
  if (animations.length > 128) {
    throw new Error(`capture animation limit exceeded: ${animations.length}`);
  }
  const visualProperties = new Set([
    "backgroundColor", "borderRadius", "bottom", "boxShadow", "clipPath",
    "color", "filter", "height", "left", "opacity", "right", "top",
    "transform", "width"
  ]);
  const animationStates = animations.map((animation) => ({
    animation,
    currentTime: animation.currentTime,
    playState: animation.playState
  }));
  for (const { animation } of animationStates) {
    try { animation.pause(); } catch {}
  }
  try {
    for (const animation of animations) {
      const effect = animation.effect;
      const element = effect?.target;
      if (!(element instanceof Element) || !(effect instanceof KeyframeEffect)) continue;
      const id = pathOf(element);
      if (!nodes[id]) continue;
      const timing = effect.getTiming();
      const computed = effect.getComputedTiming();
      const keyframes = effect.getKeyframes();
      const properties = Array.from(new Set(keyframes.flatMap((frame) =>
        Object.keys(frame).filter((property) => visualProperties.has(property))
      ))).sort();
      const checkpoints = [];
      const activeDuration = Number(computed.activeDuration);
      const delay = Number(timing.delay) || 0;
      if (Number.isFinite(activeDuration)) {
        for (const progress of [0, 25, 50, 75, 100]) {
          try {
            animation.currentTime = delay + activeDuration * progress / 100;
            const style = getComputedStyle(element);
            const values = {};
            for (const property of properties) {
              const cssName = property.replace(/[A-Z]/g, (value) => `-${value.toLowerCase()}`);
              values[property] = style.getPropertyValue(cssName).trim();
            }
            checkpoints.push({ progress, values });
          } catch {}
        }
      }
      const kind = animation.constructor?.name === "CSSAnimation"
        ? "css-animation"
        : animation.constructor?.name === "CSSTransition"
          ? "css-transition"
          : "web-animation";
      const name = kind === "css-animation"
        ? animation.animationName
        : kind === "css-transition"
          ? animation.transitionProperty
          : animation.id || properties.join(",");
      nodes[id].motions.push({
        kind,
        name,
        durationMs: Number.isFinite(activeDuration) ? Math.round(activeDuration) : 0,
        delayMs: Math.round(delay),
        endDelayMs: Math.round(Number(timing.endDelay) || 0),
        iterations: timing.iterations === Infinity ? "infinite" : String(timing.iterations),
        direction: timing.direction || "normal",
        fill: timing.fill || "none",
        easing: timing.easing || "linear",
        properties,
        checkpoints
      });
    }
  } finally {
    for (const { animation, currentTime, playState } of animationStates) {
      try {
        animation.currentTime = currentTime;
        if (playState === "running") animation.play();
      } catch {}
    }
  }
  for (const node of Object.values(nodes)) {
    node.motions.sort((left, right) =>
      `${left.kind}:${left.name}:${left.properties.join(",")}`
        .localeCompare(`${right.kind}:${right.name}:${right.properties.join(",")}`)
    );
  }
  const active = document.activeElement && document.activeElement !== document.body
    ? pathOf(document.activeElement) : "";
  return {
    nodes,
    activeElement: active,
    runtime: globalThis.__backtest?.snapshot?.() || {
      consoleErrors: [],
      requests: [],
      pendingTimers: 0,
      pendingFrames: 0,
      layoutShifts: []
    }
  };
})()
"##;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEvidence {
    nodes: BTreeMap<String, NodeEvidence>,
    active_element: String,
    runtime: RawRuntime,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRuntime {
    console_errors: Vec<String>,
    requests: Vec<String>,
    pending_timers: usize,
    pending_frames: usize,
    layout_shifts: Vec<LayoutShiftEvidence>,
}

pub async fn install(cdp: &mut Cdp, deadline: Deadline) -> anyhow::Result<()> {
    cdp.call("Page.enable", json!({}), deadline).await?;
    cdp.call("Runtime.enable", json!({}), deadline).await?;
    cdp.call(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({ "source": instrumentation::PRE_DOCUMENT }),
        deadline,
    )
    .await?;
    Ok(())
}

/// A page in a window that is not in front is treated as hidden: input dispatch
/// blocks for five seconds and animation frames are throttled. Emulating focus
/// makes an unattended comparison both fast and faithful.
pub async fn focus_page(cdp: &mut Cdp, deadline: Deadline) -> anyhow::Result<()> {
    cdp.call(
        "Emulation.setFocusEmulationEnabled",
        json!({ "enabled": true }),
        deadline,
    )
    .await?;
    Ok(())
}

pub async fn navigate(
    cdp: &mut Cdp,
    url: &str,
    viewport: &Viewport,
    deadline: Deadline,
) -> anyhow::Result<()> {
    focus_page(cdp, deadline).await?;
    cdp.call(
        "Emulation.setDeviceMetricsOverride",
        json!({
            "width": viewport.width,
            "height": viewport.height,
            "deviceScaleFactor": 1,
            "mobile": false
        }),
        deadline,
    )
    .await?;
    cdp.call(
        "Input.dispatchMouseEvent",
        json!({ "type": "mouseMoved", "x": 0, "y": 0 }),
        deadline,
    )
    .await?;
    cdp.call("Page.navigate", json!({ "url": url }), deadline)
        .await?;
    loop {
        let ready = cdp
            .evaluate("document.readyState === 'complete'", deadline)
            .await?;
        if ready == Value::Bool(true) {
            break;
        }
        deadline
            .run("document readiness", async {
                tokio::time::sleep(Duration::from_millis(5)).await;
                Ok(())
            })
            .await?;
    }
    cdp.evaluate("globalThis.__backtest?.advance(0)", deadline)
        .await?;
    settle(cdp, deadline).await
}

/// Waits for a already-rendered document to stop changing without navigating it.
pub async fn settle(cdp: &mut Cdp, deadline: Deadline) -> anyhow::Result<()> {
    let mut previous = Value::Null;
    let mut stable_samples = 0;
    loop {
        let rendered = cdp
            .evaluate(
                r#"(() => {
                  const elements = Array.from(document.querySelectorAll("*"));
                  const visible = elements.filter((element) => {
                    const rect = element.getBoundingClientRect();
                    const style = getComputedStyle(element);
                    return rect.width > 0 && rect.height > 0 &&
                      style.display !== "none" && style.visibility !== "hidden";
                  }).length;
                  return {
                    href: location.href,
                    elements: elements.length,
                    visible,
                    textLength: document.body?.innerText?.length || 0,
                    text: (document.body?.innerText || "").trim().slice(0, 160)
                  };
                })()"#,
                deadline,
            )
            .await?;
        let visible = rendered["visible"].as_u64().unwrap_or_default();
        let loader_like = visible < 20
            && rendered["text"]
                .as_str()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .contains("loading");
        if visible > 0 && !loader_like && rendered == previous {
            stable_samples += 1;
            if stable_samples >= 6 {
                return Ok(());
            }
        } else {
            stable_samples = 0;
            previous = rendered;
        }
        deadline
            .run("rendered DOM stabilization", async {
                tokio::time::sleep(Duration::from_millis(25)).await;
                Ok(())
            })
            .await?;
    }
}

pub async fn record_source(session: &Session, baseline_only: bool) -> anyhow::Result<Artifact> {
    session.verify()?;
    anyhow::ensure!(
        session.side == crate::model::Side::Source,
        "record requires a source session"
    );
    let target = browser::target(&session.cdp_url, &session.target_id).await?;
    let mut cdp = Cdp::connect(&target.web_socket_debugger_url, Duration::from_secs(5)).await?;
    let deadline = Deadline::new(30_000);
    install(&mut cdp, deadline).await?;
    navigate(
        &mut cdp,
        &session.requested_url,
        &session.viewport,
        deadline,
    )
    .await?;
    let actions = if baseline_only {
        Vec::new()
    } else {
        discover_actions(&mut cdp, deadline).await?
    };
    let states = capture_states(
        &mut cdp,
        &session.requested_url,
        &session.viewport,
        &actions,
        deadline,
    )
    .await?;
    let fingerprint = digest::json(&states[0].nodes)?;
    let mut artifact = Artifact {
        schema_version: SCHEMA_VERSION,
        source: SourceIdentity {
            requested_url: session.requested_url.clone(),
            rendered_url: session.rendered_url.clone(),
            browser: session.browser.clone(),
            fingerprint,
        },
        actions,
        states,
        digest: String::new(),
    };
    artifact.seal()?;
    Ok(artifact)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentIdentity {
    url: String,
    title: String,
    error_page: bool,
    element_count: u64,
}

const DOCUMENT_IDENTITY: &str = r#"(() => {
    const document_ = document;
    const errorPage =
        location.protocol === 'chrome-error:' ||
        location.protocol === 'edge-error:' ||
        location.href.startsWith('about:neterror') ||
        !!document_.getElementById('main-frame-error') ||
        !!document_.getElementById('security-error') ||
        typeof window.certificateErrorPageController !== 'undefined';
    return {
        url: location.href,
        title: document_.title || '',
        errorPage,
        elementCount: document_.querySelectorAll('*').length
    };
})()"#;

/// Rejects browser error pages and empty documents before anything is measured,
/// so a comparison never silently reports an interstitial as if it were the page.
async fn verify_document(
    cdp: &mut Cdp,
    requested: &str,
    deadline: Deadline,
) -> anyhow::Result<DocumentIdentity> {
    let identity: DocumentIdentity =
        serde_json::from_value(cdp.evaluate(DOCUMENT_IDENTITY, deadline).await?)?;
    if let Some(reason) = document_failure(&identity) {
        anyhow::bail!(
            "{reason}.\n  requested: {requested}\n  rendered:  {}\n  title:     {}",
            identity.url,
            identity.title
        );
    }
    Ok(identity)
}

fn document_failure(identity: &DocumentIdentity) -> Option<&'static str> {
    if identity.error_page {
        return Some(
            "the browser showed its own error page instead of this address. \
             Check that the server is running, and for https that this browser profile \
             trusts its certificate",
        );
    }
    if identity.element_count <= 3 {
        return Some(
            "this address rendered an empty document. \
             Check that the server is serving the expected build",
        );
    }
    None
}

/// Captures the current rendered state of an already-prepared tab.
/// Never navigates or reloads, so authenticated and interaction-derived state survives.
async fn snapshot_states(
    cdp: &mut Cdp,
    requested: &str,
    viewport: &Viewport,
    deadline: Deadline,
) -> anyhow::Result<Vec<State>> {
    cdp.call("Page.enable", json!({}), deadline).await?;
    cdp.call("Runtime.enable", json!({}), deadline).await?;
    focus_page(cdp, deadline).await?;
    verify_document(cdp, requested, deadline).await?;
    cdp.call(
        "Emulation.setDeviceMetricsOverride",
        json!({
            "width": viewport.width,
            "height": viewport.height,
            "deviceScaleFactor": 1,
            "mobile": false
        }),
        deadline,
    )
    .await?;
    settle(cdp, deadline).await?;
    let base = capture_state(cdp, viewport, "base", false, deadline).await?;
    cdp.call("Emulation.clearDeviceMetricsOverride", json!({}), deadline)
        .await?;
    let mut load = base.clone();
    load.scenario = "load".into();
    Ok(vec![base, load])
}

pub async fn record_source_snapshot(session: &Session) -> anyhow::Result<Artifact> {
    session.verify()?;
    anyhow::ensure!(
        session.side == crate::model::Side::Source,
        "record requires a source session"
    );
    let target = browser::target(&session.cdp_url, &session.target_id).await?;
    let mut cdp = Cdp::connect(&target.web_socket_debugger_url, Duration::from_secs(5)).await?;
    let deadline = Deadline::new(30_000);
    let states = snapshot_states(
        &mut cdp,
        &session.requested_url,
        &session.viewport,
        deadline,
    )
    .await?;
    let fingerprint = digest::json(&states[0].nodes)?;
    let mut artifact = Artifact {
        schema_version: SCHEMA_VERSION,
        source: SourceIdentity {
            requested_url: session.requested_url.clone(),
            rendered_url: session.rendered_url.clone(),
            browser: session.browser.clone(),
            fingerprint,
        },
        actions: Vec::new(),
        states,
        digest: String::new(),
    };
    artifact.seal()?;
    Ok(artifact)
}

pub async fn compare_candidate_snapshot(
    session: &Session,
    target: browser::Target,
    deadline: Deadline,
) -> anyhow::Result<Artifact> {
    let mut cdp = deadline
        .run("candidate CDP connection", async {
            Cdp::connect(&target.web_socket_debugger_url, deadline.remaining()?).await
        })
        .await?;
    // The recreation is reloaded so a rebuild is measured. Without this a fix
    // is invisible, because the tab keeps rendering the build it first loaded.
    cdp.call("Page.enable", json!({}), deadline).await?;
    cdp.call("Runtime.enable", json!({}), deadline).await?;
    navigate(
        &mut cdp,
        &session.requested_url,
        &session.viewport,
        deadline,
    )
    .await?;
    let states = snapshot_states(
        &mut cdp,
        &session.requested_url,
        &session.viewport,
        deadline,
    )
    .await?;
    let mut candidate = Artifact {
        schema_version: SCHEMA_VERSION,
        source: SourceIdentity {
            requested_url: session.requested_url.clone(),
            rendered_url: session.rendered_url.clone(),
            browser: session.browser.clone(),
            fingerprint: digest::json(&states[0].nodes)?,
        },
        actions: Vec::new(),
        states,
        digest: String::new(),
    };
    candidate.seal()?;
    Ok(candidate)
}

pub async fn validate_candidate(
    artifact: &Artifact,
    session: &Session,
) -> anyhow::Result<browser::Target> {
    artifact.verify()?;
    session.verify()?;
    anyhow::ensure!(
        session.side == crate::model::Side::Candidate,
        "candidate session required"
    );
    anyhow::ensure!(
        artifact.source.browser == session.browser,
        "source and candidate browser versions differ"
    );
    browser::target(&session.cdp_url, &session.target_id).await
}

pub async fn compare_candidate(
    artifact: &Artifact,
    session: &Session,
    target: browser::Target,
    deadline: Deadline,
) -> anyhow::Result<Artifact> {
    let mut cdp = deadline
        .run("candidate CDP connection", async {
            Cdp::connect(&target.web_socket_debugger_url, deadline.remaining()?).await
        })
        .await?;
    install(&mut cdp, deadline).await?;
    let states = capture_states(
        &mut cdp,
        &session.requested_url,
        &session.viewport,
        &artifact.actions,
        deadline,
    )
    .await?;
    let mut candidate = Artifact {
        schema_version: SCHEMA_VERSION,
        source: SourceIdentity {
            requested_url: session.requested_url.clone(),
            rendered_url: session.rendered_url.clone(),
            browser: session.browser.clone(),
            fingerprint: digest::json(&states[0].nodes)?,
        },
        actions: artifact.actions.clone(),
        states,
        digest: String::new(),
    };
    candidate.seal()?;
    Ok(candidate)
}

async fn discover_actions(cdp: &mut Cdp, deadline: Deadline) -> anyhow::Result<Vec<Action>> {
    let value = cdp
        .evaluate(
            r#"(() => {
              const pathCache = new WeakMap([[document.documentElement, "html"]]);
              const pathOf = (element) => {
                if (!element) return "";
                const authored = element.getAttribute?.("data-backtest-id");
                if (authored) return authored;
                if (pathCache.has(element)) return pathCache.get(element);
                const parent = element.parentElement;
                const peers = parent
                  ? Array.from(parent.children).filter((value) => value.tagName === element.tagName)
                  : [element];
                const path = `${pathOf(parent)}>${element.tagName.toLowerCase()}:nth-of-type(${peers.indexOf(element) + 1})`;
                pathCache.set(element, path);
                return path;
              };
              const explicit = Array.from(document.querySelectorAll("[data-backtest-action]"));
              if (explicit.length) {
                return explicit.map((element) => ({
                  target: element.getAttribute("data-backtest-id") || pathOf(element),
                  action: element.getAttribute("data-backtest-action")
                }));
              }
              const actionable = Array.from(document.querySelectorAll(
                'button,a[href],input,select,textarea,[role="button"],[tabindex]:not([tabindex="-1"])'
              )).filter((element) => {
                const rect = element.getBoundingClientRect();
                const style = getComputedStyle(element);
                return rect.width > 0 && rect.height > 0 &&
                  style.display !== "none" && style.visibility !== "hidden" &&
                  !element.disabled;
              }).slice(0, 6);
              return actionable.flatMap((element) => {
                const target = pathOf(element);
                return [
                  { target, action: "click" },
                  { target, action: "hover" }
                ];
              });
            })()"#,
            deadline,
        )
        .await?;
    let mut actions = Vec::new();
    for entry in value.as_array().into_iter().flatten() {
        let target = entry
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let action = entry
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if action == "click" {
            actions.push(Action::Click {
                target: target.into(),
            });
        } else if action == "hover" {
            actions.push(Action::Hover {
                target: target.into(),
            });
        } else if action == "animation" {
            actions.push(Action::Animation {
                target: target.into(),
            });
        } else if let Some(value) = action.strip_prefix("timer:") {
            actions.push(Action::Timer {
                milliseconds: value.parse()?,
                target: target.into(),
            });
        } else if let Some(value) = action.strip_prefix("click-sequence:") {
            actions.push(Action::ClickSequence {
                targets: value.split(',').map(str::to_owned).collect(),
                label: target.into(),
            });
        }
    }
    Ok(actions)
}

async fn capture_states(
    cdp: &mut Cdp,
    url: &str,
    viewport: &Viewport,
    actions: &[Action],
    deadline: Deadline,
) -> anyhow::Result<Vec<State>> {
    let mut states = Vec::new();
    navigate(cdp, url, viewport, deadline).await?;
    let base = capture_state(cdp, viewport, "base", false, deadline).await?;
    states.push(base.clone());
    for action in actions {
        navigate(cdp, url, viewport, deadline).await?;
        execute(cdp, action, deadline).await?;
        states.push(
            capture_state(
                cdp,
                viewport,
                &action.scenario(),
                matches!(action, Action::Animation { .. }),
                deadline,
            )
            .await?,
        );
    }
    let mut load = base;
    load.scenario = "load".into();
    states.push(load);
    Ok(states)
}

async fn execute(cdp: &mut Cdp, action: &Action, deadline: Deadline) -> anyhow::Result<()> {
    match action {
        Action::Click { target } => {
            click(cdp, target, deadline).await?;
        }
        Action::Hover { target } => {
            let expression = format!(
                r#"(() => {{
                  const target = "{}";
                  const element = document.querySelector(`[data-backtest-id="${{CSS.escape(target)}}"]`) ||
                    document.querySelector(target);
                  if (!element) throw new Error("missing action target");
                  const r = element.getBoundingClientRect();
                  return {{x:r.left+r.width/2,y:r.top+r.height/2}};
                }})()"#,
                js_escape(target),
            );
            let point = cdp.evaluate(&expression, deadline).await?;
            cdp.call(
                "Input.dispatchMouseEvent",
                json!({
                    "type": "mouseMoved",
                    "x": point["x"],
                    "y": point["y"]
                }),
                deadline,
            )
            .await?;
        }
        Action::ClickSequence { targets, .. } => {
            for target in targets {
                click(cdp, target, deadline).await?;
            }
        }
        Action::Timer { milliseconds, .. } => {
            cdp.evaluate(
                &format!("globalThis.__backtest.advance({milliseconds})"),
                deadline,
            )
            .await?;
        }
        Action::Animation { .. } => {}
    }
    cdp.evaluate("Promise.resolve().then(() => true)", deadline)
        .await?;
    Ok(())
}

async fn click(cdp: &mut Cdp, target: &str, deadline: Deadline) -> anyhow::Result<()> {
    let expression = format!(
        r#"(() => {{
          const target = "{}";
          const element = document.querySelector(`[data-backtest-id="${{CSS.escape(target)}}"]`) ||
            document.querySelector(target);
          if (!element) throw new Error("missing action target");
          const rect = element.getBoundingClientRect();
          return {{x:rect.left+rect.width/2,y:rect.top+rect.height/2}};
        }})()"#,
        js_escape(target),
    );
    let point = cdp.evaluate(&expression, deadline).await?;
    for event_type in ["mousePressed", "mouseReleased"] {
        cdp.call(
            "Input.dispatchMouseEvent",
            json!({
                "type": event_type,
                "x": point["x"],
                "y": point["y"],
                "button": "left",
                "clickCount": 1
            }),
            deadline,
        )
        .await?;
    }
    Ok(())
}

async fn capture_state(
    cdp: &mut Cdp,
    viewport: &Viewport,
    scenario: &str,
    keep_animation: bool,
    deadline: Deadline,
) -> anyhow::Result<State> {
    cdp.evaluate(
        r#"(() => {
          document.documentElement.getBoundingClientRect();
          return globalThis.__backtest?.snapshot?.() || null;
        })()"#,
        deadline,
    )
    .await?;
    let mut raw: RawEvidence =
        serde_json::from_value(cdp.evaluate(EVIDENCE_SCRIPT, deadline).await?)?;
    anyhow::ensure!(
        !raw.nodes.is_empty(),
        "capture produced no visible DOM evidence"
    );
    apply_accessibility_names(cdp, &mut raw.nodes, deadline).await?;
    cdp.evaluate(
        r#"(() => {
          globalThis.__backtestHiddenAnimations = [];
          for (const animation of document.getAnimations()) {
            const element = animation.effect?.target;
            if (!(element instanceof Element) ||
                globalThis.__backtestHiddenAnimations.some((entry) => entry.element === element)) {
              continue;
            }
            globalThis.__backtestHiddenAnimations.push({
              element,
              style: element.getAttribute("style")
            });
            element.style.setProperty("visibility", "hidden", "important");
          }
          return globalThis.__backtestHiddenAnimations.length;
        })()"#,
        deadline,
    )
    .await?;
    let screenshot_parameters =
        json!({ "format": "png", "fromSurface": true, "captureBeyondViewport": false });
    if scenario.starts_with("hover:") {
        // The first frame after a synthetic hover can precede the compositor repaint.
        cdp.call(
            "Page.captureScreenshot",
            screenshot_parameters.clone(),
            deadline,
        )
        .await?;
    }
    let result = cdp
        .call("Page.captureScreenshot", screenshot_parameters, deadline)
        .await?;
    let encoded = result
        .get("data")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let png = base64::engine::general_purpose::STANDARD.decode(encoded)?;
    let pixels = image::load_from_memory_with_format(&png, image::ImageFormat::Png)?.to_rgba8();
    let screenshot_sha256 = digest::bytes(pixels.as_raw());
    cdp.evaluate(
        r#"(() => {
          for (const { element, style } of globalThis.__backtestHiddenAnimations || []) {
            if (style === null) element.removeAttribute("style");
            else element.setAttribute("style", style);
          }
          delete globalThis.__backtestHiddenAnimations;
          return true;
        })()"#,
        deadline,
    )
    .await?;
    let mut nodes = raw.nodes;
    apply_rendered_content_hashes(&mut nodes, &pixels)?;
    let raster_tiles = capture_raster_tiles(&pixels);
    if !keep_animation {
        for node in nodes.values_mut() {
            node.animation_duration_ms = None;
            node.animation_delay_ms = None;
            node.animation_easing.clear();
            node.animation_direction.clear();
        }
    }
    Ok(State {
        viewport: viewport.clone(),
        scenario: scenario.into(),
        nodes,
        active_element: raw.active_element,
        runtime: RuntimeEvidence {
            console_errors: raw.runtime.console_errors,
            requests: raw.runtime.requests,
            pending_timers: raw.runtime.pending_timers,
            pending_frames: raw.runtime.pending_frames,
            layout_shifts: raw.runtime.layout_shifts,
        },
        screenshot_sha256,
        raster_tiles,
        capture_complete: true,
    })
}

fn capture_raster_tiles(pixels: &image::RgbaImage) -> Vec<RasterTileEvidence> {
    const TILE_SIZE: u32 = 32;
    let mut tiles = Vec::new();
    for top in (0..pixels.height()).step_by(TILE_SIZE as usize) {
        for left in (0..pixels.width()).step_by(TILE_SIZE as usize) {
            let width = TILE_SIZE.min(pixels.width() - left);
            let height = TILE_SIZE.min(pixels.height() - top);
            let mut bytes = Vec::with_capacity((width * height * 4) as usize);
            for y in top..top + height {
                for x in left..left + width {
                    bytes.extend_from_slice(&pixels.get_pixel(x, y).0);
                }
            }
            tiles.push(RasterTileEvidence {
                x: left,
                y: top,
                width,
                height,
                sha256: digest::bytes(&bytes),
            });
        }
    }
    tiles
}

fn apply_rendered_content_hashes(
    nodes: &mut BTreeMap<String, NodeEvidence>,
    pixels: &image::RgbaImage,
) -> anyhow::Result<()> {
    let width = pixels.width();
    let height = pixels.height();
    let maximum_area = u64::from(width) * u64::from(height) * 16;
    let mut captured_area = 0_u64;
    for node in nodes
        .values_mut()
        .filter(|node| node.visible && matches!(node.tag.as_str(), "img" | "svg"))
    {
        let left = node.x.floor().max(0.0).min(f64::from(width)) as u32;
        let top = node.y.floor().max(0.0).min(f64::from(height)) as u32;
        let right = (node.x + node.width).ceil().max(0.0).min(f64::from(width)) as u32;
        let bottom = (node.y + node.height)
            .ceil()
            .max(0.0)
            .min(f64::from(height)) as u32;
        if right <= left || bottom <= top {
            continue;
        }
        let area = u64::from(right - left) * u64::from(bottom - top);
        captured_area += area;
        anyhow::ensure!(
            captured_area <= maximum_area,
            "rendered image and SVG evidence exceeded viewport budget"
        );
        let mut bytes = Vec::with_capacity(area as usize * 4);
        for y in top..bottom {
            for x in left..right {
                bytes.extend_from_slice(&pixels.get_pixel(x, y).0);
            }
        }
        node.rendered_content_sha256 = digest::bytes(&bytes);
    }
    Ok(())
}

async fn apply_accessibility_names(
    cdp: &mut Cdp,
    nodes: &mut BTreeMap<String, NodeEvidence>,
    deadline: Deadline,
) -> anyhow::Result<()> {
    let snapshot = cdp
        .call(
            "DOMSnapshot.captureSnapshot",
            json!({ "computedStyles": [], "includeDOMRects": false, "includePaintOrder": false }),
            deadline,
        )
        .await?;
    let backend_targets = snapshot_backend_targets(&snapshot)?;
    let tree = cdp
        .call(
            "Accessibility.getFullAXTree",
            json!({ "depth": -1 }),
            deadline,
        )
        .await?;
    let ax_nodes = tree
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let ax_by_backend: HashMap<u64, &Value> = ax_nodes
        .iter()
        .filter(|node| node["ignored"] != Value::Bool(true))
        .filter_map(|node| Some((node["backendDOMNodeId"].as_u64()?, node)))
        .collect();
    for evidence in nodes.values_mut() {
        evidence.accessible_name.clear();
    }
    for (backend, target) in backend_targets {
        let Some(evidence) = nodes.get_mut(&target) else {
            continue;
        };
        let Some(ax) = ax_by_backend.get(&backend) else {
            continue;
        };
        if let Some(name) = ax["name"]["value"].as_str() {
            evidence.accessible_name = name.into();
        }
        if let Some(role) = ax["role"]["value"].as_str() {
            evidence.role = role.into();
        }
    }
    Ok(())
}

fn snapshot_backend_targets(snapshot: &Value) -> anyhow::Result<HashMap<u64, String>> {
    let strings = snapshot["strings"]
        .as_array()
        .context("DOM snapshot omitted strings")?;
    let raw_nodes = &snapshot["documents"][0]["nodes"];
    let backend_ids = raw_nodes["backendNodeId"]
        .as_array()
        .context("DOM snapshot omitted backend node IDs")?;
    let node_names = raw_nodes["nodeName"]
        .as_array()
        .context("DOM snapshot omitted node names")?;
    let parent_indexes = raw_nodes["parentIndex"]
        .as_array()
        .context("DOM snapshot omitted parent indexes")?;
    let attributes = raw_nodes["attributes"]
        .as_array()
        .context("DOM snapshot omitted attributes")?;
    anyhow::ensure!(
        backend_ids.len() == node_names.len()
            && backend_ids.len() == parent_indexes.len()
            && backend_ids.len() == attributes.len(),
        "DOM snapshot node arrays differ in length"
    );

    let string_at = |value: &Value| -> anyhow::Result<&str> {
        let index = value
            .as_u64()
            .context("invalid DOM snapshot string index")? as usize;
        strings
            .get(index)
            .and_then(Value::as_str)
            .context("DOM snapshot string index out of range")
    };
    let mut paths = vec![String::new(); backend_ids.len()];
    let mut sibling_counts: HashMap<(i64, String), usize> = HashMap::new();
    let mut result = HashMap::new();
    for index in 0..backend_ids.len() {
        let tag = string_at(&node_names[index])?.to_ascii_lowercase();
        if tag.starts_with('#') {
            continue;
        }
        let parent = parent_indexes[index].as_i64().unwrap_or(-1);
        let authored = attributes[index]
            .as_array()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .chunks_exact(2)
            .find_map(|pair| {
                (string_at(pair[0]).ok()? == "data-backtest-id")
                    .then(|| string_at(pair[1]).ok())
                    .flatten()
            });
        let path = if let Some(authored) = authored {
            authored.to_owned()
        } else if tag == "html" {
            "html".into()
        } else {
            let count = sibling_counts.entry((parent, tag.clone())).or_default();
            *count += 1;
            let parent_path = usize::try_from(parent)
                .ok()
                .and_then(|value| paths.get(value))
                .cloned()
                .unwrap_or_default();
            format!("{parent_path}>{tag}:nth-of-type({count})")
        };
        paths[index] = path.clone();
        if let Some(backend) = backend_ids[index].as_u64() {
            result.insert(backend, path);
        }
    }
    Ok(result)
}

fn js_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(url: &str, error_page: bool, element_count: u64) -> DocumentIdentity {
        DocumentIdentity {
            url: url.into(),
            title: "page".into(),
            error_page,
            element_count,
        }
    }

    #[test]
    fn rejects_a_browser_error_page_instead_of_measuring_it() {
        let reason = document_failure(&identity("chrome-error://chromewebdata/", true, 56))
            .expect("an error page must never be measured");
        assert!(reason.contains("error page"));
        assert!(reason.contains("certificate"));
    }

    #[test]
    fn rejects_an_empty_document() {
        let reason = document_failure(&identity("http://127.0.0.1:8080/", false, 3))
            .expect("an empty document must never be measured");
        assert!(reason.contains("empty document"));
    }

    #[test]
    fn accepts_a_rendered_page() {
        assert!(document_failure(&identity("http://127.0.0.1:8080/", false, 900)).is_none());
    }
}
