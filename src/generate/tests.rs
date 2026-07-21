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
    let app = std::fs::read_to_string(root.join("src/App.jsx")).unwrap();
    assert!(app.contains("Interaction1"));
    assert!(app.contains("aria-expanded={\"false\"}"));
    assert!(app.contains("onKeyDown"));
    assert!(app.contains("data-recreate-trigger"));
    assert!(app.contains("document.querySelector"));
    assert!(app.contains("event.key==='Escape'"));
    assert!(app.contains("function Baseline0({activate,showStartup,onStartupDone})"));
    assert!(app.contains("onStartupDone={()=>setStartupDone(true)}"));
    let states = std::fs::read_to_string(root.join("src/states.jsx")).unwrap();
    assert!(app.contains("setAttribute('aria-expanded'"));
    assert!(states.contains("aria-modal={\"true\"}"));
    assert!(states.contains("autoFocus"));
    assert!(states.contains("createPortal"));
    assert!(app.contains("const overlay=state===1?"));
    assert!(app.contains("mergeHorizontalScroll(captureScroll(event.currentTarget),captured)"));
    assert!(app.contains("live.get(path)?.[2]??0"));
    assert!(!app.contains("live.get(path)?.[2]??top"));
    assert!(app.contains("startSequences(document"));
    assert!(app.contains("data-recreate-sequence=\"0\""));
    assert!(app.contains("reduceInteraction({openSurface:state||null"));
    assert!(app.contains("setState(command.surface)"));
    assert!(app.contains("moveCarousel({offset:0,extent:offset},'forward')"));
    assert!(app.contains("initialScrolls[viewport]"));
    assert!(app.contains("data-recreate-active"));
    assert!(app.contains("restoreFocus.current=trigger"));
    assert!(app.contains("trigger.focus({preventScroll:true})"));
    assert!(root.join("src/runtime/sequence.mjs").exists());
    assert!(root.join("src/runtime/interaction.mjs").exists());
    assert!(app.contains("smooth:true"));
    assert!(app.contains("(now-started)/320"));
    assert!(app.contains("scrollEase(progress)"));
    assert!(app.contains("target.focus({preventScroll:true})"));
    let css = std::fs::read_to_string(root.join("src/styles.css")).unwrap();
    assert!(css.contains("@media(min-width:769px) and (max-width:1440px)"));
    assert!(css.contains("@media(min-width:391px) and (max-width:768px)"));
    assert!(css.contains("@media(min-width:321px) and (max-width:390px)"));
    assert!(css.contains("@media(max-width:320px)"));
    assert!(css.contains("content:\"mobile\";color:blue;"));
    assert!(css.contains("content:none;"));
    assert!(css.contains("@keyframes"));
    assert!(css.contains("[data-recreate-control]:focus-visible"));
}

#[tokio::test]
async fn text_entry_state_preserves_the_mounted_control() {
    let directory = tempfile::tempdir().unwrap();
    write_project(&support::text_entry_specification(), directory.path(), &[])
        .await
        .unwrap();
    let source = directory.path().join("react/src");
    let app = std::fs::read_to_string(source.join("App.jsx")).unwrap();
    let states = std::fs::read_to_string(source.join("states.jsx")).unwrap();

    assert!(app.contains("inputActive===true&&state===next)return"));
    assert!(app.contains("closableStates=[false,false]"));
    assert!(app.contains("statefulStates=[false,true]"));
    assert!(app.contains("replacementStates=[false,false]"));
    assert!(states.contains("<ExistingSurface"), "{states}");
    assert!(states.contains("<InsertedSurface"));
    assert!(states.contains("hidden={"));
    assert!(!states.contains("<ReplacementSurface path={\"html>body:nth-of-type(1)>div:nth-of-type(1)>div:nth-of-type(1)>textarea"));
}
