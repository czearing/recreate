use super::super::{source_dedupe_support::reusable_block, source_free_names::free_names};

fn names(fragment: &str) -> Vec<String> {
    free_names(fragment).into_iter().collect()
}

#[test]
fn sees_a_component_the_fragment_renders_but_does_not_define() {
    assert_eq!(
        names("<Row>\n  <Label>{\"a\"}</Label>\n</Row>"),
        ["Label", "Row"]
    );
}

#[test]
fn reads_only_the_leftmost_segment_of_a_member_tag() {
    assert_eq!(names("<CollectionItems.Row />"), ["CollectionItems"]);
}

#[test]
fn leaves_intrinsic_elements_alone() {
    assert!(names("<div className={\"a\"}><span>{\"b\"}</span></div>").is_empty());
}

#[test]
fn sees_a_handler_named_inside_an_inline_arrow() {
    let fragment = "<div onKeyDown={event=>keyActivate(event,activate)} />";
    assert_eq!(names(fragment), ["activate", "keyActivate"]);
}

#[test]
fn treats_an_arrow_parameter_as_bound_by_the_fragment() {
    assert!(!free_names("<div ref={element=>element?.focus()} />").contains("element"));
}

#[test]
fn treats_the_contents_of_a_string_as_text_rather_than_code() {
    assert!(free_names("<div title={\"press activate to open\"} />").is_empty());
}

#[test]
fn does_not_let_a_brace_inside_a_string_end_the_expression_it_sits_in() {
    assert!(
        free_names("<div title={\"close with }\" + describe(row)} />").contains("describe"),
        "a brace inside a string must not cut the region short and hide the names after it"
    );
}

#[test]
fn treats_a_property_read_as_belonging_to_its_object() {
    assert!(!free_names("<div hidden={state.open} />").contains("open"));
}

#[test]
fn keeps_the_shared_block_gate_refusing_a_document_scoped_trigger() {
    let block = format!(
        "<div data-recreate-trigger={{\"7\"}}>\n{}\n</div>",
        "  <span>{\"padding\"}</span>\n".repeat(80)
    );
    assert!(!reusable_block(&block));
}

#[test]
fn refuses_a_shared_block_naming_a_component_it_cannot_bind() {
    let block = format!(
        "<div>\n{}\n</div>",
        "  <Label>{\"padding\"}</Label>\n".repeat(80)
    );
    assert!(!reusable_block(&block));
}

#[test]
fn lets_the_shared_block_gate_accept_markup_that_binds_everything_it_names() {
    let block = format!(
        "<div>\n{}\n</div>",
        "  <span>{\"padding\"}</span>\n".repeat(80)
    );
    assert!(reusable_block(&block));
}
