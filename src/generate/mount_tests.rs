use super::{document, mount};

#[test]
fn mounts_at_body_without_adding_a_wrapper() {
    let (source, markup) = mount(false, "").unwrap();
    assert_eq!(source, "createRoot(document.body).render(<App />);");
    assert!(markup.is_empty());
}

#[test]
fn preserves_an_existing_root_element() {
    let (source, markup) = mount(true, "root-class").unwrap();
    assert!(source.contains("document.getElementById('root')"));
    assert!(source.contains("root.className=\"root-class\""));
    assert_eq!(markup, "<div id=\"root\"></div>");
}

#[test]
fn generated_document_avoids_implicit_favicon_requests() {
    let html = document("Example", "");
    assert!(html.contains("<link rel=\"icon\" href=\"data:,\">"));
}
