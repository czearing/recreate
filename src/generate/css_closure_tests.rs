use super::self_contained;

/// The stylesheet a relocated SVG is carved from, shaped like the emitter's own output:
/// the captured custom-property layer as a `:root` rule, baked class rules on one line
/// each, and `@keyframes` blocks re-serialised from `cssText` across several lines.
///
/// `fade` is the control. It is the same shape of reference as `pulse` — an animation name
/// followed from a carried rule into a carried definition — with a literal value where
/// `pulse` has a variable. If it is ever absent the closure did not run at all, and an
/// assertion about variables would be reporting on a mechanism that never engaged.
const CSS: &str = concat!(
    ":root{--pulse-fill:#c0392b;--unread:8px;}\n",
    ".r_pulse{animation-name:pulse;fill:rgb(51, 51, 51);}\n",
    ".r_fade{animation-name:fade;fill:rgb(51, 51, 51);}\n",
    "@keyframes pulse { \n  0% { fill: rgb(51, 51, 51); }\n  100% { fill: var(--pulse-fill); }\n}\n",
    "@keyframes fade { \n  0% { fill: rgb(51, 51, 51); }\n  100% { fill: rgb(41, 128, 185); }\n}\n",
);

/// How many times `name` appears giving a value, rather than reading one. A `var()`
/// reference and a declaration are the same token, so counting the token alone cannot tell
/// a carried definition from a copied selector that matches nothing.
fn declaring(styles: &str, name: &str) -> usize {
    styles
        .match_indices(name)
        .filter(|(index, _)| {
            let rest = styles[index + name.len()..].trim_start();
            rest.strip_prefix(':')
                .is_some_and(|value| !value.trim_start().starts_with([';', '}']))
        })
        .count()
}

/// The defect. A definition carried verbatim is text like any other, and the names it
/// spells are unmet in the new document exactly as the fragment's own were. A custom
/// property spells its name in no at-rule prelude, so the loop that follows definitions by
/// name never sees it, and the keyframe's midpoint — authored text the engine interpolates
/// at animation time, never any element's computed style, so never baked — arrives holding
/// a reference to nothing.
///
/// Asserted against the control on purpose: `fade` proves the closure ran, so a missing
/// value for `pulse` is attributable to the variable and not to the mechanism.
#[test]
fn carries_the_value_a_carried_definition_reads() {
    let styles = self_contained(CSS, &["r_pulse".into(), "r_fade".into()]);

    assert!(
        styles.contains("@keyframes fade"),
        "the control definition must be carried, or nothing here is about variables: {styles}"
    );
    assert!(
        styles.contains("@keyframes pulse"),
        "the subject definition must be carried: {styles}"
    );
    assert_eq!(
        declaring(&styles, "--pulse-fill"),
        1,
        "the value the carried definition reads must be declared where the fragment inherits it: {styles}"
    );
}

/// The value has to arrive on a scope the relocated fragment still inherits from. Its own
/// outermost element is the new document's root, so `:root` is that scope; copying the
/// declaring rule as written carries the token under a selector — `html` here — that
/// matches no element in an SVG document, so nothing inherits it and no paint changes.
#[test]
fn declares_the_inherited_value_on_the_new_document_root() {
    let css = CSS.replace(":root{--pulse-fill", "html{--pulse-fill");
    let styles = self_contained(&css, &["r_pulse".into()]);
    let start = styles
        .find("--pulse-fill:")
        .expect("the value is declared somewhere");

    assert!(
        styles[..start].ends_with(":root{"),
        "the declaration must sit on the root of the document the fragment became: {styles}"
    );
}

/// The reference must survive as a reference. Substituting the literal at carry time reads
/// as a fix and destroys the authored indirection: one definition block serves every
/// element naming it, and those elements need not hold the same value.
#[test]
fn leaves_the_reference_itself_alone() {
    let styles = self_contained(CSS, &["r_pulse".into()]);

    assert!(
        styles.contains("var(--pulse-fill)"),
        "the definition must keep reading the name rather than being rewritten: {styles}"
    );
}

/// The loose direction of the closure is carrying too little, not too much. A value
/// nothing reads is dead weight in every fragment, and the whole point of carving a
/// stylesheet is that the asset stays readable.
#[test]
fn leaves_behind_a_value_nothing_reads() {
    let styles = self_contained(CSS, &["r_fade".into()]);

    assert!(
        !styles.contains("--unread"),
        "a value no carried text reads must not be carried: {styles}"
    );
    assert!(
        !styles.contains("--pulse-fill"),
        "nor one read only by a definition this fragment never named: {styles}"
    );
}

/// A value stated only under a condition keeps it. Re-declaring it unconditionally would
/// assert, outside that condition, a value the source never gave.
#[test]
fn keeps_the_condition_a_value_was_stated_under() {
    let css = CSS.replace(
        ":root{--pulse-fill:#c0392b;--unread:8px;}",
        "@media(max-width:320px){:root{--pulse-fill:#c0392b;}}",
    );
    let styles = self_contained(&css, &["r_pulse".into()]);
    let start = styles
        .find("--pulse-fill:")
        .expect("the value is declared somewhere");

    assert!(
        styles[..start].contains("@media(max-width:320px)"),
        "a conditional value must stay conditional: {styles}"
    );
}

/// A re-declared value can itself read a further name, so the second name is wanted too.
/// The carry is one step of a fixed point, not a single pass.
#[test]
fn follows_a_value_that_reads_another() {
    let css = CSS.replace(
        "--pulse-fill:#c0392b;",
        "--pulse-fill:var(--brand);--brand:#c0392b;",
    );
    let styles = self_contained(&css, &["r_pulse".into()]);

    assert_eq!(
        declaring(&styles, "--brand"),
        1,
        "the name the carried value reads must be carried in turn: {styles}"
    );
}

/// Only a scope the fragment is certain to have kept can supply a value. A declaration on
/// some intermediate ancestor may or may not have been overridden before reaching this
/// fragment, and that is not answerable from the stylesheet's text — so it is left alone.
/// Painting a value the element never held is worse than the absent one already produced.
#[test]
fn does_not_guess_a_value_from_a_scope_it_cannot_place() {
    let css = CSS.replace(
        ":root{--pulse-fill:#c0392b;",
        ".theme-dark{--pulse-fill:#c0392b;",
    );
    let styles = self_contained(&css, &["r_pulse".into()]);

    assert_eq!(
        declaring(&styles, "--pulse-fill"),
        0,
        "a value from an unplaceable scope must not be invented: {styles}"
    );
}

/// The fragment's own baked scope already holds the value, so nothing is missing and the
/// root must not restate it. A second declaration on a further ancestor is a duplicate at
/// best, and at worst overrides nothing while suggesting it does.
#[test]
fn leaves_a_value_the_fragment_already_carries() {
    let css = CSS.replace(".r_pulse{", ".r_pulse{--pulse-fill:#c0392b;");
    let styles = self_contained(&css, &["r_pulse".into()]);

    assert_eq!(
        declaring(&styles, "--pulse-fill"),
        1,
        "a value the fragment already declares must not be restated: {styles}"
    );
    assert!(
        !styles.contains(":root"),
        "no ancestor scope is needed when the fragment holds the value itself: {styles}"
    );
}
