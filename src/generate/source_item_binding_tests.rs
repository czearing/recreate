use super::{source_item_component::render, source_item_dedupe::extract};
use std::collections::BTreeSet;

fn exports(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

fn page(tag: &str) -> String {
    format!(
        "<main>\n  <{tag} data-testid={{\"row\"}} role={{\"button\"}}>\n    \
         <Label>{{\"FIRSTROWTITLETOKENWITHLENGTH\"}}</Label>\n    \
         <Label>{{\"FIRSTROWCAPTIONTOKENWITHLENGTH\"}}</Label>\n  </{tag}>\n  \
         <{tag} data-testid={{\"row\"}} role={{\"button\"}}>\n    \
         <Label>{{\"SECONDROWTITLETOKENWITHLENGTH\"}}</Label>\n    \
         <Label>{{\"SECONDROWCAPTIONTOKENWITHLENGTH\"}}</Label>\n  </{tag}>\n</main>"
    )
}

#[test]
fn imports_a_component_whose_name_begins_with_the_word_generated() {
    let module = render(
        "Row",
        "<GeneratedRow>{title}</GeneratedRow>",
        &[(0, "title".into())],
        &exports(&["GeneratedRow"]),
    );
    assert!(module.contains("import {GeneratedRow} from '../index.js';"));
}

#[test]
fn imports_every_component_the_lifted_markup_renders() {
    let module = render(
        "Row",
        "<GeneratedRow><Label>{title}</Label></GeneratedRow>",
        &[(0, "title".into())],
        &exports(&["GeneratedRow", "Label"]),
    );
    assert!(module.contains("import {GeneratedRow,Label} from '../index.js';"));
}

#[test]
fn never_imports_the_component_it_is_defining() {
    let module = render(
        "Row",
        "<Row>{title}</Row>",
        &[(0, "title".into())],
        &exports(&["Row"]),
    );
    assert!(!module.contains("import {Row}"));
}

#[test]
fn never_imports_a_component_the_destination_does_not_export() {
    let module = render(
        "Row",
        "<ExistingSurface>{title}</ExistingSurface>",
        &[(0, "title".into())],
        &exports(&["Label"]),
    );
    assert!(!module.contains("ExistingSurface}"));
    assert!(!module.contains("import {"));
}

#[test]
fn lifts_a_repeated_item_whose_components_the_destination_exports() {
    let mut source = page("GeneratedRow");
    let items = extract(&mut [&mut source], &exports(&["GeneratedRow", "Label"]));
    assert_eq!(items.len(), 1);
    assert!(source.contains("<CollectionItems."));
}

#[test]
fn refuses_a_repeated_item_naming_a_component_the_destination_cannot_resolve() {
    let mut source = page("ExistingSurface");
    let items = extract(&mut [&mut source], &exports(&["Label"]));
    assert!(items.is_empty());
    assert!(!source.contains("<CollectionItems."));
}

#[test]
fn refuses_a_repeated_item_carrying_a_handler_identifier() {
    let mut source = page("GeneratedRow").replace(
        "role={\"button\"}>",
        "role={\"button\"} onKeyDown={event=>keyActivate(event,activate)}>",
    );
    let items = extract(&mut [&mut source], &exports(&["GeneratedRow", "Label"]));
    assert!(items.is_empty());
    assert!(!source.contains("<CollectionItems."));
}

#[test]
fn emits_no_module_naming_anything_it_neither_binds_nor_imports() {
    let mut source = page("GeneratedRow");
    let items = extract(&mut [&mut source], &exports(&["GeneratedRow", "Label"]));
    assert_eq!(items.len(), 1);
    for item in &items {
        let head = item.source.lines().take_while(|line| !line.is_empty());
        let mut bound = head
            .flat_map(|line| line.split(['{', '}', ',', ' ']))
            .map(|name| name.trim().to_string())
            .collect::<BTreeSet<_>>();
        bound.insert(item.name.clone());
        bound.extend(
            item.source
                .split_once(&format!("export function {}({{", item.name))
                .and_then(|(_, rest)| rest.split_once("})"))
                .into_iter()
                .flat_map(|(props, _)| props.split(','))
                .map(|prop| prop.trim().to_string()),
        );
        for name in super::source_free_names::free_names(
            item.source
                .split_once("return (")
                .and_then(|(_, rest)| rest.rsplit_once("  );"))
                .expect("rendered body")
                .0,
        ) {
            assert!(
                bound.contains(&name),
                "{} names {name} without binding it:\n{}",
                item.name,
                item.source
            );
        }
    }
}
