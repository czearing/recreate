use super::group_constants;

#[test]
fn emits_a_declaration_shared_by_every_rule_once() {
    let css = ".a{color:red;float:none}\n.b{color:blue;float:none}\n";
    let grouped = group_constants(css);
    assert_eq!(grouped.matches("float:none").count(), 1);
    assert!(grouped.contains(".a,.b{float:none}"));
    assert!(grouped.contains(".a{color:red}"));
    assert!(grouped.contains(".b{color:blue}"));
}

#[test]
fn keeps_a_property_that_varies_anywhere_in_place() {
    let css = ".a{float:none}\n.b{float:left}\n.c{float:none}\n";
    let grouped = group_constants(css);
    assert_eq!(grouped.matches("float:none").count(), 2);
    assert!(grouped.contains(".b{float:left}"));
}

#[test]
fn leaves_a_conditional_declaration_inside_its_at_rule() {
    let css = ".a{float:none}\n.b{float:none}\n@media (max-width:600px){\n.c{float:none}\n}\n";
    let grouped = group_constants(css);
    assert!(grouped.contains("@media (max-width:600px){"));
    // The conditional copy must survive untouched: hoisting it would apply it at every width.
    let media = grouped.split("@media (max-width:600px){").nth(1).unwrap();
    assert!(media.contains(".c{float:none}"));
    assert!(!grouped.contains(".a,.b,.c{"));
}

#[test]
fn drops_a_rule_left_with_nothing_of_its_own() {
    let css = ".a{float:none}\n.b{float:none}\n";
    let grouped = group_constants(css);
    assert!(grouped.contains(".a,.b{float:none}"));
    assert!(!grouped.contains(".a{}"));
}

#[test]
fn leaves_a_stylesheet_with_no_repetition_alone() {
    let css = ".a{color:red}\n.b{display:flex}\n";
    assert_eq!(group_constants(css), css);
}
