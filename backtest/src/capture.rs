use crate::{
    browser,
    cdp::Cdp,
    deadline::Deadline,
    digest,
    model::{
        Action, ActionKind, AnimationSnapshot, Artifact, Checkpoint, NodeSnapshot, ScenarioEvidence,
        Session, Snapshot, SourceIdentity, StateEvidence, Viewport, SCHEMA_VERSION,
    },
};
use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::json;
use std::{collections::BTreeMap, fs, path::Path, time::Duration};

const INSTRUMENT: &str = r#"
(() => {
  if (window.__backtestInstalled) return;
  window.__backtestInstalled = true;
  window.__backtestNow = 0;
  window.__backtestNextTask = 1;
  window.__backtestTasks = new Map();
  window.__backtestRafs = new Map();
  window.__backtestErrors = [];
  window.__backtestRequests = [];
  window.__backtestPending = 0;
  const enqueue = (callback, delay, interval, args) => {
    const id = window.__backtestNextTask++;
    window.__backtestTasks.set(id, {
      id, callback, due: window.__backtestNow + Math.max(0, Number(delay) || 0),
      interval, args
    });
    return id;
  };
  window.setTimeout = (callback, delay, ...args) =>
    enqueue(callback, delay, 0, args);
  window.setInterval = (callback, delay, ...args) =>
    enqueue(callback, delay, Math.max(1, Number(delay) || 1), args);
  window.clearTimeout = window.clearInterval = id =>
    window.__backtestTasks.delete(id);
  window.requestAnimationFrame = callback => {
    const id = window.__backtestNextTask++;
    window.__backtestRafs.set(id, callback);
    return id;
  };
  window.cancelAnimationFrame = id => window.__backtestRafs.delete(id);
  window.requestIdleCallback = callback =>
    enqueue(() => callback({didTimeout:false,timeRemaining:()=>50}), 0, 0, []);
  window.cancelIdleCallback = id => window.__backtestTasks.delete(id);
  const originalFetch = window.fetch.bind(window);
  window.fetch = async (...args) => {
    const input = args[0];
    const url = typeof input === 'string' ? input : input?.url || '';
    window.__backtestRequests.push(new URL(url, location.href).pathname);
    window.__backtestPending++;
    try { return await originalFetch(...args); }
    finally { window.__backtestPending--; }
  };
  const originalError = console.error.bind(console);
  console.error = (...args) => {
    window.__backtestErrors.push(args.map(String).join(' '));
    originalError(...args);
  };
  addEventListener('error', event => {
    window.__backtestErrors.push(String(event.message || event.error || 'error'));
  });
  addEventListener('unhandledrejection', event => {
    window.__backtestErrors.push(String(event.reason || 'unhandled rejection'));
  });
  window.__backtestAdvance = async milliseconds => {
    const target = window.__backtestNow + Math.max(0, Number(milliseconds) || 0);
    let guard = 0;
    while (guard++ < 10000) {
      const tasks = [...window.__backtestTasks.values()]
        .filter(task => task.due <= target)
        .sort((left, right) => left.due - right.due || left.id - right.id);
      if (!tasks.length) break;
      const task = tasks[0];
      window.__backtestNow = task.due;
      if (task.interval) task.due += task.interval;
      else window.__backtestTasks.delete(task.id);
      if (typeof task.callback === 'function') task.callback(...task.args);
      else (0, eval)(String(task.callback));
      await Promise.resolve();
      const rafs = [...window.__backtestRafs.entries()];
      window.__backtestRafs.clear();
      for (const [, callback] of rafs) callback(window.__backtestNow);
      await Promise.resolve();
    }
    window.__backtestNow = target;
    for (const animation of document.getAnimations({subtree:true})) {
      const timing = animation.effect?.getTiming?.() || {};
      const duration = Number(timing.duration || 0);
      if (duration > 0 && Number.isFinite(duration)) {
        animation.pause();
        animation.currentTime = Math.max(0, target - Number(timing.delay || 0)) % duration;
      }
    }
    await Promise.resolve();
    return {
      now: window.__backtestNow,
      pendingTasks: window.__backtestTasks.size,
      pendingRequests: window.__backtestPending
    };
  };
  window.__backtestAnimationProgress = async progress => {
    for (const animation of document.getAnimations({subtree:true})) {
      const timing = animation.effect?.getTiming?.() || {};
      const duration = Number(timing.duration || 0);
      if (duration > 0 && Number.isFinite(duration)) {
        animation.pause();
        animation.currentTime = duration * Number(progress);
      }
    }
    await Promise.resolve();
  };
})()
"#;

