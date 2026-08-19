//! The other half of the invariant in [`super::tests`], stated the same way: a rule declared in
//! order to measure the page has to be in force wherever the read looks — and being *declared*
//! in a scope is not being *in force* there.
//!
//! Reach and weight are independent. A rule the page out-ranks is a rule that was delivered and
//! did nothing, and nothing reports it: the read succeeds and hands back the page's own live
//! values, which is exactly what a page with nothing to say looks like. A measurement rule
//! therefore has to sit where the page's own declarations cannot outrank it, however they are
//! written — and a selector cannot be made unbeatable, because whatever weight it is given, one
//! more compound beats it.

use super::double::evaluate;

/// The defect. A page that declares its own layer must not be able to order its declarations
/// ahead of the reading's, and the assertion is phrased against the page's own text rather than
/// against a name this test picked.
#[test]
fn the_reading_is_declared_ahead_of_every_layer_the_page_declares() {
    let order = evaluate(
        "document.styleSheets = [sheet('@layer page;')];\
         \nconst seen = underRules('R', () => layerOrder(document));",
        "seen",
    );
    let names = order.as_array().expect("a layer order");
    assert_eq!(
        names.len(),
        2,
        "the reading declares exactly one layer of its own: {order}"
    );
    assert_ne!(
        names[0], "page",
        "a layer the page declared is ordered first, so any important declaration in it \
         outranks the measurement whatever the measurement's selector is: {order}"
    );
}

/// Claiming the position is worth nothing if the rules are not in the layer that holds it. This
/// is the half a mutant can delete while leaving the claim in place and the order assertion
/// still green.
#[test]
fn the_declared_rules_are_the_ones_the_claimed_position_carries() {
    let held = evaluate(
        "document.styleSheets = [sheet('@layer page;')];\
         \nconst first = underRules('R', () => layerOrder(document))[0];\
         \nconst seen = underRules('R', () => inForce()[0][0]);",
        "[first, seen]",
    );
    let pair = held
        .as_array()
        .expect("a claimed name and the delivered text");
    let claimed = pair[0].as_str().expect("a claimed layer name");
    let text = pair[1].as_str().expect("the delivered text");
    assert!(
        text.contains(claimed) && text.contains('R'),
        "the rules must be assigned to the layer whose position was claimed: {text}"
    );
}

/// Every scope sorts its own layers, so a claim made in the document answers for the document
/// alone. This is the weight axis of the reach invariant the sibling file states.
#[test]
fn every_scope_is_claimed_and_not_only_the_one_the_walk_started_in() {
    let orders = evaluate(
        "for (const each of everyScope) each.styleSheets = [sheet('@layer page;')];\
         \nconst seen = underRules('R', () => everyScope.map(layerOrder));",
        "seen",
    );
    for order in orders.as_array().expect("one layer order per scope") {
        let names = order.as_array().expect("a layer order");
        assert_ne!(
            names[0], "page",
            "a scope the read reaches but the claim did not is measured under a rule the page \
             outranks: {orders}"
        );
    }
}

/// A measuring pass may not cost the page its own cascade. The claim adds a name; it must not
/// reorder, drop or merge the layers that were already there.
#[test]
fn the_layers_the_page_declared_keep_the_order_it_declared_them_in() {
    let order = evaluate(
        "document.styleSheets = [sheet('@layer first;', '@layer second;')];\
         \nconst seen = underRules('R', () => layerOrder(document));",
        "seen.slice(-2)",
    );
    assert_eq!(order, serde_json::json!(["first", "second"]));
}

/// Nothing may be inserted ahead of an `@import`, and CSSOM says so by throwing. A claim that
/// took the refusal for a failure would leave the whole sheet unclaimed; the earliest position
/// the grammar accepts is still earlier than every later sheet.
#[test]
fn a_sheet_led_by_an_import_is_claimed_at_the_first_position_it_accepts() {
    let order = evaluate(
        "document.styleSheets = [\
         \n  sheet('@import url(a.css);', '@layer page;'),\
         \n  sheet('@layer later;')\
         \n];\
         \nconst seen = underRules('R', () => layerOrder(document));",
        "seen",
    );
    let names = order.as_array().expect("a layer order");
    assert_ne!(
        names[0], "page",
        "the claim gave up on the sheet instead of taking the next position in it: {order}"
    );
}

/// A sheet the page loaded from another origin hands back nothing and is not the reading's to
/// write to. It is one sheet to pass over, not a reason to stop claiming.
#[test]
fn a_sheet_that_will_not_be_read_is_passed_over_rather_than_fatal() {
    let order = evaluate(
        "document.styleSheets = [unreadableSheet(), sheet('@layer page;')];\
         \nconst seen = underRules('R', () => layerOrder(document));",
        "seen",
    );
    let names = order.as_array().expect("a layer order");
    assert_ne!(names[0], "page", "{order}");
}

/// The page is left holding exactly the rules it was found holding. A claim left behind is a
/// permanent change to the page's cascade that every later stage then reads as the page's own.
#[test]
fn every_claimed_sheet_is_left_holding_what_it_was_found_holding() {
    let held = evaluate(
        "for (const each of everyScope) each.styleSheets = [sheet('@layer page;')];\
         \nunderRules('R', () => null);",
        "rulesHeld()",
    );
    assert_eq!(
        held,
        serde_json::json!([
            ["@layer page;"],
            ["@layer page;"],
            ["@layer page;"],
            ["@layer page;"]
        ])
    );
}

/// What is withdrawn is this reading's own rule, identified by which rule it is rather than by
/// where it sat when it was inserted. A read runs the page's own code, and a page that inserts
/// a rule of its own while the read is in flight would otherwise lose it and keep the claim.
#[test]
fn a_rule_the_page_inserts_during_the_read_outlives_the_reading() {
    let held = evaluate(
        "document.styleSheets = [sheet('@layer page;')];\
         \nunderRules('R', () => {\
         \n  document.styleSheets[0].insertRule('.late{}', 0);\
         \n});",
        "rulesHeld()[0]",
    );
    assert_eq!(held, serde_json::json!([".late{}\n@layer page;"]));
}

/// A read that throws is still a read that ended, and a page left under a measurement rule is a
/// page no later stage can read. The claim is withdrawn on the same terms as the sheet.
#[test]
fn a_read_that_throws_still_releases_every_claim() {
    let held = evaluate(
        "document.styleSheets = [sheet('@layer page;')];\
         \ntry { underRules('R', () => { throw new Error('read failed'); }); } catch (failed) {}",
        "rulesHeld()[0]",
    );
    assert_eq!(held, serde_json::json!(["@layer page;"]));
}
