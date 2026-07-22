use crate::{browser, cdp::Cdp, cli::FidelityArgs, fidelity_responsive, fidelity_script};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    nodes: Vec<NodeSnapshot>,
    animations: Vec<AnimationSnapshot>,
    document: [f64; 2],
    root_hovered: bool,
    hit_path: Option<String>,
    visibility: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeSnapshot {
    path: String,
    tag: String,
    class_name: String,
    text: String,
    rect: [f64; 4],
    style: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnimationSnapshot {
    target: String,
    pseudo: Option<String>,
    current_time: f64,
    duration: f64,
    delay: f64,
    easing: String,
    properties: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize)]
struct Frame {
    elapsed_ms: u64,
    snapshot: Snapshot,
}

#[derive(Clone, Debug, Serialize)]
struct Trace {
    label: String,
    hover: Vec<Frame>,
    leave: Vec<Frame>,
}

#[derive(Debug, Serialize)]
struct Report {
    passed: bool,
    source: Trace,
    candidate: Trace,
    responsive_source: Vec<fidelity_responsive::ResponsiveFrame>,
    responsive_candidate: Vec<fidelity_responsive::ResponsiveFrame>,
    details: Vec<String>,
}

pub async fn run(args: FidelityArgs) -> Result<()> {
    reset(&args, &args.source_target).await?;
    reset(&args, &args.candidate_target).await?;
    let source = trace(&args, &args.source_target).await?;
    let candidate = trace(&args, &args.candidate_target).await?;
    let text_lock = fidelity_responsive::text_map(&args, &args.source_target).await?;
    let responsive_source =
        fidelity_responsive::trace(&args, &args.source_target, &text_lock).await?;
    let responsive_candidate =
        fidelity_responsive::trace(&args, &args.candidate_target, &text_lock).await?;
    let mut details = compare(&source, &candidate);
    details.extend(fidelity_responsive::compare(
        &responsive_source,
        &responsive_candidate,
    ));
    details.truncate(100);
    let report = Report {
        passed: details.is_empty(),
        source,
        candidate,
        responsive_source,
        responsive_candidate,
        details,
    };
    if let Some(path) = &args.output {
        std::fs::write(path, serde_json::to_vec_pretty(&report)?)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    let summary_details = report.details.iter().take(20).collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "passed": report.passed,
            "detail_count": report.details.len(),
            "details": summary_details,
            "source_label": report.source.label,
            "candidate_label": report.candidate.label,
            "output": args.output,
        }))?
    );
    anyhow::ensure!(report.passed, "hover fidelity mismatch");
    Ok(())
}