const SNAPSHOT: &str = r#"
(() => {
  const pathCache = new WeakMap([[document.documentElement, 'html']]);
  const pathOf = element => {
    if (!element) return '';
    if (element.dataset?.backtestId) return element.dataset.backtestId;
    const cached = pathCache.get(element);
    if (cached) return cached;
    const parent = element.parentElement;
    const peers = parent
      ? [...parent.children].filter(value => value.tagName === element.tagName)
      : [element];
    const path = `${pathOf(parent)}>${element.tagName.toLowerCase()}:nth-of-type(${peers.indexOf(element)+1})`;
    pathCache.set(element, path);
    return path;
  };
  const implicitRole = element => {
    const tag = element.tagName.toLowerCase();
    if (tag === 'button') return 'button';
    if (tag === 'a' && element.hasAttribute('href')) return 'link';
    if (tag === 'input') return element.type === 'checkbox' ? 'checkbox' : 'textbox';
    if (tag === 'select') return 'combobox';
    if (tag === 'textarea') return 'textbox';
    return '';
  };
  const properties = [
    'display','visibility','position','font-family','font-size','font-weight',
    'line-height','letter-spacing','color','background-color','border-color',
    'border-radius','box-shadow','opacity','transform','overflow'
  ];
  const nodes = [...document.querySelectorAll('*')]
    .filter(element => !['SCRIPT','STYLE','NOSCRIPT'].includes(element.tagName))
    .map(element => {
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      const id = pathOf(element);
      const parent = element.parentElement ? pathOf(element.parentElement) : null;
      const attributes = {};
      for (const name of [
        'aria-expanded','aria-pressed','aria-selected','aria-disabled',
        'disabled','hidden','checked','selected','value','placeholder','title'
      ]) if (element.hasAttribute(name)) attributes[name] = element.getAttribute(name) || '';
      const leaf = element.childElementCount === 0;
      return {
        id, path:id, parent,
        tag:element.tagName.toLowerCase(),
        text:leaf ? (element.textContent || '').replace(/\s+/g,' ').trim() : '',
        role:element.getAttribute('role') || implicitRole(element),
        name:(element.getAttribute('aria-label') || (leaf ? element.textContent : '') || '')
          .replace(/\s+/g,' ').trim(),
        attributes,
        rect:[rect.x,rect.y,rect.width,rect.height],
        style:Object.fromEntries(properties.map(property => [property,style.getPropertyValue(property)]))
      };
    });
  const animations = document.getAnimations({subtree:true}).map(animation => {
    const timing = animation.effect?.getTiming?.() || {};
    const keyframes = animation.effect?.getKeyframes?.() || [];
    const properties = [...new Set(keyframes.flatMap(frame => Object.keys(frame))
      .filter(key => !['offset','easing','composite','computedOffset'].includes(key)))].sort();
    return {
      target:pathOf(animation.effect?.target),
      duration:Number(timing.duration || 0),
      delay:Number(timing.delay || 0),
      iterations:String(timing.iterations === Infinity ? 'infinite' : timing.iterations || 1),
      direction:String(timing.direction || 'normal'),
      easing:String(timing.easing || 'linear'),
      fill:String(timing.fill || 'none'),
      properties,
      keyframes:JSON.stringify(keyframes.map(frame => {
        const result = {};
        for (const key of Object.keys(frame).sort()) {
          if (!['computedOffset','composite'].includes(key)) result[key] = frame[key];
        }
        return result;
      }))
    };
  }).sort((left,right) => left.target.localeCompare(right.target));
  const active = document.activeElement && document.activeElement !== document.body
    ? pathOf(document.activeElement) : null;
  return {
    url:location.href,
    title:document.title,
    document:[document.documentElement.scrollWidth,document.documentElement.scrollHeight],
    nodes, animations, active,
    consoleErrors:[...new Set(window.__backtestErrors || [])].sort(),
    unexpectedRequests:[...new Set(window.__backtestRequests || [])].sort(),
    pendingRequests:Number(window.__backtestPending || 0)
  };
})()
"#;

