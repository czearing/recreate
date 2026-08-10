use crate::model::PageState;

/// One element the capture recorded as scrolled, with the offsets it was holding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scrolled<'a> {
    pub path: &'a str,
    pub left: i64,
    pub top: i64,
}

/// Whether `path` is the element that scrolls the document rather than a box of its own.
///
/// `document.scrollingElement` is the root element in standards mode and `body` in quirks
/// mode, and the capture walks both, so whichever one the engine used carries the offset and
/// the other stays at zero. This is the only place a tag may be named: the runtime's
/// `setScroll` routes the window through the global `scrollTo` and everything else through
/// `element.scrollTo`, so the two must be told apart before they are serialized.
pub fn scrolls_document(state: &PageState, path: &str) -> bool {
    path == "html"
        || state
            .nodes
            .iter()
            .any(|node| node.path == path && node.tag == "body")
}

/// Elements whose recorded scroll offset in `state` differs from the offset in `baseline`.
///
/// Ownership is read, never inferred. The capture writes `scroll_left`/`scroll_top` for every
/// element in both states, so the element holding a changed offset *is* the element that
/// scrolled — there is no ancestor to search for and no `overflow` value to interpret. That
/// matters because scrollability cannot be decided from one axis of computed `overflow` at
/// all: per CSS Overflow 3 the scrollable values are `auto`, `scroll` *and* `hidden`, `clip`
/// alone forbids scrolling through any mechanism, and a specified `visible` computes to `auto`
/// whenever the other axis is scrollable. An element that cannot scroll cannot report a
/// changed offset, so physics decides the question that an allow-list kept getting wrong.
pub fn changed<'a>(baseline: &PageState, state: &'a PageState) -> Vec<Scrolled<'a>> {
    state
        .dom
        .iter()
        .filter_map(|(path, current)| {
            let previous = baseline.dom.get(path);
            let left = offset(
                current.scroll_left,
                previous.map_or(0.0, |dom| dom.scroll_left),
            );
            let top = offset(
                current.scroll_top,
                previous.map_or(0.0, |dom| dom.scroll_top),
            );
            (left != 0 || top != 0).then_some(Scrolled {
                path: path.as_str(),
                left,
                top,
            })
        })
        .collect()
}

/// Elements the capture recorded as already scrolled when the page came to rest.
pub fn resting(state: &PageState) -> Vec<Scrolled<'_>> {
    state
        .dom
        .iter()
        .filter_map(|(path, dom)| {
            let (left, top) = (offset(dom.scroll_left, 0.0), offset(dom.scroll_top, 0.0));
            (left != 0 || top != 0).then_some(Scrolled {
                path: path.as_str(),
                left,
                top,
            })
        })
        .collect()
}

fn offset(current: f64, previous: f64) -> i64 {
    let shift = current - previous;
    if shift.abs() < 1.0 {
        0
    } else {
        shift.round() as i64
    }
}