async fn reset(args: &FidelityArgs, target_id: &str) -> Result<()> {
    let target = browser::list(&args.cdp_url)
        .await?
        .into_iter()
        .find(|value| value.id == target_id)
        .with_context(|| format!("target not found: {target_id}"))?;
    let mut cdp = Cdp::connect(&target.websocket_url).await?;
    cdp.enable(&["Page", "Runtime"]).await?;
    cdp.send("Page.reload", json!({})).await?;
    for _ in 0..80 {
        let ready = cdp
            .evaluate(
                "document.readyState==='complete'&&document.body&&document.body.children.length>0",
            )
            .await?
            .as_bool()
            .unwrap_or(false);
        if ready {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    anyhow::bail!("target did not reload: {target_id}")
}

async fn trace(args: &FidelityArgs, target: &str) -> Result<Trace> {
    let target = browser::list(&args.cdp_url)
        .await?
        .into_iter()
        .find(|value| value.id == target)
        .with_context(|| format!("target not found: {target}"))?;
    let mut cdp = Cdp::connect(&target.websocket_url).await?;
    cdp.enable(&["Page", "Runtime"]).await?;
    cdp.send("Page.bringToFront", json!({})).await?;
    browser::set_viewport(&mut cdp, args.width, args.height).await?;
    cdp.send(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"no-preference"}]}),
    )
    .await?;
    move_pointer(&mut cdp, -100.0, -100.0).await?;
    press_escape(&mut cdp).await?;
    click_pointer(
        &mut cdp,
        f64::from(args.width.saturating_sub(20)),
        f64::from(args.height.saturating_sub(20)),
    )
    .await?;
    press_escape(&mut cdp).await?;
    settle(&mut cdp, 50).await;
    let mut descriptor = cdp.evaluate(&fidelity_script::prepare(&args.label)).await?;
    if descriptor["x"].as_f64().is_none() {
        cdp.send("Page.reload", json!({})).await?;
        for _ in 0..20 {
            settle(&mut cdp, 250).await;
            descriptor = cdp.evaluate(&fidelity_script::prepare(&args.label)).await?;
            if descriptor["x"].as_f64().is_some() {
                break;
            }
        }
    }
    descriptor["x"].as_f64().with_context(|| {
        format!(
            "hover target {:?} not found on {} ({})",
            args.label, target.id, target.url,
        ) + &format!(": {descriptor}")
    })?;
    descriptor["y"].as_f64().context("hover target missing y")?;
    wait_interactable(&mut cdp).await?;
    descriptor = cdp.evaluate(&fidelity_script::prepare(&args.label)).await?;
    descriptor["x"].as_f64().context("hover target missing x")?;
    descriptor["y"].as_f64().context("hover target missing y")?;
    let mut hover = vec![frame(&mut cdp, 0).await?];
    activate_hover(&mut cdp).await?;
    sample(&mut cdp, &mut hover, &[0, 16, 16, 32, 56, 80, 120]).await?;
    move_pointer(&mut cdp, -100.0, -100.0).await?;
    let mut leave = Vec::new();
    sample(&mut cdp, &mut leave, &[0, 16, 16, 32, 56, 80, 120]).await?;
    Ok(Trace {
        label: descriptor["label"].as_str().unwrap_or_default().into(),
        hover,
        leave,
    })
}