const DISCOVER: &str = r#"
(() => {
  const pathOf = element => element.dataset.backtestId || '';
  const visible = element => {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 &&
      style.display !== 'none' && style.visibility !== 'hidden';
  };
  const explicit = [...document.querySelectorAll('[data-backtest-action]')];
  const fallback = [...document.querySelectorAll(
    'button,a[href],input,select,textarea,[role="button"],[tabindex]:not([tabindex="-1"])'
  )];
  const values = (explicit.length ? explicit : fallback)
    .filter(visible)
    .map((element,index) => {
      const target=pathOf(element) || `action-${index}`;
      if(!element.dataset.backtestId)element.dataset.backtestRuntimeId=target;
      return {
        kind:element.dataset.backtestAction || 'click',
        target:element.dataset.backtestFollow || target,
        trigger:element.dataset.backtestFollow ? target : null,
        label:(element.getAttribute('aria-label') || element.textContent || '').replace(/\s+/g,' ').trim(),
        value:element.dataset.backtestValue || null,
        checkpoints:(element.dataset.backtestCheckpoints || '0,100')
          .split(',').map(Number).filter(Number.isFinite)
      };
    });
  for (const checkpoint of (document.documentElement.dataset.backtestTimerCheckpoints || '')
    .split(',').map(Number).filter(Number.isFinite)) {
    values.push({
      kind:'timer', target:String(checkpoint), trigger:null,
      label:`timer ${checkpoint}`, value:null, checkpoints:[checkpoint]
    });
  }
  return values.slice(0, 16);
})()
"#;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSnapshot {
    url: String,
    title: String,
    document: [f64; 2],
    nodes: Vec<NodeSnapshot>,
    animations: Vec<AnimationSnapshot>,
    active: Option<String>,
    console_errors: Vec<String>,
    unexpected_requests: Vec<String>,
    pending_requests: u32,
}

#[derive(Deserialize)]
struct DiscoveredAction {
    kind: String,
    target: String,
    trigger: Option<String>,
    label: String,
    value: Option<String>,
    checkpoints: Vec<u64>,
}

pub async fn install(cdp: &mut Cdp) -> anyhow::Result<()> {
    cdp.enable(&["Page", "Runtime", "Network", "Log"]).await?;
    cdp.send(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({"source":INSTRUMENT}),
    )
    .await?;
    cdp.evaluate(INSTRUMENT).await?;
    Ok(())
}

pub async fn reload(cdp: &mut Cdp, viewport: &Viewport) -> anyhow::Result<()> {
    browser::set_viewport(cdp, viewport).await?;
    cdp.send(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"no-preference"}]}),
    )
    .await?;
    cdp.clear_events();
    cdp.send("Page.reload", json!({"ignoreCache":false})).await?;
    wait_ready(cdp).await
}

pub async fn navigate(cdp: &mut Cdp, url: &str, viewport: &Viewport) -> anyhow::Result<()> {
    browser::set_viewport(cdp, viewport).await?;
    cdp.clear_events();
    cdp.send("Page.navigate", json!({"url":url})).await?;
    wait_ready(cdp).await
}

