use super::{CAMEL_CASED, HYPHENATED, RENAMED, document_attribute, jsx_attribute};

/// The property the two directions exist to satisfy, stated over arbitrary names rather
/// than over the tables' own contents.
///
/// One direction used to be a rule with unbounded domain and the other a search bounded by
/// `HYPHENATED`, so the round trip held inside the list and failed silently outside it: a
/// name absent from the list was camel-cased on the way out and never restored on the way
/// back, and both halves reported success. The list cannot be the fix — a presentation
/// attribute is defined by reference to the CSS property set, so the family grows with the
/// styling module and any list transcribed today is short tomorrow. Sharing one domain is
/// what makes an omission cost nothing.
#[test]
fn every_name_survives_the_trip_out_to_jsx_and_back_to_the_document() {
    let mut names: Vec<String> = HYPHENATED
        .iter()
        .chain(RENAMED.iter().map(|(document, _)| document))
        .map(|name| (*name).to_owned())
        .collect();
    names.extend(CAMEL_CASED.iter().map(|name| name.to_ascii_lowercase()));
    names.extend(
        [
            // Multi-word SVG presentation attributes the list does not name. Both are real
            // and still parsed; they are here as absences, so replacing them with whatever
            // the list gains next would defeat the test.
            "color-rendering",
            "enable-background",
            // Hyphenated names no specification will ever add, which is what proves the rule
            // holds beyond any list rather than beyond the current one.
            "author-invented-attribute",
            "x-two-word",
            // SVG's own camelCase spellings, which no inverse may touch.
            "viewBox",
            "preserveAspectRatio",
            "gradientTransform",
            "stdDeviation",
        ]
        .map(str::to_owned),
    );
    for name in names {
        assert_eq!(
            document_attribute(&jsx_attribute(&name)),
            name,
            "{name} did not survive the round trip"
        );
    }
}

/// The defect that prompted the table: a `<time datetime="…">` was emitted as
/// `datetime={…}`, which React does not recognise. `datetime` contains no hyphen, so the
/// hyphen-driven conversion returned it unchanged and only an exception entry can fix it.
#[test]
fn single_word_attributes_that_camel_case_in_jsx_are_translated() {
    assert_eq!(jsx_attribute("datetime"), "dateTime");
    assert_eq!(jsx_attribute("colspan"), "colSpan");
    assert_eq!(jsx_attribute("rowspan"), "rowSpan");
    assert_eq!(jsx_attribute("maxlength"), "maxLength");
    assert_eq!(jsx_attribute("crossorigin"), "crossOrigin");
    assert_eq!(jsx_attribute("srcset"), "srcSet");
    assert_eq!(jsx_attribute("contenteditable"), "contentEditable");
    assert_eq!(jsx_attribute("autocomplete"), "autoComplete");
    assert_eq!(jsx_attribute("novalidate"), "noValidate");
    assert_eq!(jsx_attribute("charset"), "charSet");
}

/// The cases that already worked must keep working, so a fix cannot be a swap of one gap
/// for another.
#[test]
fn hyphenated_and_renamed_attributes_keep_their_existing_translations() {
    assert_eq!(jsx_attribute("stroke-width"), "strokeWidth");
    assert_eq!(jsx_attribute("clip-path"), "clipPath");
    assert_eq!(jsx_attribute("for"), "htmlFor");
    assert_eq!(jsx_attribute("class"), "className");
    assert_eq!(jsx_attribute("tabindex"), "tabIndex");
    assert_eq!(jsx_attribute("readonly"), "readOnly");
}

/// React accepts these exactly as authored, and camel-casing them would emit a prop no
/// renderer knows. This is the inverse of the fix: a table applied too eagerly breaks them.
#[test]
fn accessibility_and_data_attributes_are_passed_through_verbatim() {
    assert_eq!(jsx_attribute("aria-label"), "aria-label");
    assert_eq!(jsx_attribute("aria-live"), "aria-live");
    assert_eq!(jsx_attribute("data-count"), "data-count");
    assert_eq!(jsx_attribute("data-max-length"), "data-max-length");
}

/// Attributes whose HTML and JSX spellings already agree must be returned untouched rather
/// than mangled by a rule that assumes every name needs work.
#[test]
fn attributes_that_are_already_correct_are_left_alone() {
    for name in [
        "id",
        "href",
        "src",
        "title",
        "value",
        "role",
        "type",
        "placeholder",
    ] {
        assert_eq!(jsx_attribute(name), name);
    }
}

/// The two hyphenated HTML attributes, which sat in the case-only table until this was
/// written. The backward direction restores an entry of that table by lowercasing it, so it
/// emitted `acceptcharset` and `httpequiv` — names no parser knows — while the forward
/// direction happened to be right only because the case-insensitive lookup could never match
/// across a hyphen and the total rule downstream rescued it. Removing that rule is what
/// exposed them, and the family they actually belong to is the hyphenated one.
#[test]
fn the_two_hyphenated_html_attributes_translate_in_both_directions() {
    assert_eq!(jsx_attribute("accept-charset"), "acceptCharset");
    assert_eq!(jsx_attribute("http-equiv"), "httpEquiv");
    assert_eq!(document_attribute("acceptCharset"), "accept-charset");
    assert_eq!(document_attribute("httpEquiv"), "http-equiv");
}

/// The table is a list of canonical names whose HTML spelling is its own lowercase form.
/// If an entry were ever written so that lowercasing it did not give the HTML attribute —
/// a stray hyphen, or a name that is not purely a case change — the lookup would silently
/// never match it, and the entry would be dead weight that looks like coverage.
#[test]
fn every_table_entry_is_reachable_from_its_html_spelling() {
    for canonical in CAMEL_CASED {
        let html = canonical.to_ascii_lowercase();
        assert_ne!(
            *canonical, html,
            "{canonical} is not a case change and does not belong in this table"
        );
        assert_eq!(
            jsx_attribute(&html),
            *canonical,
            "{canonical} is unreachable from its own lowercase spelling"
        );
    }
}

/// A duplicated entry would make the table's first match arbitrary, so the list must be a
/// set. Checked on the canonical spelling and on the lookup key alike.
#[test]
fn the_table_holds_no_duplicate_entries() {
    let mut keys: Vec<String> = CAMEL_CASED
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect();
    let total = keys.len();
    keys.sort();
    keys.dedup();
    assert_eq!(
        keys.len(),
        total,
        "the attribute table contains a duplicate"
    );
}