async fn activate_hover(cdp: &mut Cdp) -> Result<()> {
    for _ in 0..40 {
        let point = cdp
            .evaluate(
                "(()=>{const root=window.__recreateFidelityRoot;\
                 const rect=root.getBoundingClientRect();return {\
                 x:rect.x+rect.width/2,y:rect.y+rect.height/2}})()",
            )
            .await?;
        move_pointer(
            cdp,
            point["x"].as_f64().context("hover point missing x")?,
            point["y"].as_f64().context("hover point missing y")?,
        )
        .await?;
        if cdp
            .evaluate("window.__recreateFidelityRoot.matches(':hover')")
            .await?
            .as_bool()
            .unwrap_or(false)
        {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    anyhow::bail!("fidelity hover did not activate")
}

async fn wait_interactable(cdp: &mut Cdp) -> Result<()> {
    for _ in 0..480 {
        let interactable = cdp
            .evaluate(
                "(()=>{const root=window.__recreateFidelityRoot;if(!root)return false;\
                 const rect=root.getBoundingClientRect();const hit=document.elementFromPoint(\
                 rect.x+rect.width/2,rect.y+rect.height/2);return !!hit&&root.contains(hit)})()",
            )
            .await?
            .as_bool()
            .unwrap_or(false);
        if interactable {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    anyhow::bail!("fidelity target remained covered")
}

async fn sample(cdp: &mut Cdp, frames: &mut Vec<Frame>, delays: &[u64]) -> Result<()> {
    let mut elapsed = 0;
    for delay in delays {
        settle(cdp, *delay).await;
        elapsed += delay;
        frames.push(frame(cdp, elapsed).await?);
    }
    Ok(())
}

async fn frame(cdp: &mut Cdp, elapsed_ms: u64) -> Result<Frame> {
    Ok(Frame {
        elapsed_ms,
        snapshot: serde_json::from_value(cdp.evaluate(fidelity_script::SNAPSHOT).await?)?,
    })
}

async fn move_pointer(cdp: &mut Cdp, x: f64, y: f64) -> Result<()> {
    cdp.send(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseMoved","x":x,"y":y}),
    )
    .await?;
    Ok(())
}

async fn click_pointer(cdp: &mut Cdp, x: f64, y: f64) -> Result<()> {
    move_pointer(cdp, x, y).await?;
    for event_type in ["mousePressed", "mouseReleased"] {
        cdp.send(
            "Input.dispatchMouseEvent",
            json!({
                "type": event_type,
                "x": x,
                "y": y,
                "button": "left",
                "clickCount": 1
            }),
        )
        .await?;
    }
    Ok(())
}

async fn press_escape(cdp: &mut Cdp) -> Result<()> {
    for event_type in ["keyDown", "keyUp"] {
        cdp.send(
            "Input.dispatchKeyEvent",
            json!({
                "type": event_type,
                "key": "Escape",
                "code": "Escape",
                "windowsVirtualKeyCode": 27,
                "nativeVirtualKeyCode": 27
            }),
        )
        .await?;
    }
    Ok(())
}

async fn settle(_cdp: &mut Cdp, delay: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
}

fn compare(source: &Trace, candidate: &Trace) -> Vec<String> {
    let mut details = Vec::new();
    compare_phase("hover", &source.hover, &candidate.hover, &mut details);
    compare_phase("leave", &source.leave, &candidate.leave, &mut details);
    details
}

fn compare_phase(name: &str, source: &[Frame], candidate: &[Frame], details: &mut Vec<String>) {
    let source_motion = motion(source);
    let candidate_motion = motion(candidate);
    let mut active_styles = style_changes(source);
    for (path, properties) in style_changes(candidate) {
        active_styles.entry(path).or_default().extend(properties);
    }
    for (path, properties) in &source_motion {
        match candidate_motion.get(path) {
            None => details.push(format!("{name}: missing animated target {path}")),
            Some(actual) if actual != properties => details.push(format!(
                "{name}: properties {path}: source={properties:?} candidate={actual:?}"
            )),
            _ => {}
        }
    }
    if let (Some(expected), Some(actual)) = (source.first(), candidate.first()) {
        compare_frame(name, expected, actual, &active_styles, details);
    }
    if let (Some(expected), Some(actual)) = (source.last(), candidate.last()) {
        compare_frame(name, expected, actual, &active_styles, details);
    }
    compare_geometry_ranges(name, source, candidate, details);
}

fn motion(frames: &[Frame]) -> BTreeMap<String, BTreeSet<String>> {
    let mut values = BTreeMap::<String, BTreeSet<String>>::new();
    for animation in frames.iter().flat_map(|frame| &frame.snapshot.animations) {
        for property in &animation.properties {
            values
                .entry(animation.target.clone())
                .or_default()
                .insert(format!(
                    "{property}|{:.2}|{:.2}|{}|{}",
                    animation.duration,
                    animation.delay,
                    animation.easing,
                    animation.pseudo.as_deref().unwrap_or_default()
                ));
        }
    }
    values
}

fn style_changes(frames: &[Frame]) -> BTreeMap<String, BTreeSet<String>> {
    let Some((first, last)) = frames.first().zip(frames.last()) else {
        return BTreeMap::new();
    };
    let last: BTreeMap<_, _> = last
        .snapshot
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    let mut changes = BTreeMap::<String, BTreeSet<String>>::new();
    for node in &first.snapshot.nodes {
        let Some(final_node) = last.get(node.path.as_str()) else {
            continue;
        };
        for property in [
            "opacity",
            "color",
            "backgroundColor",
            "boxShadow",
            "fill",
            "stroke",
        ] {
            if node.style.get(property) != final_node.style.get(property) {
                changes
                    .entry(node.path.clone())
                    .or_default()
                    .insert(property.into());
            }
        }
    }
    changes
}

fn compare_geometry_ranges(
    name: &str,
    source: &[Frame],
    candidate: &[Frame],
    details: &mut Vec<String>,
) {
    let expected = geometry_ranges(source);
    let actual = geometry_ranges(candidate);
    for (path, expected_range) in expected {
        let Some(actual_range) = actual.get(&path) else {
            continue;
        };
        let delta = expected_range
            .iter()
            .zip(actual_range)
            .map(|(left, right)| (left - right).abs())
            .fold(0.0, f64::max);
        if delta > 1.0 {
            details.push(format!(
                "{name}: geometry trajectory {path} delta={delta:.2}"
            ));
        }
    }
}

fn geometry_ranges(frames: &[Frame]) -> BTreeMap<String, [f64; 8]> {
    let mut ranges = BTreeMap::new();
    let origin = frames
        .first()
        .and_then(|frame| frame.snapshot.nodes.iter().find(|node| node.path == "."))
        .map(|node| [node.rect[0], node.rect[1]])
        .unwrap_or_default();
    for node in frames.iter().flat_map(|frame| &frame.snapshot.nodes) {
        let range = ranges
            .entry(node.path.clone())
            .or_insert([f64::INFINITY; 8]);
        let rect = [
            node.rect[0] - origin[0],
            node.rect[1] - origin[1],
            node.rect[2],
            node.rect[3],
        ];
        for (index, value) in rect.iter().enumerate() {
            range[index] = range[index].min(*value);
            let maximum = index + 4;
            range[maximum] = if range[maximum].is_infinite() {
                *value
            } else {
                range[maximum].max(*value)
            };
        }
    }
    ranges
}

fn compare_frame(
    name: &str,
    source: &Frame,
    candidate: &Frame,
    active_styles: &BTreeMap<String, BTreeSet<String>>,
    details: &mut Vec<String>,
) {
    let expected_root = source
        .snapshot
        .nodes
        .iter()
        .find(|node| node.path == ".")
        .map(|node| [node.rect[0], node.rect[1]])
        .unwrap_or_default();
    let actual_root = candidate
        .snapshot
        .nodes
        .iter()
        .find(|node| node.path == ".")
        .map(|node| [node.rect[0], node.rect[1]])
        .unwrap_or_default();
    if source.snapshot.root_hovered != candidate.snapshot.root_hovered {
        details.push(format!(
            "{name}: hover activation at {}ms source={} candidate={} candidate_hit={:?}",
            source.elapsed_ms,
            source.snapshot.root_hovered,
            candidate.snapshot.root_hovered,
            candidate.snapshot.hit_path
        ));
    }
    let actual: BTreeMap<_, _> = candidate
        .snapshot
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    for expected in &source.snapshot.nodes {
        let Some(node) = actual.get(expected.path.as_str()) else {
            details.push(format!("{name}: missing node {}", expected.path));
            continue;
        };
        let expected_rect = [
            expected.rect[0] - expected_root[0],
            expected.rect[1] - expected_root[1],
            expected.rect[2],
            expected.rect[3],
        ];
        let actual_rect = [
            node.rect[0] - actual_root[0],
            node.rect[1] - actual_root[1],
            node.rect[2],
            node.rect[3],
        ];
        let delta = expected_rect
            .iter()
            .zip(actual_rect)
            .map(|(left, right)| (left - right).abs())
            .fold(0.0, f64::max);
        if delta > 1.0 {
            details.push(format!(
                "{name}: geometry {} at {}ms delta={delta:.2}",
                expected.path, source.elapsed_ms
            ));
        }
        let mut compared = BTreeSet::from([
            "opacity",
            "color",
            "backgroundColor",
            "boxShadow",
            "fill",
            "stroke",
            "borderTopColor",
            "borderRightColor",
            "borderBottomColor",
            "borderLeftColor",
        ]);
        compared.extend(
            active_styles
                .get(&expected.path)
                .into_iter()
                .flatten()
                .map(String::as_str),
        );
        for property in compared {
            if expected.style.get(property) != node.style.get(property) {
                details.push(format!(
                    "{name}: style {} {property} at {}ms",
                    expected.path, source.elapsed_ms
                ));
            }
        }
        if expected.text != node.text {
            details.push(format!(
                "{name}: text {} at {}ms source={:?} candidate={:?}",
                expected.path, source.elapsed_ms, expected.text, node.text
            ));
        }
    }
    if source.snapshot.document != candidate.snapshot.document {
        details.push(format!("{name}: document geometry changed"));
    }
    details.truncate(100);
}

#[cfg(test)]
#[path = "fidelity_tests.rs"]
mod tests;
