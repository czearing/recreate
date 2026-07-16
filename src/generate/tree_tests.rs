use super::*;

#[test]
fn identifies_dynamic_attributes() {
    assert!(dynamic_attribute("href"));
    assert!(dynamic_attribute("role"));
    assert!(dynamic_attribute("aria-expanded"));
    assert!(!dynamic_attribute("style"));
}
