use std::collections::BTreeMap;

/// The one place that localises asset URLs embedded in CSS text.
///
/// Every caller passes a map of original absolute URL to the local path the recreation
/// serves, and the URLs are matched as substrings because a declaration carries them
/// inside `url()` rather than as its whole value. Substring matching is order-sensitive:
/// `BTreeMap` iterates byte-lexicographically and a string that is a prefix of another
/// always sorts before it, so iterating the map directly reliably rewrites the shorter URL
/// first, consumes it out of the longer one, and strands the longer URL's tail on the
/// other asset's local path. Replacing longest-first is the maximal-munch rule and is what
/// makes the result independent of how the URLs happen to be spelled.
///
/// An attribute whose entire value is a URL is not this operation — `jsx_attrs` looks the
/// value up whole, which has no substring hazard, and must stay that way.
pub(super) fn rewrite(text: &str, assets: &BTreeMap<String, String>) -> String {
    // The CSSOM serialises every image reference as `url()`, including inside `image-set()`,
    // so text without it holds no asset URL. Most pages carry no assets at all.
    if assets.is_empty() || !text.contains("url(") {
        return text.to_string();
    }
    let mut replacements: Vec<_> = assets.iter().collect();
    replacements.sort_by_key(|(url, _)| std::cmp::Reverse(url.len()));
    replacements
        .into_iter()
        .fold(text.to_string(), |text, (url, local)| {
            let text = text.replace(url, local);
            // A stylesheet may write the same asset without its scheme, or — for its own
            // origin — as a root-relative path. Left unrewritten those spellings point at
            // files the recreation never serves, so the browser silently falls back: a
            // missing webfont in particular changes text metrics and rewraps every line of
            // the page. Only the start of a url() value is matched, so a path already
            // rewritten by an earlier entry is never hit again.
            scheme_relative(url)
                .into_iter()
                .chain(origin_relative_path(url))
                .fold(text, |text, spelling| {
                    replace_url_value(&text, spelling, local)
                })
        })
}

/// `//host/path`, which resolves under whichever scheme the page was served over.
fn scheme_relative(url: &str) -> Option<&str> {
    url.strip_prefix("https:").or_else(|| url.strip_prefix("http:"))
}

fn replace_url_value(text: &str, path: &str, local: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(path) {
        let opens_value = matches!(rest[..at].chars().next_back(), Some('(' | '"' | '\''));
        out.push_str(&rest[..at]);
        out.push_str(if opens_value { local } else { path });
        rest = &rest[at + path.len()..];
    }
    out.push_str(rest);
    out
}

fn origin_relative_path(url: &str) -> Option<&str> {
    let rest = scheme_relative(url)?.strip_prefix("//")?;
    let start = rest.find('/')?;
    Some(&rest[start..])
}

#[cfg(test)]
#[path = "asset_urls_tests.rs"]
mod tests;
