use super::*;

#[path = "project_test_support.rs"]
mod support;

#[tokio::test]
async fn writes_semantic_component_project() {
    let directory = tempfile::tempdir().unwrap();
    let specification = support::specification();
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
    assert!(app.contains("function Baseline0({activate})"));
    assert!(app.contains("return <View activate={activate}/>"));
    let states = std::fs::read_to_string(root.join("src/states.jsx")).unwrap();
    assert!(states.contains("aria-expanded={\"true\"}"));
    assert!(states.contains("aria-modal={\"true\"}"));
    assert!(states.contains("autoFocus"));
    assert!(states.contains("event.key==='Escape'"));
    assert!(states.contains("onClick={event=>onReset(event)}"));
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