pub async fn wait_ready(cdp: &mut Cdp) -> anyhow::Result<()> {
    for _ in 0..100 {
        let ready = cdp
            .evaluate(
                "document.readyState!=='loading'&&!!document.body&&\
                 document.documentElement.getBoundingClientRect().width>0",
            )
            .await?
            .as_bool()
            .unwrap_or(false);
        if ready {
            cdp.evaluate("window.__backtestAdvance?.(0)").await?;
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    anyhow::bail!("page did not reach document readiness")
}

pub async fn fingerprint(cdp: &mut Cdp) -> anyhow::Result<String> {
    let value = cdp
        .evaluate(
            "JSON.stringify({url:location.href,title:document.title,\
             text:(document.body?.innerText||'').replace(/\\s+/g,' ').trim().slice(0,500),\
             controls:document.querySelectorAll('button,a,input,select,textarea').length})",
        )
        .await?;
    Ok(digest::bytes(value.as_str().unwrap_or_default().as_bytes()))
}

pub async fn snapshot(cdp: &mut Cdp) -> anyhow::Result<Snapshot> {
    let raw: RawSnapshot = serde_json::from_value(cdp.evaluate(SNAPSHOT).await?)?;
    let image = cdp
        .send(
            "Page.captureScreenshot",
            json!({"format":"png","fromSurface":true}),
        )
        .await?;
    let screenshot_png = image["data"].as_str().unwrap_or_default().to_string();
    let decoded = STANDARD.decode(&screenshot_png)?;
    let pixels = image::load_from_memory_with_format(&decoded, image::ImageFormat::Png)?
        .to_rgba8();
    let console_events = cdp
        .take_events_named("Runtime.consoleAPICalled")
        .into_iter()
        .filter(|event| event["params"]["type"] == "error")
        .map(|event| event["params"]["args"].to_string())
        .collect::<Vec<_>>();
    let network_failures = cdp
        .take_events_named("Network.loadingFailed")
        .into_iter()
        .filter(|event| event["params"]["canceled"] != true)
        .map(|event| {
            event["params"]["errorText"]
                .as_str()
                .unwrap_or("network failure")
                .into()
        })
        .collect();
    let mut console_errors = raw.console_errors;
    console_errors.extend(console_events);
    console_errors.sort();
    console_errors.dedup();
    Ok(Snapshot {
        url: raw.url,
        title: raw.title,
        document: raw.document,
        nodes: raw.nodes,
        animations: raw.animations,
        active: raw.active,
        pixel_hash: digest::bytes(pixels.as_raw()),
        screenshot_png,
        console_errors,
        network_failures,
        unexpected_requests: raw.unexpected_requests,
        pending_requests: raw.pending_requests,
    })
}

pub async fn discover(cdp: &mut Cdp) -> anyhow::Result<Vec<(Action, Vec<u64>)>> {
    let values: Vec<DiscoveredAction> = serde_json::from_value(cdp.evaluate(DISCOVER).await?)?;
    values
        .into_iter()
        .map(|value| {
            let kind = match value.kind.as_str() {
                "hover" => ActionKind::Hover,
                "focus" => ActionKind::Focus,
                "input" => ActionKind::Input,
                "escape" => ActionKind::Escape,
                "timer" => ActionKind::Timer,
                _ => ActionKind::Click,
            };
            Ok((
                Action {
                    kind,
                    target: value.target,
                    label: value.label,
                    trigger: value.trigger,
                    value: value.value,
                },
                value.checkpoints,
            ))
        })
        .collect()
}

pub async fn apply(cdp: &mut Cdp, action: &Action) -> anyhow::Result<()> {
    if matches!(action.kind, ActionKind::Timer) {
        return Ok(());
    }
    if matches!(action.kind, ActionKind::Escape) {
        for event_type in ["keyDown", "keyUp"] {
            cdp.send(
                "Input.dispatchKeyEvent",
                json!({"type":event_type,"key":"Escape","code":"Escape","windowsVirtualKeyCode":27}),
            )
            .await?;
        }
        return Ok(());
    }
    let primary_target = action.trigger.as_deref().unwrap_or(&action.target);
    let selector = format!(
        "[data-backtest-id={value}],[data-backtest-runtime-id={value}]",
        value = serde_json::to_string(primary_target)?
    );
    let selector_json = serde_json::to_string(&selector)?;
    let point = cdp
        .evaluate(&format!(
            "(()=>{{const element=document.querySelector({selector});\
             if(!element)return null;const rect=element.getBoundingClientRect();\
             const hit=document.elementFromPoint(rect.x+rect.width/2,rect.y+rect.height/2);\
             return {{x:rect.x+rect.width/2,y:rect.y+rect.height/2,\
             ready:!!hit&&(element===hit||element.contains(hit))}}}})()",
            selector = selector_json
        ))
        .await?;
    anyhow::ensure!(point["ready"] == true, "action target not interactable: {}", action.target);
    let x = point["x"].as_f64().context("action x missing")?;
    let y = point["y"].as_f64().context("action y missing")?;
    match action.kind {
        ActionKind::Hover => {
            cdp.send("Input.dispatchMouseEvent", json!({"type":"mouseMoved","x":x,"y":y}))
                .await?;
        }
        ActionKind::Focus => {
            cdp.evaluate(&format!(
                "document.querySelector({selector}).focus({{preventScroll:true}})",
                selector = selector_json
            ))
            .await?;
        }
        ActionKind::Input => {
            cdp.evaluate(&format!(
                "(()=>{{const element=document.querySelector({selector});\
                 const setter=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')?.set||\
                 Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,'value')?.set;\
                 setter.call(element,{});element.dispatchEvent(new InputEvent('input',{{bubbles:true,inputType:'insertText'}}));\
                 element.dispatchEvent(new Event('change',{{bubbles:true}}))}})()",
                serde_json::to_string(action.value.as_deref().unwrap_or("backtest"))?,
                selector = selector_json
            ))
            .await?;
        }
        ActionKind::Click => {
            for event_type in ["mousePressed", "mouseReleased"] {
                cdp.send(
                    "Input.dispatchMouseEvent",
                    json!({"type":event_type,"x":x,"y":y,"button":"left","clickCount":1}),
                )
                .await?;
            }
            if action.trigger.is_some() {
                cdp.evaluate("window.__backtestAdvance?.(0)").await?;
                let follow = format!(
                    "[data-backtest-id={}]",
                    serde_json::to_string(&action.target)?
                );
                cdp.evaluate(&format!(
                    "document.querySelector({})?.click()",
                    serde_json::to_string(&follow)?
                ))
                .await?;
            }
        }
        ActionKind::Escape | ActionKind::Timer => {}
    }
    cdp.evaluate("window.__backtestAdvance?.(0)").await?;
    Ok(())
}

pub async fn advance(cdp: &mut Cdp, delta_ms: u64) -> anyhow::Result<()> {
    cdp.evaluate(&format!("window.__backtestAdvance?.({delta_ms})"))
        .await?;
    Ok(())
}

pub async fn record_state(cdp: &mut Cdp, viewport: Viewport) -> anyhow::Result<StateEvidence> {
    reload(cdp, &viewport).await?;
    let baseline = snapshot(cdp).await?;
    let discovered = discover(cdp).await?;
    let mut scenarios = Vec::new();
    for (action, checkpoints) in discovered {
        reload(cdp, &viewport).await?;
        apply(cdp, &action).await?;
        let mut elapsed = 0;
        let mut captured = Vec::new();
        for checkpoint in checkpoints {
            let delta = checkpoint.saturating_sub(elapsed);
            advance(cdp, delta).await?;
            elapsed = checkpoint;
            captured.push(Checkpoint {
                virtual_ms: checkpoint,
                snapshot: snapshot(cdp).await?,
            });
        }
        scenarios.push(ScenarioEvidence {
            id: format!("{}:{}", action.kind.as_str(), action.target),
            action,
            checkpoints: captured,
        });
    }
    Ok(StateEvidence {
        viewport,
        baseline,
        scenarios,
    })
}

pub async fn replay_state(
    cdp: &mut Cdp,
    source: &StateEvidence,
) -> anyhow::Result<StateEvidence> {
    reload(cdp, &source.viewport).await?;
    let baseline = snapshot(cdp).await?;
    let mut scenarios = Vec::new();
    for scenario in &source.scenarios {
        reload(cdp, &source.viewport).await?;
        apply(cdp, &scenario.action).await?;
        let mut elapsed = 0;
        let mut checkpoints = Vec::new();
        for expected in &scenario.checkpoints {
            advance(cdp, expected.virtual_ms.saturating_sub(elapsed)).await?;
            elapsed = expected.virtual_ms;
            checkpoints.push(Checkpoint {
                virtual_ms: expected.virtual_ms,
                snapshot: snapshot(cdp).await?,
            });
        }
        scenarios.push(ScenarioEvidence {
            id: scenario.id.clone(),
            action: scenario.action.clone(),
            checkpoints,
        });
    }
    Ok(StateEvidence {
        viewport: source.viewport.clone(),
        baseline,
        scenarios,
    })
}

pub async fn prepare_session(
    side: &str,
    url: &str,
    endpoint: &str,
    viewport: Viewport,
    ready_selector: Option<&str>,
) -> anyhow::Result<(Session, browser::BrowserProcess)> {
    let process = browser::ensure(endpoint, side, true, None).await?;
    let target = browser::create(endpoint, url).await?;
    browser::activate(endpoint, &target.id).await?;
    let mut cdp = Cdp::connect(&target.websocket_url, Duration::from_secs(30)).await?;
    install(&mut cdp).await?;
    wait_ready(&mut cdp).await?;
    if let Some(selector) = ready_selector {
        for _ in 0..600 {
            let found = cdp
                .evaluate(&format!(
                    "!!document.querySelector({})",
                    serde_json::to_string(selector)?
                ))
                .await?
                .as_bool()
                .unwrap_or(false);
            if found {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    let rendered_url = cdp
        .evaluate("location.href")
        .await?
        .as_str()
        .unwrap_or(url)
        .to_string();
    let fingerprint = fingerprint(&mut cdp).await?;
    let browser = browser::version(endpoint).await?;
    Ok((
        Session {
            schema_version: SCHEMA_VERSION,
            side: side.into(),
            cdp_url: endpoint.into(),
            target_id: target.id,
            requested_url: url.into(),
            rendered_url,
            browser,
            viewport,
            fingerprint,
        },
        process,
    ))
}

pub async fn record(
    session: &Session,
    viewports: Vec<Viewport>,
) -> anyhow::Result<Artifact> {
    let (_, mut cdp) =
        browser::connect(&session.cdp_url, &session.target_id, Duration::from_secs(30)).await?;
    install(&mut cdp).await?;
    let current = fingerprint(&mut cdp).await?;
    anyhow::ensure!(
        current == session.fingerprint,
        "prepared source fingerprint changed"
    );
    let mut states = Vec::new();
    for viewport in viewports {
        states.push(record_state(&mut cdp, viewport).await?);
    }
    let mut artifact = Artifact {
        schema_version: SCHEMA_VERSION,
        source: SourceIdentity {
            requested_url: session.requested_url.clone(),
            rendered_url: session.rendered_url.clone(),
            browser: session.browser.clone(),
            fingerprint: session.fingerprint.clone(),
        },
        states,
        digest: String::new(),
    };
    artifact.digest = digest::json(&artifact)?;
    Ok(artifact)
}

pub fn read_session(path: &Path) -> anyhow::Result<Session> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub fn write_session(path: &Path, session: &Session) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(session)?)?;
    Ok(())
}

pub fn read_artifact(path: &Path) -> anyhow::Result<Artifact> {
    let artifact: Artifact = serde_json::from_slice(&fs::read(path)?)?;
    let mut unsigned = artifact.clone();
    let digest = unsigned.digest.clone();
    unsigned.digest.clear();
    anyhow::ensure!(digest::json(&unsigned)? == digest, "artifact digest mismatch");
    Ok(artifact)
}

pub fn write_artifact(path: &Path, artifact: &Artifact) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(artifact)?)?;
    Ok(())
}

pub async fn compare_candidate(
    deadline: Deadline,
    artifact: &Artifact,
    session: &Session,
) -> anyhow::Result<Vec<StateEvidence>> {
    let (_, mut cdp) = deadline
        .run(
            "candidate connection",
            browser::connect(&session.cdp_url, &session.target_id, deadline.remaining()),
        )
        .await?;
    cdp.set_timeout(deadline.remaining());
    deadline.run("candidate instrumentation", install(&mut cdp)).await?;
    let mut states = Vec::new();
    for source in &artifact.states {
        cdp.set_timeout(deadline.remaining());
        states.push(
            deadline
                .run("candidate replay", replay_state(&mut cdp, source))
                .await?,
        );
    }
    Ok(states)
}

pub fn parse_viewports(value: &str) -> anyhow::Result<Vec<Viewport>> {
    value
        .split(',')
        .map(|entry| {
            let (width, height) = entry
                .trim()
                .split_once('x')
                .context("viewport must be WIDTHxHEIGHT")?;
            Ok(Viewport {
                width: width.parse()?,
                height: height.parse()?,
            })
        })
        .collect()
}

pub fn save_screenshots(artifact: &Artifact, directory: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(directory)?;
    for state in &artifact.states {
        let bytes = STANDARD.decode(&state.baseline.screenshot_png)?;
        fs::write(
            directory.join(format!(
                "source-{}x{}.png",
                state.viewport.width, state.viewport.height
            )),
            bytes,
        )?;
    }
    Ok(())
}
