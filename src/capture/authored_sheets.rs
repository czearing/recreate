use crate::cdp::Cdp;
use serde::Serialize;
use serde_json::json;

/// A stylesheet the page's own script cannot read.
///
/// `CSSStyleSheet.cssRules` throws `SecurityError` for any cross-origin sheet served
/// without `Access-Control-Allow-Origin`, which is how most production sites serve CSS.
/// The capture walk swallowed that error, so those pages produced no authored rules at
/// all and every box fell back to a sampled pixel. `CSS.getStyleSheetText` reads the
/// text out of the browser's already-parsed CSSOM and issues no request, so CORS never
/// applies to it.
///
/// The text travels with the sheet's own address because text alone cannot say which
/// sheet it came from, and a sheet carries state its text does not: the `media` attribute
/// of the `<link>` that fetched it conditions every rule inside without appearing in it.
#[derive(Debug, Serialize)]
pub struct AuthoredSheet {
    pub text: String,
    pub href: String,
}

#[derive(Debug, Default)]
pub struct AuthoredSheets {
    pub sheets: Vec<AuthoredSheet>,
    pub unreadable: usize,
}

pub async fn collect(cdp: &mut Cdp) -> AuthoredSheets {
    if cdp.enable(&["DOM", "CSS"]).await.is_err() {
        return AuthoredSheets::default();
    }
    let added = cdp.take_events_named("CSS.styleSheetAdded");
    let mut sheets = AuthoredSheets::default();
    for event in &added {
        let header = &event["params"]["header"];
        let Some(id) = header["styleSheetId"].as_str() else {
            continue;
        };
        let href = header["sourceURL"].as_str().unwrap_or_default().to_string();
        match cdp
            .send("CSS.getStyleSheetText", json!({ "styleSheetId": id }))
            .await
        {
            Ok(value) => match value["text"].as_str() {
                Some(text) if !text.trim().is_empty() => sheets.sheets.push(AuthoredSheet {
                    text: text.to_string(),
                    href,
                }),
                _ => sheets.unreadable += 1,
            },
            Err(_) => sheets.unreadable += 1,
        }
    }
    cdp.put_events_named("CSS.styleSheetAdded", added);
    sheets
}

#[cfg(test)]
mod tests {
    use super::{AuthoredSheet, AuthoredSheets};

    /// An empty capture must be visible as an empty capture, never as a success with
    /// nothing in it, because that is indistinguishable from a page with no CSS.
    #[test]
    fn an_unreadable_sheet_is_counted_rather_than_dropped() {
        let sheets = AuthoredSheets {
            sheets: vec![AuthoredSheet {
                text: ".a{color:red}".into(),
                href: "https://cdn/a.css".into(),
            }],
            unreadable: 2,
        };
        assert_eq!(sheets.sheets.len(), 1);
        assert_eq!(sheets.unreadable, 2);
    }
}
