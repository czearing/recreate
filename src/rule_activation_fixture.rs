//! The scripted CSSOM the activation tests share.

use super::style;
use serde_json::{Value, json};

/// The scene: a 300px container, a false `@supports` condition, a `@container` condition
/// that a 300px container cannot satisfy, and — so that dropping every grouped rule cannot
/// pass — conditions that do hold. The sheet is supplied twice, which is what the capture
/// does when it cannot tell which sheets the page failed to read.
pub(super) fn scene() -> Value {
    let sheet = json!([
        style(".panel", "padding", "24px"),
        {
            "prelude": "@container panelwrap (min-width: 900px)",
            "conditionText": "panelwrap (min-width: 900px)",
            "rules": [style(".panel", "width", "100%")]
        },
        {
            "prelude": "@supports (color: nonexistent-color-function(1))",
            "conditionText": "(color: nonexistent-color-function(1))",
            "rules": [style(".panel", "max-width", "50%")]
        },
        {
            "prelude": "@supports (display: grid)",
            "conditionText": "(display: grid)",
            "rules": [style(".grid", "display", "grid")]
        },
        {
            "prelude": "@media (min-width: 900px)",
            "conditionText": "(min-width: 900px)",
            "rules": [style(".panel", "color", "red")]
        },
        {
            "prelude": "@media (min-width: 0px)",
            "conditionText": "(min-width: 0px)",
            "rules": [{
                "prelude": "@supports (display: grid)",
                "conditionText": "(display: grid)",
                "rules": [style(".wide", "gap", "8px")]
            }]
        },
        {
            "prelude": "@keyframes spin",
            "keyframes": true,
            "rules": [style("from", "rotate", "0deg"), style("to", "rotate", "360deg")]
        },
        { "prelude": "@property --angle", "declarations": { "syntax": "'<angle>'" } }
    ]);
    json!({
        "elements": [
            { "path": "/main/div", "classes": ["wrap"] },
            { "path": "/main/div/div", "classes": ["panel"] },
            { "path": "/main/div/span", "classes": ["grid"] },
            { "path": "/main/div/p", "classes": ["wide"] }
        ],
        "matching": {
            "@supports (display: grid)": ["/main/div", "/main/div/div", "/main/div/span", "/main/div/p"],
            "@media (min-width: 0px)": ["/main/div", "/main/div/div", "/main/div/span", "/main/div/p"]
        },
        "sheets": [sheet.clone(), sheet]
    })
}
