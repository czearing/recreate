use crate::model::{DomNode, PageState};

/// One element the capture recorded as scrolled, with the offsets it was holding.
///
/// The fields are private and `at` is the only constructor, so within this crate a `Scrolled`
/// can only come from a captured record. That is what makes the wrong kind unspeakable rather
/// than merely discouraged: the previous version documented the same contract in prose and a
/// second producer filled the same fields with a distance anyway.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scrolled<'a> {
    path: &'a str,
    left: i64,
    top: i64,
}

impl<'a> Scrolled<'a> {
    /// The only way to obtain a `Scrolled`, and it takes the capture's record of one element
    /// rather than two numbers. A distance is not a record, so it cannot be stored here — the
    /// kind is settled once, where the value is read, and every later stage receives a
    /// position. That matters because the runtime replays these through `scrollTo`, which sets
    /// an absolute position; the relative operation is `scrollBy` and is never used.
    fn at(path: &'a str, dom: &DomNode) -> Self {
        Self {
            path,
            left: whole(dom.scroll_left),
            top: whole(dom.scroll_top),
        }
    }

    pub fn path(&self) -> &'a str {
        self.path
    }

    pub fn left(&self) -> i64 {
        self.left
    }

    pub fn top(&self) -> i64 {
        self.top
    }

    fn is_displaced(&self) -> bool {
        self.left != 0 || self.top != 0
    }
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

/// Elements whose recorded offset in `state` differs from the offset in `baseline`, each
/// carrying the offset it was holding in `state`.
///
/// Ownership is read, never inferred. The capture writes `scroll_left`/`scroll_top` for every
/// element in both states, so the element holding a changed offset *is* the element that
/// scrolled — there is no ancestor to search for and no `overflow` value to interpret. That
/// matters because scrollability cannot be decided from one axis of computed `overflow` at
/// all: per CSS Overflow 3 the scrollable values are `auto`, `scroll` *and* `hidden`, `clip`
/// alone forbids scrolling through any mechanism, and a specified `visible` computes to `auto`
/// whenever the other axis is scrollable. An element that cannot scroll cannot report a
/// changed offset, so physics decides the question that an allow-list kept getting wrong.
///
/// The two states answer different questions and both are needed. `baseline` decides *whether*
/// an element belongs here, because an element belongs precisely when it moved. `state` decides
/// *what* it carries, because the consumer replays a position. Reading the value from the
/// difference is the same number only while the element rested at zero.
pub fn moved<'a>(baseline: &PageState, state: &'a PageState) -> Vec<Scrolled<'a>> {
    state
        .dom
        .iter()
        .filter(|(path, current)| shift(baseline, path, current) != (0, 0))
        .map(|(path, current)| Scrolled::at(path.as_str(), current))
        .collect()
}

/// Elements the capture recorded as already scrolled when the page came to rest.
pub fn resting(state: &PageState) -> Vec<Scrolled<'_>> {
    state
        .dom
        .iter()
        .map(|(path, dom)| Scrolled::at(path.as_str(), dom))
        .filter(Scrolled::is_displaced)
        .collect()
}

/// Whether anything other than the document moved along the horizontal axis.
///
/// This is the one question about scroll that is genuinely differential — it asks what changed,
/// not where anything is — so it is answered here as a boolean. An element already resting at a
/// horizontal offset has not moved horizontally, and reading its position would say it had.
pub fn shifted_horizontally(baseline: &PageState, state: &PageState) -> bool {
    state.dom.iter().any(|(path, current)| {
        shift(baseline, path, current).0 != 0 && !scrolls_document(state, path)
    })
}

/// How far an element moved between the two states, computed where it is asked for and never
/// stored, so it cannot reach a consumer that expects a position.
fn shift(baseline: &PageState, path: &str, current: &DomNode) -> (i64, i64) {
    let previous = baseline.dom.get(path);
    (
        whole(current.scroll_left - previous.map_or(0.0, |dom| dom.scroll_left)),
        whole(current.scroll_top - previous.map_or(0.0, |dom| dom.scroll_top)),
    )
}

/// Sub-pixel offsets are device-pixel-ratio noise rather than scroll, on either question.
fn whole(offset: f64) -> i64 {
    if offset.abs() < 1.0 {
        0
    } else {
        offset.round() as i64
    }
}
