use super::{source_item_dedupe, source_item_names};

#[test]
fn rejects_outer_containers_with_only_nested_controls() {
    let block = "<section>\n  <button role={\"button\"}>{\"Open a deliberately long nested control that would otherwise qualify the surrounding section for extraction\"}</button>\n  <div>{\"Additional static collection content\"}</div>\n</section>";
    assert!(!source_item_dedupe::reusable(block));
}

#[test]
fn derives_site_neutral_collection_names() {
    let source = &mut r#"<div data-testid={"product-card"} />"#.to_string();
    assert_eq!(
        source_item_names::collection_entity(&[source]).as_deref(),
        Some("Product")
    );
    assert_eq!(
        source_item_names::item_name(
            r#"<div data-testid={"product-card"}>"#,
            &[],
            Some("Product")
        ),
        "ProductCard"
    );
}
