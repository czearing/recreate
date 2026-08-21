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
fn shares_a_value_even_though_the_property_varies_elsewhere() {
    let css = ".a{float:none}\n.b{float:left}\n.c{float:none}\n";
    let grouped = group_constants(css);
    // One rule disagreeing must not cost every rule that agrees: `.a` and `.c` still share.
    assert_eq!(grouped.matches("float:none").count(), 1);
    assert!(grouped.contains(".a,.c{float:none}"));
    assert!(grouped.contains(".b{float:left}"));
}

#[test]
fn leaves_a_property_two_rules_on_one_selector_disagree_on_in_place() {
    // Which value `.a` computes depends on rule order, so moving either copy to the front could
    // change the winner. Only `.b` and `.c`, which no second rule contests, may share.
    let css = ".a{color:red}\n.a{color:blue}\n.b{color:blue}\n.c{color:blue}\n";
    let grouped = group_constants(css);
    assert!(grouped.contains(".b,.c{color:blue}"));
    assert!(grouped.contains(".a{color:red}"));
    assert!(grouped.contains(".a{color:blue}"));
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
