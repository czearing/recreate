//! The key a recorded shorthand division is filed under.
//!
//! The division is looked up by the declarations block as the artifact spells it, and the
//! artifact spells every reference resolved against the sheet that held it. The two views are
//! produced by one expression for that reason: projected apart, a division would be filed
//! under text nothing later carries, and the base-arm stage would silently fall back to
//! "cannot say" for every shorthand holding a reference.

use super::super::reach_harness::{ORIGIN, rule, rules, walk_with_shorthands};
use serde_json::json;

fn leaf() -> serde_json::Value {
    json!({ "tag": "p" })
}
/// A block's recorded division is keyed by the block text, and the artifact spells that text
/// with every reference resolved against the sheet that held it. Projecting the two views
/// apart is what would file a division under a key nothing later looks it up by, silently
/// restoring the defect for every shorthand that carries a reference.
#[test]
fn keys_a_recorded_division_by_the_text_the_artifact_carries() {
    let sheet = "http://rig.test:59700/styles/theme.css";
    let block = "background: url(\"tile.png\") rgb(255, 0, 0);";
    let result = walk_with_shorthands(
        &leaf(),
        &json!([rule(format!(".card {{ {block} }}"), sheet)]),
        &json!([{
            "text": block,
            "base": sheet,
            "shares": {
                "background-color": "rgb(255, 0, 0)",
                "background-image": "url(\"tile.png\")"
            }
        }]),
        ORIGIN,
    );

    let carried = rules(&result);
    let divisions = result["shorthands"].as_object().expect("shorthands");
    let key = carried[0]
        .split_once('{')
        .expect("a rule has a block")
        .1
        .trim()
        .trim_end_matches('}')
        .trim();

    assert!(
        divisions.contains_key(key),
        "the division was filed under a key the artifact never carries: {divisions:?} vs {key:?}"
    );
    assert_eq!(
        divisions[key]["background-image"], "url(\"/styles/tile.png\")",
        "a share's own reference was left pointing at the capture rig"
    );
}
