use super::*;

use super::project_test_support as support;

#[tokio::test]
async fn writes_semantic_component_project() {
    let directory = tempfile::tempdir().unwrap();
    let mut specification = support::specification();
    for state in &mut specification.states {
        state
            .attribute_sequences
            .push(crate::model::AttributeSequence {
                target: state.nodes[3].path.clone(),
                attribute: "data-copy".into(),
                values: vec!["First".into(), "Second".into()],
                interval_ms: 100,
                steps: Vec::new(),
            });
    }

    let components = super::tree::components(&specification, &Default::default());
    assert!(
        components
            .items
            .iter()
            .any(|item| item.name == "ResultCard"),
        "{:?}",
        components
            .items
            .iter()
            .map(|item| &item.name)
            .collect::<Vec<_>>()
    );
    write_project(&specification, directory.path(), &[])
        .await
        .unwrap();
    let root = directory.path().join("react");
    assert!(root.join("src/states.jsx").exists());
    let app = read_source_tree(&root.join("src"));
    assert!(app.contains("Interaction1"));
    assert!(app.contains("aria-expanded={\"false\"}"));
    assert!(app.contains("onKeyDown"));
    assert!(app.contains("data-recreate-trigger"));
    assert!(app.contains("document.querySelector"));
    assert!(app.contains("event.key==='Escape'"));
    assert!(app.contains("function Baseline0({activate,showStartup,onStartupDone})"));
    assert!(app.contains("baselineViews[viewport]({activate,showStartup:!startupDone"));
    assert!(app.contains("onStartupDone:()=>setStartupDone(true)"));
    let states = app.clone();
    assert!(app.contains("setAttribute('aria-expanded'"));
    assert!(
        states.contains("aria-modal={\"true\"}") || states.contains("[\"aria-modal\",\"true\"]")
    );
    assert!(states.contains("autoFocus") || app.contains("focusedTargets"));
    assert!(states.contains("createPortal"));
    assert!(!states.contains("<ReplacementSurface"));
    assert!(app.contains("const renderState=value=>value===1?"));
    assert!(app.contains("const contentState=closableStates[state]?returnState.current:state"));
    assert!(app.contains("const popup=closableStates[state]?renderState(state):null"));
    assert!(app.contains("const controlStyles="));
    assert!(app.contains("const baselineSelectedTokens="));
    assert!(app.contains("const baselinePressedTokens="));
    assert!(app.contains("const mergeStateScroll="));
    assert!(app.contains("const selectedState=useRef(baselineSelectedState)"));
    assert!(app.contains("replacementStates=[false,false]"));
    assert!(app.contains("mergeHorizontalScroll(captureScroll(event.currentTarget),captured)"));
    assert!(app.contains("live.get(path)?.[2]??0"));
    assert!(!app.contains("live.get(path)?.[2]??top"));
    assert!(app.contains("startSequences(document"));
    assert!(app.contains("data-recreate-sequence=\"0\""));
    assert!(app.contains("reduceInteraction({openSurface:state||null"));
    assert!(app.contains("stateful:statefulStates[next],closable:closableStates[next]"));
    assert!(app.contains("setState(command.surface)"));
    assert!(app.contains("const carouselPrevious="));
    assert!(app.contains("const carouselNext="));
    assert!(app.contains("if(!carouselState||!carouselPrevious||!carouselNext)return"));
    assert!(app.contains("initialScrolls[viewport]"));
    assert!(app.contains("data-recreate-active"));
    assert!(app.contains("restoreFocus.current=trigger"));
    assert!(app.contains("trigger.focus({preventScroll:true})"));
    assert!(app.contains("requestAnimationFrame(()=>requestAnimationFrame(()=>{trigger.focus"));
    assert!(app.contains("const preservePosition="));
    assert!(!app.contains("location.reload()"));
    assert!(!states.contains("const stableRoots=roots.map"));
    assert!(states.contains("existing.__recreateReplacement=token"));
    assert!(states.contains("existing.__recreateBaseline??"));
    assert!(states.contains("for(const[node]of baseline.children)"));
    assert!(states.contains("const[host]=React.useState"));
    assert!(!states.contains("setTarget(host)"));
    assert!(states.contains("const target=floating?document.body"));
    assert!(root.join("src/runtime/sequence.mjs").exists());
    assert!(root.join("src/runtime/interaction.mjs").exists());
    assert!(app.contains("smooth:true"));
    assert!(app.contains("(now-started)/320"));
    assert!(app.contains("scrollEase(progress)"));
    assert!(app.contains("target.focus({preventScroll:true})"));
    assert!(app.contains("const focusedTargets=[null,"));
    let css = read_css_tree(&root.join("src"));
    assert!(css.contains("@media(min-width:769px) and (max-width:1440px)"));
    assert!(css.contains("@media(min-width:391px) and (max-width:768px)"));
    assert!(css.contains("@media(min-width:321px) and (max-width:390px)"));
    assert!(css.contains("@media(max-width:320px)"));
    assert!(css.contains("content:\"mobile\";"));
    assert!(css.contains("color:blue;"));
    assert!(css.contains("content:none;"));
    assert!(css.contains("@keyframes"));
    assert!(!css.contains("[data-recreate-control]:focus-visible"));
}

#[tokio::test]
async fn text_entry_state_preserves_the_mounted_control() {
    let directory = tempfile::tempdir().unwrap();
    write_project(&support::text_entry_specification(), directory.path(), &[])
        .await
        .unwrap();
    let source = directory.path().join("react/src");
    let app = read_source_tree(&source);
    let states = app.clone();

    assert!(app.contains("inputActive===true&&state===next)return"));
    assert!(app.contains("closableStates=[false,false]"));
    assert!(app.contains("statefulStates=[false,true]"));
    assert!(app.contains("replacementStates=[false,false]"));
    assert!(states.contains("<ExistingSurface"), "{states}");
    assert!(states.contains("<InsertedSurface"));
    assert!(states.contains("hidden={"));
    assert!(states.contains("attributes={"));
    assert!(states.contains("detach={true}"));
    assert!(!states.contains("SuppressPortals"));
    assert!(!states.contains("<ReplacementSurface path={\"html>body:nth-of-type(1)>div:nth-of-type(1)>div:nth-of-type(1)>textarea"));
}

pub(super) fn read_source_tree(root: &std::path::Path) -> String {
    read_tree(root, &["js", "jsx", "mjs"])
}

pub(super) fn read_css_tree(root: &std::path::Path) -> String {
    read_tree(root, &["css"])
}

fn read_tree(root: &std::path::Path, extensions: &[&str]) -> String {
    let mut output = String::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            pending.extend(
                std::fs::read_dir(path)
                    .unwrap()
                    .filter_map(Result::ok)
                    .map(|entry| entry.path()),
            );
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
        {
            output.push_str(&std::fs::read_to_string(path).unwrap());
            output.push('\n');
        }
    }
    output
}
